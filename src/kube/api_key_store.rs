// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Secret-backed persistence for [`crate::api::key_store`].
//!
//! All records live in one Secret (`veyron-api-keys`, key `keys.json`), the same shape as
//! `veyron-users`. Writes use `replace` with the Secret's `resourceVersion`, so two admins
//! creating keys at once cannot silently drop each other's record (the loser gets a conflict).
//! Only hashes and metadata are stored, never a token.

use super::KubeClient;
use crate::api::key_store::{KeyBackend, KeyError, Snapshot, StoredKey};
use k8s_openapi::ByteString;
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::PostParams;
use std::collections::BTreeMap;

pub const API_KEYS_SECRET_NAME: &str = "veyron-api-keys";
const API_KEYS_SECRET_KEY: &str = "keys.json";

/// Short-lived read cache for the per-request check. Cleared by every write through this
/// process; other API replicas see a change within the TTL (a revocation takes up to that long).
static CACHE: std::sync::Mutex<Option<(std::time::Instant, Vec<StoredKey>)>> =
    std::sync::Mutex::new(None);
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(5);

fn clear_cache() {
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

pub struct KubeKeyBackend {
    client: kube::Client,
    namespace: String,
}

impl KubeKeyBackend {
    pub fn new(client: &KubeClient, namespace: &str) -> Self {
        Self {
            client: client.client(),
            namespace: namespace.to_string(),
        }
    }

    fn api(&self) -> kube::Api<Secret> {
        kube::Api::namespaced(self.client.clone(), &self.namespace)
    }
}

fn be(e: impl std::fmt::Display) -> KeyError {
    KeyError::Backend(e.to_string())
}

fn decode(secret: &Secret) -> Result<Vec<StoredKey>, KeyError> {
    let Some(raw) = secret
        .data
        .as_ref()
        .and_then(|d| d.get(API_KEYS_SECRET_KEY))
    else {
        return Ok(Vec::new());
    };
    // A corrupt record set must not read as "no keys" and then be overwritten on the next write.
    serde_json::from_slice(&raw.0).map_err(|e| be(format!("corrupt {API_KEYS_SECRET_NAME}: {e}")))
}

impl KeyBackend for KubeKeyBackend {
    async fn load(&self) -> Result<Snapshot, KeyError> {
        match self.api().get(API_KEYS_SECRET_NAME).await {
            Ok(secret) => Ok(Snapshot {
                keys: decode(&secret)?,
                version: secret.metadata.resource_version.clone(),
            }),
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(Snapshot::default()),
            Err(e) => Err(be(e)),
        }
    }

    async fn save(&self, keys: &[StoredKey], version: Option<&str>) -> Result<(), KeyError> {
        clear_cache();
        let body = serde_json::to_vec(keys).map_err(be)?;
        let data = BTreeMap::from([(API_KEYS_SECRET_KEY.to_string(), ByteString(body))]);
        let secrets = self.api();
        let conflict = |e: kube::Error| match e {
            kube::Error::Api(ref a) if a.code == 409 => KeyError::Conflict,
            other => be(other),
        };
        match version {
            Some(rv) => {
                let secret = Secret {
                    metadata: ObjectMeta {
                        name: Some(API_KEYS_SECRET_NAME.to_string()),
                        namespace: Some(self.namespace.clone()),
                        resource_version: Some(rv.to_string()),
                        labels: Some(BTreeMap::from([(
                            "veyron.io/managed-by".to_string(),
                            "veyron".to_string(),
                        )])),
                        ..Default::default()
                    },
                    data: Some(data),
                    ..Default::default()
                };
                secrets
                    .replace(API_KEYS_SECRET_NAME, &PostParams::default(), &secret)
                    .await
                    .map(|_| ())
                    .map_err(conflict)
            }
            None => {
                let secret = Secret {
                    metadata: ObjectMeta {
                        name: Some(API_KEYS_SECRET_NAME.to_string()),
                        namespace: Some(self.namespace.clone()),
                        labels: Some(BTreeMap::from([(
                            "veyron.io/managed-by".to_string(),
                            "veyron".to_string(),
                        )])),
                        ..Default::default()
                    },
                    data: Some(data),
                    ..Default::default()
                };
                secrets
                    .create(&PostParams::default(), &secret)
                    .await
                    .map(|_| ())
                    .map_err(conflict)
            }
        }
    }
}

impl KubeClient {
    /// Records for the per-request check, cached for a few seconds. Errors are not cached and
    /// the caller must treat them as "not authenticated".
    pub async fn cached_api_keys(&self, namespace: &str) -> Result<Vec<StoredKey>, KeyError> {
        {
            let c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((at, keys)) = c.as_ref()
                && at.elapsed() < CACHE_TTL
            {
                return Ok(keys.clone());
            }
        }
        let keys = KubeKeyBackend::new(self, namespace).load().await?.keys;
        *CACHE.lock().unwrap_or_else(|e| e.into_inner()) =
            Some((std::time::Instant::now(), keys.clone()));
        Ok(keys)
    }
}
