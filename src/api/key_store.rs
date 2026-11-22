// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Persistent API keys: create, list, revoke, expire, scope.
//!
//! This is additive to the environment keys (`VEYRON_API_KEY`, `VEYRON_API_KEYS`), which are
//! validated exactly as before and always win. A stored key looks like `vk_<id>_<secret>`:
//!
//! * the plaintext token is returned once, by [`KeyStore::create`], and never stored or logged;
//! * only `SHA-256(domain | pepper | salt | token)` and the per-key random salt are persisted.
//!   The tokens are 256-bit random values, so a fast salted hash is sufficient (there is nothing
//!   to brute-force); the pepper lives outside the Secret (see [`Pepper`]);
//! * verification looks the key up by id, recomputes the hash and compares in constant time.
//!
//! Persistence goes through the [`KeyBackend`] trait. The cluster implementation is a single
//! Secret (`src/kube/api_key_store.rs`); tests use an in-memory fake.

use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::future::Future;

/// Every stored key's token starts with this, so other bearer credentials (JWTs, env keys)
/// never trigger a store read.
pub const TOKEN_PREFIX: &str = "vk_";
/// Upper bound on records kept in the Secret (it has a 1 MiB limit).
pub const MAX_KEYS: usize = 200;
pub const MAX_EXPIRY_DAYS: u32 = 3650;
/// Revoked or expired records are dropped from the store this long after they stopped working.
const RETENTION_DAYS: i64 = 90;
const DOMAIN: &[u8] = b"veyron-api-key-v1";

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("{0}")]
    Invalid(String),
    #[error("an active key named '{0}' already exists")]
    Duplicate(String),
    #[error("key store is full ({MAX_KEYS} records); revoke and wait for old records to age out")]
    Full,
    #[error("no active key '{0}'")]
    NotFound(String),
    #[error("key store changed concurrently; retry")]
    Conflict,
    #[error(
        "no pepper available: set VEYRON_API_KEY_PEPPER (16+ chars) or VEYRON_API_KEY, the same value on the server"
    )]
    NoPepper,
    #[error("key store backend: {0}")]
    Backend(String),
}

/// Why a presented token was not accepted. Callers answer all of them with the same 401.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyError {
    Malformed,
    Unknown,
    Revoked,
    Expired,
}

/// Secret mixed into every stored hash, kept out of the Secret that holds the hashes so that
/// a copy of the store alone is not enough to test guesses.
#[derive(Clone)]
pub struct Pepper(Vec<u8>);

impl Pepper {
    /// `explicit` is `VEYRON_API_KEY_PEPPER`; `primary` is `VEYRON_API_KEY`, used (domain-separated)
    /// when no explicit pepper is set so the feature works without extra configuration.
    /// Rotating either value invalidates stored keys. Returns `None` when neither is usable.
    pub fn from_values(explicit: Option<&str>, primary: Option<&str>) -> Option<Self> {
        let explicit = explicit.map(str::trim).filter(|v| !v.is_empty());
        if let Some(p) = explicit {
            return (p.len() >= 16).then(|| Self(p.as_bytes().to_vec()));
        }
        let primary = primary.map(str::trim).filter(|v| !v.is_empty())?;
        let mut h = Sha256::new();
        h.update(b"veyron-api-key-pepper-from-primary-v1");
        h.update(primary.as_bytes());
        Some(Self(h.finalize().to_vec()))
    }

    pub fn from_env() -> Option<Self> {
        Self::from_values(
            std::env::var("VEYRON_API_KEY_PEPPER").ok().as_deref(),
            std::env::var("VEYRON_API_KEY").ok().as_deref(),
        )
    }

    /// Short non-reversible tag, stored per key so a CLI run with a different pepper than the
    /// server can be told apart from a bad key.
    pub fn id(&self) -> String {
        let mut h = Sha256::new();
        h.update(b"veyron-pepper-id");
        h.update(&self.0);
        hex::encode(h.finalize())[..8].to_string()
    }
}

