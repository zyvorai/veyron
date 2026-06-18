// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

pub use crate::kube::kubevirt_subresources::vmi_guest_exec;

use anyhow::Result;
use kube::Client;
use serde_json::{Value, json};

pub async fn guest_info(client: Client, namespace: &str, vmi_name: &str) -> Result<Value> {
    let body = json!({ "execute": "guest-info" });
    crate::kube::guestkit_client::qga_execute_for_vmi(client, namespace, vmi_name, body).await
}
