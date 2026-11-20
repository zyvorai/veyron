// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! 30-day evaluation trial enforcement for the standalone client tarball.
//!
//! Only meaningful when built with `--features trial` (see
//! `scripts/package-binary-remote.sh --trial` and the Dockerfile's
//! `VEYRON_EXTRA_FEATURES` build arg). Every other build — in-cluster
//! Helm/`deploy-remote.sh` images, a plain `cargo build` — compiles the no-op stub
//! below and is never time-limited.
//!
//! Verification is signature-based (Ed25519/EdDSA via `jsonwebtoken` + `ring`, both
//! already dependencies), not a locally-writable timestamp file: the trial token
//! bundled in the tarball is cryptographically signed by a private key held only by
//! ZyvorAI (see `src/bin/trial-tool.rs`), and the binary only ever embeds the matching
//! *public* key. Deleting or editing a local file cannot forge a new valid token or
//! extend the expiry — the only way to get a working trial past expiry is a freshly
//! signed token, i.e. contacting sales.

use serde::{Deserialize, Serialize};

/// Ed25519 public key (raw 32 bytes, standard base64), matching the private key used
/// by `trial-tool issue`. Regenerate with `cargo run --bin trial-tool -- keygen` and
/// replace this constant when rotating the signing key — doing so invalidates every
/// previously issued token, so reissue any trials still active.
pub const TRIAL_PUBLIC_KEY_B64: &str = "Zo2596K+cFrC3r0lvodqfBvU/LJGr8RU43KEAbAFqW0=";

pub const SALES_CONTACT: &str = "sales@zyvor.dev";

/// Marker embedded in every issued token so a token minted for a different ZyvorAI
/// product can never be mistaken for a valid Veyron trial.
pub const PRODUCT_TAG: &str = "veyron-trial";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialClaims {
    /// Who this trial was issued to (freeform — company/contact name).
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
    pub product: String,
}

#[cfg(feature = "trial")]
mod enforce {
    use super::*;
    use base64::Engine;
    use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
    use std::path::PathBuf;

    fn decoding_key() -> Option<DecodingKey> {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(TRIAL_PUBLIC_KEY_B64)
            .ok()?;
        Some(DecodingKey::from_ed_der(&raw))
    }

    /// Checked in order: explicit env override, next to the running binary (how the
    /// client tarball ships it), then the user config dir (in case someone moves it).
    fn candidate_token_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                paths.push(dir.join("trial.token"));
            }
        }
        if let Some(cfg) = dirs::config_dir() {
            paths.push(cfg.join("veyron").join("trial.token"));
        }
        paths.push(PathBuf::from("trial.token"));
        paths
    }

    fn locate_token() -> Option<String> {
        if let Ok(t) = std::env::var("VEYRON_TRIAL_TOKEN") {
            let t = t.trim().to_string();
            if !t.is_empty() {
                return Some(t);
            }
        }
        for p in candidate_token_paths() {
            if let Ok(s) = std::fs::read_to_string(&p) {
                let s = s.trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
        None
    }

    fn verify(token: &str) -> Result<TrialClaims, String> {
        let key = decoding_key().ok_or_else(|| {
            "trial build is missing its embedded public key (build error)".to_string()
        })?;
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.validate_exp = true;
        // Our claims never carry an `aud` — an unconfigured expected audience is only a
        // problem when the token itself *has* one (see the OIDC bearer-validation fix
        // in src/api/oidc.rs); disabling it here is just defensive belt-and-braces.
        validation.validate_aud = false;
        let data = decode::<TrialClaims>(token.trim(), &key, &validation)
            .map_err(|e| format!("invalid or expired trial token ({e})"))?;
        if data.claims.product != PRODUCT_TAG {
            return Err("token was not issued for this product".to_string());
        }
        Ok(data.claims)
    }

    fn print_banner(msg: &str) {
        eprintln!();
        eprintln!("============================================================");
        eprintln!("{msg}");
        eprintln!("Contact {SALES_CONTACT} for a license.");
        eprintln!("============================================================");
        eprintln!();
    }

    pub fn enforce() {
        let Some(token) = locate_token() else {
            print_banner("No Veyron trial license found.");
            std::process::exit(1);
        };

        let claims = match verify(&token) {
            Ok(c) => c,
            Err(e) => {
                print_banner(&format!("Veyron trial license invalid: {e}"));
                std::process::exit(1);
            }
        };

        let now = chrono::Utc::now().timestamp();
        let remaining_secs = claims.exp - now;
        if remaining_secs <= 0 {
            let expired_on = chrono::DateTime::from_timestamp(claims.exp, 0)
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            print_banner(&format!(
                "Veyron trial for \"{}\" expired on {expired_on}.",
                claims.sub
            ));
            std::process::exit(1);
        }

        let remaining_days = remaining_secs / 86_400;
        if remaining_days <= 7 {
            eprintln!(
                "[TRIAL] Veyron trial for \"{}\" — {remaining_days} day(s) remaining. Contact {SALES_CONTACT} to purchase a license.",
                claims.sub
            );
        }
    }

    pub fn status_json() -> serde_json::Value {
        match locate_token().and_then(|t| verify(&t).ok()) {
            Some(claims) => {
                let now = chrono::Utc::now().timestamp();
                let days_remaining = ((claims.exp - now) as f64 / 86_400.0).ceil().max(0.0) as i64;
                serde_json::json!({
                    "trial": true,
                    "licensee": claims.sub,
                    "expires_at": chrono::DateTime::from_timestamp(claims.exp, 0).map(|d| d.to_rfc3339()),
                    "days_remaining": days_remaining,
                    "expired": now >= claims.exp,
                    "sales_contact": SALES_CONTACT,
                })
            }
            None => serde_json::json!({
                "trial": true,
                "licensed": false,
                "sales_contact": SALES_CONTACT,
            }),
        }
    }
}

#[cfg(feature = "trial")]
pub fn enforce_trial() {
    enforce::enforce();
}

#[cfg(not(feature = "trial"))]
pub fn enforce_trial() {}

#[cfg(feature = "trial")]
pub fn trial_status_json() -> serde_json::Value {
    enforce::status_json()
}

#[cfg(not(feature = "trial"))]
pub fn trial_status_json() -> serde_json::Value {
    serde_json::json!({ "trial": false })
}