impl std::fmt::Debug for Pepper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pepper(..)")
    }
}

/// One persisted record. Never contains the token.
#[derive(Clone, Serialize, Deserialize)]
pub struct StoredKey {
    pub id: String,
    pub name: String,
    /// `admin` | `write` | `readonly`
    pub role: String,
    /// Namespaces the key may act in, or `*`/empty for no restriction. See [`scopes_allow_path`].
    #[serde(default)]
    pub scopes: Vec<String>,
    pub salt: String,
    pub hash: String,
    #[serde(default)]
    pub pepper_id: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by: Option<String>,
}

impl std::fmt::Debug for StoredKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredKey")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("role", &self.role)
            .finish_non_exhaustive()
    }
}

impl StoredKey {
    pub fn status(&self, now: DateTime<Utc>) -> &'static str {
        if self.revoked_at.is_some() {
            "revoked"
        } else if self.expires_at.is_some_and(|e| e <= now) {
            "expired"
        } else {
            "active"
        }
    }
}

/// What `list` and the REST API return: everything except salt and hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeySummary {
    pub id: String,
    pub name: String,
    pub role: String,
    pub scopes: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub pepper_matches: bool,
}

/// A successfully verified credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub id: String,
    pub name: String,
    pub role: String,
    pub scopes: Vec<String>,
}

impl Verified {
    pub fn allows_path(&self, path: &str) -> bool {
        scopes_allow_path(&self.scopes, path)
    }
}

#[derive(Debug, Clone)]
pub struct CreateSpec {
    pub name: String,
    pub role: String,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<u32>,
}

/// Records as read, plus an opaque version the backend uses to detect concurrent writers.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub keys: Vec<StoredKey>,
    pub version: Option<String>,
}

pub trait KeyBackend {
    fn load(&self) -> impl Future<Output = Result<Snapshot, KeyError>> + Send;
    /// Must fail with [`KeyError::Conflict`] when `version` is no longer current.
    fn save(
        &self,
        keys: &[StoredKey],
        version: Option<&str>,
    ) -> impl Future<Output = Result<(), KeyError>> + Send;
}

pub fn valid_role(role: &str) -> bool {
    matches!(role, "admin" | "write" | "readonly")
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
}

/// A namespace name or `*`.
pub fn valid_scope(scope: &str) -> bool {
    scope == "*"
        || (!scope.is_empty()
            && scope.len() <= 63
            && scope
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
            && !scope.starts_with('-')
            && !scope.ends_with('-'))
}

pub fn is_store_token(token: &str) -> bool {
    token.starts_with(TOKEN_PREFIX)
}

/// The namespace a route names in its path (`/api/v1/vms/{ns}/{name}/...`, with or without the
/// `/veyron` alias). Only path-embedded namespaces are trusted for scope decisions: a query
/// string or body namespace may be ignored or overridden by the handler.
pub fn path_namespace(path: &str) -> Option<&str> {
    let rest = path
        .strip_prefix("/api/v1/veyron/")
        .or_else(|| path.strip_prefix("/api/v1/"))?;
    let mut parts = rest.split('/');
    if parts.next()? != "vms" {
        return None;
    }
    let ns = parts.next().filter(|n| valid_scope(n) && *n != "*")?;
    // `/vms/{ns}` alone is not a route; require a name segment.
    parts.next().filter(|n| !n.is_empty())?;
    Some(ns)
}

/// Empty scopes or `*` mean unrestricted. Otherwise the request must name one of the scoped
/// namespaces in its path; every other route (cluster-wide, query/body namespaces) is denied.
pub fn scopes_allow_path(scopes: &[String], path: &str) -> bool {
    if scopes.is_empty() || scopes.iter().any(|s| s == "*") {
        return true;
    }
    path_namespace(path).is_some_and(|ns| scopes.iter().any(|s| s == ns))
}

