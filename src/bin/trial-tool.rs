// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
//! Sales / packaging tooling for Ed25519-signed trial tokens.
//!
//! ```text
//! cargo run --features trial --bin trial-tool -- keygen
//! cargo run --features trial --bin trial-tool -- issue --who "Acme" --days 30 -o trial.token
//! ```
//!
//! Private key stays with ZyvorAI. Only the public key is embedded in the binary
//! (`veyron::trial::TRIAL_PUBLIC_KEY_B64`).

use base64::Engine;
use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::fs;
use std::path::PathBuf;
use veyron::trial::{PRODUCT_TAG, TrialClaims};

#[derive(Parser)]
#[command(
    name = "trial-tool",
    about = "Issue Ed25519-signed Veyron trial tokens"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate a new Ed25519 keypair. Paste the public key into src/trial.rs.
    Keygen {
        #[arg(long, default_value = "secrets/trial-ed25519.pkcs8")]
        out_private: PathBuf,
    },
    /// Sign a trial token (requires private PKCS8 from keygen).
    Issue {
        #[arg(long)]
        who: String,
        #[arg(long, default_value_t = 30)]
        days: i64,
        #[arg(long, default_value = "secrets/trial-ed25519.pkcs8")]
        private_key: PathBuf,
        #[arg(short, long, default_value = "trial.token")]
        output: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Keygen { out_private } => {
            if let Some(parent) = out_private.parent() {
                fs::create_dir_all(parent)?;
            }
            let rng = SystemRandom::new();
            let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng)
                .map_err(|_| anyhow::anyhow!("ed25519 keygen failed"))?;
            fs::write(&out_private, pkcs8.as_ref())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&out_private)?.permissions();
                perms.set_mode(0o600);
                fs::set_permissions(&out_private, perms)?;
            }
            let pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
                .map_err(|e| anyhow::anyhow!("parse pkcs8: {e}"))?;
            let pub_b64 =
                base64::engine::general_purpose::STANDARD.encode(pair.public_key().as_ref());
            println!("Wrote private PKCS8 → {}", out_private.display());
            println!("Paste into src/trial.rs:\n");
            println!("pub const TRIAL_PUBLIC_KEY_B64: &str = \"{pub_b64}\";");
            println!("\nRe-issue any active trials after rotating.");
        }
        Cmd::Issue {
            who,
            days,
            private_key,
            output,
        } => {
            let pkcs8 = fs::read(&private_key)
                .map_err(|e| anyhow::anyhow!("read {}: {e}", private_key.display()))?;
            let now = Utc::now();
            let claims = TrialClaims {
                sub: who.clone(),
                iat: now.timestamp(),
                exp: (now + Duration::days(days)).timestamp(),
                product: PRODUCT_TAG.to_string(),
            };
            let key = EncodingKey::from_ed_der(&pkcs8);
            let token = encode(&Header::new(Algorithm::EdDSA), &claims, &key)?;
            fs::write(&output, format!("{token}\n"))?;
            println!(
                "Issued trial for \"{who}\" → {} (expires in {days} days)",
                output.display()
            );
            println!("Ship this file next to the binary (or set VEYRON_TRIAL_TOKEN).");
        }
    }
    Ok(())
}
