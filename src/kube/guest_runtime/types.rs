// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuestRuntimeKind {
    GuestKit,
    QemuGa,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestRuntimeMode {
    Auto,
    GuestKit,
    QemuGa,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuestRuntimeStatus {
    pub connected: bool,
    #[serde(rename = "guest_runtime")]
    pub runtime_kind: String,
    #[serde(rename = "guest_agent_connected")]
    pub guest_agent_connected: bool,
    pub version: Option<String>,
    pub capabilities: Option<serde_json::Value>,
}