/// Constant-time equality for equal-length and unequal-length inputs alike (length is public).
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc = 0u8;
    for (x, y) in a.iter().zip(b) {
        acc |= x ^ y;
    }
    acc == 0
}

fn hash_token(pepper: &Pepper, salt: &[u8], token: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(DOMAIN);
    h.update((pepper.0.len() as u32).to_be_bytes());
    h.update(&pepper.0);
    h.update((salt.len() as u32).to_be_bytes());
    h.update(salt);
    h.update(token.as_bytes());
    h.finalize().into()
}

fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    OsRng.fill_bytes(&mut buf);
    hex::encode(buf)
}

fn summarize(k: &StoredKey, pepper: Option<&Pepper>, now: DateTime<Utc>) -> KeySummary {
    KeySummary {
        id: k.id.clone(),
        name: k.name.clone(),
        role: k.role.clone(),
        scopes: k.scopes.clone(),
        status: k.status(now).to_string(),
        created_at: k.created_at,
        created_by: k.created_by.clone(),
        expires_at: k.expires_at,
        revoked_at: k.revoked_at,
        pepper_matches: pepper.is_none_or(|p| k.pepper_id.is_empty() || k.pepper_id == p.id()),
    }
}

/// Pure verification against a set of records (used by the request path with a cached read).
pub fn verify_token(
    keys: &[StoredKey],
    pepper: &Pepper,
    token: &str,
    now: DateTime<Utc>,
) -> Result<Verified, VerifyError> {
    let rest = token
        .strip_prefix(TOKEN_PREFIX)
        .ok_or(VerifyError::Malformed)?;
    let (id, secret) = rest.split_once('_').ok_or(VerifyError::Malformed)?;
    if id.len() != 16 || secret.len() != 64 {
        return Err(VerifyError::Malformed);
    }
    let found = keys.iter().find(|k| k.id == id);
    // Do the same hashing work whether or not the id exists.
    let dummy_salt = [0u8; 16];
    let salt = found
        .and_then(|k| hex::decode(&k.salt).ok())
        .unwrap_or_else(|| dummy_salt.to_vec());
    let computed = hash_token(pepper, &salt, token);
    let Some(k) = found else {
        return Err(VerifyError::Unknown);
    };
    let stored = hex::decode(&k.hash).unwrap_or_default();
    if !ct_eq(&computed, &stored) {
        return Err(VerifyError::Unknown);
    }
    if k.revoked_at.is_some() {
        return Err(VerifyError::Revoked);
    }
    if k.expires_at.is_some_and(|e| e <= now) {
        return Err(VerifyError::Expired);
    }
    if !valid_role(&k.role) {
        return Err(VerifyError::Unknown);
    }
    Ok(Verified {
        id: k.id.clone(),
        name: k.name.clone(),
        role: k.role.clone(),
        scopes: k.scopes.clone(),
    })
}

pub struct KeyStore<B: KeyBackend> {
    backend: B,
    pepper: Option<Pepper>,
}

impl<B: KeyBackend> KeyStore<B> {
    pub fn new(backend: B, pepper: Option<Pepper>) -> Self {
        Self { backend, pepper }
    }

