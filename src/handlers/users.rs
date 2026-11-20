// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! `veyron auth-user {create,list,delete,set-password}` — dashboard login
//! accounts, backed by the `veyron-users` Secret (`src/kube/user_store.rs`).
//! This bootstraps the very first admin account without already holding one.

use anyhow::{Context, Result};

use crate::cli::AuthUserCommands;
use crate::kube::KubeClient;
use crate::kube::user_store;
use crate::tui::colors::cli as color;

pub async fn handle_auth_user(command: AuthUserCommands, namespace: &str) -> Result<()> {
    let client = KubeClient::new().await.context("connect to cluster")?;

    match command {
        AuthUserCommands::Create {
            username,
            role,
            display_name,
        } => {
            if !user_store::validate_username(&username) {
                anyhow::bail!("invalid username '{username}'");
            }
            if !user_store::validate_role(&role) {
                anyhow::bail!("invalid role '{role}' (expected admin, write, or readonly)");
            }
            let password = dialoguer::Password::new()
                .with_prompt(format!("Password for '{username}'"))
                .with_confirmation("Confirm password", "Passwords didn't match")
                .interact()
                .context("read password")?;

            client
                .create_user(
                    namespace,
                    &username,
                    &password,
                    &role,
                    display_name.as_deref().unwrap_or(""),
                )
                .await?;
            println!(
                "{}",
                color::success(&format!(
                    "Dashboard login account '{username}' created ({role})"
                ))
            );
        }
        AuthUserCommands::List => {
            let users = client.list_users(namespace).await?;
            if users.is_empty() {
                println!("{}", color::muted("No dashboard login accounts yet."));
                println!(
                    "{}",
                    color::muted("Create one with: veyron auth-user create <username>")
                );
                return Ok(());
            }
            println!("{}", color::header("Dashboard login accounts"));
            for u in users {
                println!(
                    "  {}  {}  {}  {}",
                    color::value(&u.username),
                    color::resource(&u.role, "cpu"),
                    color::muted(&u.display_name),
                    color::muted(&u.created_at)
                );
            }
        }
        AuthUserCommands::Delete { username } => {
            let confirmed = dialoguer::Confirm::new()
                .with_prompt(format!(
                    "Delete dashboard login account '{username}'? This cannot be undone."
                ))
                .default(false)
                .interact()
                .unwrap_or(false);
            if !confirmed {
                println!("{}", color::muted("Cancelled."));
                return Ok(());
            }
            client.delete_user(namespace, &username).await?;
            println!(
                "{}",
                color::success(&format!("Dashboard login account '{username}' deleted"))
            );
        }
        AuthUserCommands::SetPassword { username } => {
            if client.get_user(namespace, &username).await?.is_none() {
                anyhow::bail!("user '{username}' not found");
            }
            let password = dialoguer::Password::new()
                .with_prompt(format!("New password for '{username}'"))
                .with_confirmation("Confirm password", "Passwords didn't match")
                .interact()
                .context("read password")?;
            client
                .set_user_password(namespace, &username, &password)
                .await?;
            println!(
                "{}",
                color::success(&format!("Password updated for '{username}'"))
            );
        }
    }

    Ok(())
}
