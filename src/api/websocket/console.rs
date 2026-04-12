use serde::{Deserialize, Serialize};

/// Console session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleSessionRequest {
    pub vm_name: String,
    pub namespace: String,
    pub terminal_type: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

/// Console session info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleSession {
    pub session_id: String,
    pub vm_name: String,
    pub namespace: String,
    pub connected: bool,
    pub created_at: String,
}

/// Console message (sent/received over WebSocket)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub session_id: String,
    pub data: String,
    pub message_type: ConsoleMessageType,
}

/// Console message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsoleMessageType {
    Input,
    Output,
    Resize,
    Ping,
    Close,
}