    /// Create a key. Returns its summary and the plaintext token, which exists nowhere else.
    pub async fn create(
        &self,
        spec: CreateSpec,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<(KeySummary, String), KeyError> {
        let pepper = self.pepper.as_ref().ok_or(KeyError::NoPepper)?;
        let name = spec.name.trim().to_string();
        if !valid_name(&name) {
            return Err(KeyError::Invalid(
                "name must be 1-64 characters of A-Z a-z 0-9 . _ -".into(),
            ));
        }
        if !valid_role(&spec.role) {
            return Err(KeyError::Invalid(
                "role must be admin, write or readonly".into(),
            ));
        }
        let mut scopes: Vec<String> = spec.scopes.iter().map(|s| s.trim().to_string()).collect();
        if let Some(bad) = scopes.iter().find(|s| !valid_scope(s)) {
            return Err(KeyError::Invalid(format!(
                "invalid scope '{bad}' (a namespace name or *)"
            )));
        }
        scopes.sort();
        scopes.dedup();
        if let Some(days) = spec.expires_in_days
            && (days == 0 || days > MAX_EXPIRY_DAYS)
        {
            return Err(KeyError::Invalid(format!(
                "expiry must be 1-{MAX_EXPIRY_DAYS} days"
            )));
        }

        let snap = self.backend.load().await?;
        let mut keys: Vec<StoredKey> = snap
            .keys
            .into_iter()
            .filter(|k| !aged_out(k, now))
            .collect();
        if keys
            .iter()
            .any(|k| k.name == name && k.status(now) == "active")
        {
            return Err(KeyError::Duplicate(name));
        }
        if keys.len() >= MAX_KEYS {
            return Err(KeyError::Full);
        }

        let id = random_hex(8);
        let token = format!("{TOKEN_PREFIX}{id}_{}", random_hex(32));
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let record = StoredKey {
            id,
            name,
            role: spec.role,
            scopes,
            salt: hex::encode(salt),
            hash: hex::encode(hash_token(pepper, &salt, &token)),
            pepper_id: pepper.id(),
            created_at: now,
            created_by: actor.to_string(),
            expires_at: spec
                .expires_in_days
                .map(|d| now + Duration::days(i64::from(d))),
            revoked_at: None,
            revoked_by: None,
        };
        let summary = summarize(&record, Some(pepper), now);
        keys.push(record);
        self.backend.save(&keys, snap.version.as_deref()).await?;
        Ok((summary, token))
    }

    pub async fn list(
        &self,
        include_inactive: bool,
        now: DateTime<Utc>,
    ) -> Result<Vec<KeySummary>, KeyError> {
        let snap = self.backend.load().await?;
        Ok(snap
            .keys
            .iter()
            .filter(|k| include_inactive || k.status(now) == "active")
            .map(|k| summarize(k, self.pepper.as_ref(), now))
            .collect())
    }

    /// Revoke an active key by id or name. The record stays (marked) for audit until it ages out.
    pub async fn revoke(
        &self,
        id_or_name: &str,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<KeySummary, KeyError> {
        let snap = self.backend.load().await?;
        let mut keys = snap.keys;
        let target = id_or_name.trim();
        let idx = keys
            .iter()
            .position(|k| k.revoked_at.is_none() && (k.id == target || k.name == target))
            .ok_or_else(|| KeyError::NotFound(target.to_string()))?;
        keys[idx].revoked_at = Some(now);
        keys[idx].revoked_by = Some(actor.to_string());
        let summary = summarize(&keys[idx], self.pepper.as_ref(), now);
        keys.retain(|k| !aged_out(k, now));
        self.backend.save(&keys, snap.version.as_deref()).await?;
        Ok(summary)
    }

    pub async fn verify(&self, token: &str, now: DateTime<Utc>) -> Result<Verified, VerifyError> {
        let pepper = self.pepper.as_ref().ok_or(VerifyError::Unknown)?;
        let snap = self
            .backend
            .load()
            .await
            .map_err(|_| VerifyError::Unknown)?;
        verify_token(&snap.keys, pepper, token, now)
    }
}

fn aged_out(k: &StoredKey, now: DateTime<Utc>) -> bool {
    let ended = k.revoked_at.or(k.expires_at);
    ended.is_some_and(|t| t + Duration::days(RETENTION_DAYS) < now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// In-memory store with optimistic-concurrency versioning like the Secret-backed one.
    #[derive(Default)]
    struct FakeBackend {
        state: Mutex<(Vec<StoredKey>, u64)>,
    }

    impl FakeBackend {
        fn raw_json(&self) -> String {
            serde_json::to_string(&self.state.lock().unwrap().0).unwrap()
        }
        fn bump(&self) {
            self.state.lock().unwrap().1 += 1;
        }
    }

    impl KeyBackend for &FakeBackend {
        async fn load(&self) -> Result<Snapshot, KeyError> {
            let s = self.state.lock().unwrap();
            Ok(Snapshot {
                keys: s.0.clone(),
                version: Some(s.1.to_string()),
            })
        }
        async fn save(&self, keys: &[StoredKey], version: Option<&str>) -> Result<(), KeyError> {
            let mut s = self.state.lock().unwrap();
            if version != Some(s.1.to_string().as_str()) {
                return Err(KeyError::Conflict);
            }
            s.0 = keys.to_vec();
            s.1 += 1;
            Ok(())
        }
    }

    fn pepper() -> Option<Pepper> {
        Pepper::from_values(Some("0123456789abcdef-pepper"), None)
    }

    fn spec(name: &str, role: &str) -> CreateSpec {
        CreateSpec {
            name: name.into(),
            role: role.into(),
            scopes: vec![],
            expires_in_days: None,
        }
    }

    #[tokio::test]
    async fn create_list_verify_revoke() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, pepper());
        let now = Utc::now();
        let (sum, token) = store
            .create(spec("ci", "write"), "admin", now)
            .await
            .unwrap();
        assert!(token.starts_with("vk_"));
        assert_eq!(sum.status, "active");
        assert_eq!(sum.role, "write");

        let v = store.verify(&token, now).await.unwrap();
        assert_eq!((v.name.as_str(), v.role.as_str()), ("ci", "write"));

        let listed = store.list(false, now).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, sum.id);

        // Revoke by name; verification stops immediately, record is kept and marked.
        let r = store.revoke("ci", "admin", now).await.unwrap();
        assert_eq!(r.status, "revoked");
        assert_eq!(store.verify(&token, now).await, Err(VerifyError::Revoked));
        assert!(store.list(false, now).await.unwrap().is_empty());
        let all = store.list(true, now).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, "revoked");
        // Second revoke finds nothing active.
        assert!(matches!(
            store.revoke(&sum.id, "admin", now).await,
            Err(KeyError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn plaintext_is_never_persisted() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, pepper());
        let (_, token) = store
            .create(spec("ci", "admin"), "a", Utc::now())
            .await
            .unwrap();
        let raw = b.raw_json();
        let secret = token.rsplit('_').next().unwrap();
        assert!(!raw.contains(secret));
        assert!(!raw.contains(&token));
        assert!(raw.contains("\"hash\""));
    }

    #[tokio::test]
    async fn expiry_is_enforced() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, pepper());
        let now = Utc::now();
        let mut s = spec("short", "readonly");
        s.expires_in_days = Some(1);
        let (sum, token) = store.create(s, "a", now).await.unwrap();
        assert!(sum.expires_at.is_some());
        assert!(
            store
                .verify(&token, now + Duration::hours(23))
                .await
                .is_ok()
        );
        assert_eq!(
            store.verify(&token, now + Duration::hours(25)).await,
            Err(VerifyError::Expired)
        );
        let l = store.list(true, now + Duration::hours(25)).await.unwrap();
        assert_eq!(l[0].status, "expired");
        assert!(
            store
                .list(false, now + Duration::hours(25))
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn hash_verification_rejects_wrong_tampered_and_other_pepper() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, pepper());
        let now = Utc::now();
        let (sum, token) = store.create(spec("k", "admin"), "a", now).await.unwrap();

        // Same id, different secret.
        let forged = format!("vk_{}_{}", sum.id, "0".repeat(64));
        assert_eq!(store.verify(&forged, now).await, Err(VerifyError::Unknown));
        // Unknown id, malformed forms.
        let other = format!("vk_{}_{}", "f".repeat(16), "0".repeat(64));
        assert_eq!(store.verify(&other, now).await, Err(VerifyError::Unknown));
        for bad in [
            "",
            "vk_",
            "vk_abc",
            "vk_abc_def",
            "bearer-token",
            &token[1..],
        ] {
            assert!(store.verify(bad, now).await.is_err(), "{bad}");
        }
        // A server with a different pepper cannot validate it.
        let wrong = KeyStore::new(
            &b,
            Pepper::from_values(Some("another-pepper-value-xyz"), None),
        );
        assert_eq!(wrong.verify(&token, now).await, Err(VerifyError::Unknown));
        // Tampering with the stored hash breaks verification.
        b.state.lock().unwrap().0[0].hash = "00".repeat(32);
        assert_eq!(store.verify(&token, now).await, Err(VerifyError::Unknown));
    }

    #[tokio::test]
    async fn salts_differ_and_hashes_are_salted() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, pepper());
        let now = Utc::now();
        store.create(spec("a", "admin"), "x", now).await.unwrap();
        store.create(spec("b", "admin"), "x", now).await.unwrap();
        let st = b.state.lock().unwrap();
        assert_ne!(st.0[0].salt, st.0[1].salt);
        assert_ne!(st.0[0].hash, st.0[1].hash);
        // Same token, different salt or pepper, gives a different hash.
        let p = pepper().unwrap();
        let q = Pepper::from_values(Some("another-pepper-value-xyz"), None).unwrap();
        assert_ne!(hash_token(&p, b"s1", "t"), hash_token(&p, b"s2", "t"));
        assert_ne!(hash_token(&p, b"s1", "t"), hash_token(&q, b"s1", "t"));
    }

    #[tokio::test]
    async fn input_validation_and_limits() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, pepper());
        let now = Utc::now();
        for bad in ["", "has space", "a:b", "a,b", &"x".repeat(65)] {
            assert!(matches!(
                store.create(spec(bad, "admin"), "a", now).await,
                Err(KeyError::Invalid(_))
            ));
        }
        assert!(matches!(
            store.create(spec("ok", "root"), "a", now).await,
            Err(KeyError::Invalid(_))
        ));
        let mut s = spec("ok", "admin");
        s.scopes = vec!["Bad_NS".into()];
        assert!(store.create(s, "a", now).await.is_err());
        for days in [0, MAX_EXPIRY_DAYS + 1] {
            let mut s = spec("ok", "admin");
            s.expires_in_days = Some(days);
            assert!(store.create(s, "a", now).await.is_err());
        }
        store.create(spec("dup", "admin"), "a", now).await.unwrap();
        assert!(matches!(
            store.create(spec("dup", "write"), "a", now).await,
            Err(KeyError::Duplicate(_))
        ));
        // A revoked name can be reused.
        store.revoke("dup", "a", now).await.unwrap();
        store.create(spec("dup", "write"), "a", now).await.unwrap();
    }

    #[tokio::test]
    async fn no_pepper_means_no_keys() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, None);
        assert!(matches!(
            store.create(spec("k", "admin"), "a", Utc::now()).await,
            Err(KeyError::NoPepper)
        ));
        assert!(store.verify("vk_x_y", Utc::now()).await.is_err());
    }

    #[tokio::test]
    async fn concurrent_writer_is_a_conflict() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, pepper());
        // Simulate another writer landing between our load and save.
        struct Racy<'a>(&'a FakeBackend);
        impl KeyBackend for Racy<'_> {
            async fn load(&self) -> Result<Snapshot, KeyError> {
                let s = self.0.load().await;
                self.0.bump();
                s
            }
            async fn save(&self, k: &[StoredKey], v: Option<&str>) -> Result<(), KeyError> {
                self.0.save(k, v).await
            }
        }
        let racy = KeyStore::new(Racy(&b), pepper());
        assert!(matches!(
            racy.create(spec("k", "admin"), "a", Utc::now()).await,
            Err(KeyError::Conflict)
        ));
        assert!(
            store
                .create(spec("k", "admin"), "a", Utc::now())
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn old_inactive_records_age_out() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, pepper());
        let t0 = Utc::now();
        store.create(spec("old", "admin"), "a", t0).await.unwrap();
        store.revoke("old", "a", t0).await.unwrap();
        let later = t0 + Duration::days(RETENTION_DAYS + 1);
        store
            .create(spec("new", "admin"), "a", later)
            .await
            .unwrap();
        let all = store.list(true, later).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "new");
    }

    #[test]
    fn scope_enforcement() {
        let ns = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // Unrestricted.
        assert!(scopes_allow_path(&[], "/api/v1/nodes"));
        assert!(scopes_allow_path(&ns(&["*"]), "/api/v1/nodes"));
        // Scoped: only path-embedded namespaces in scope.
        let s = ns(&["team-a"]);
        assert!(scopes_allow_path(&s, "/api/v1/vms/team-a/web"));
        assert!(scopes_allow_path(&s, "/api/v1/vms/team-a/web/start"));
        assert!(scopes_allow_path(&s, "/api/v1/veyron/vms/team-a/web"));
        assert!(!scopes_allow_path(&s, "/api/v1/vms/team-b/web"));
        assert!(!scopes_allow_path(&s, "/api/v1/vms/team-b/web/team-a"));
        // Cluster-wide, listing, and query-only namespaces are denied.
        assert!(!scopes_allow_path(&s, "/api/v1/vms"));
        assert!(!scopes_allow_path(&s, "/api/v1/vms/team-a"));
        assert!(!scopes_allow_path(&s, "/api/v1/nodes"));
        assert!(!scopes_allow_path(&s, "/api/v1/auth/api-keys"));
        assert!(!scopes_allow_path(&s, "/api/v1/vms/*/web"));
        assert!(!scopes_allow_path(&s, "/vms/team-a/web"));
    }

    #[tokio::test]
    async fn verified_scopes_are_carried() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, pepper());
        let now = Utc::now();
        let mut s = spec("tenant", "write");
        s.scopes = vec!["team-a".into(), "team-a".into()];
        let (sum, token) = store.create(s, "a", now).await.unwrap();
        assert_eq!(sum.scopes, vec!["team-a".to_string()]);
        let v = store.verify(&token, now).await.unwrap();
        assert!(v.allows_path("/api/v1/vms/team-a/x"));
        assert!(!v.allows_path("/api/v1/vms/team-b/x"));
        assert!(!v.allows_path("/api/v1/nodes"));
    }

    #[test]
    fn pepper_sources() {
        assert!(Pepper::from_values(None, None).is_none());
        assert!(Pepper::from_values(Some("short"), Some("primary-key")).is_none());
        let a = Pepper::from_values(None, Some("primary-key")).unwrap();
        let b = Pepper::from_values(None, Some("primary-key2")).unwrap();
        assert_ne!(a.id(), b.id());
        assert_eq!(format!("{a:?}"), "Pepper(..)");
    }

    #[test]
    fn ct_eq_basics() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"ab"));
    }

    /// Env keys are untouched: the store only ever sees tokens with the `vk_` prefix, and an
    /// env-style key (any other shape) is never accepted by it.
    #[tokio::test]
    async fn env_style_keys_are_not_store_tokens() {
        let b = FakeBackend::default();
        let store = KeyStore::new(&b, pepper());
        store
            .create(spec("k", "admin"), "a", Utc::now())
            .await
            .unwrap();
        for env_key in ["0123456789abcdef0123456789abcdef", "Admin@321", "ci-key"] {
            assert!(!is_store_token(env_key));
            assert!(store.verify(env_key, Utc::now()).await.is_err());
        }
        assert!(is_store_token("vk_abc_def"));
    }
}
