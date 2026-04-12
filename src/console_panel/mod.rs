// Console Panel - Serial/VNC console session management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleManager {
    pub sessions: Vec<ConsoleSession>,
    pub config: ConsoleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleSession {
    pub id: String,
    pub vm_name: String,
    pub namespace: String,
    pub console_type: ConsoleType,
    pub status: ConsoleStatus,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConsoleType { Serial, VNC, SSH, RDP }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConsoleStatus { Connecting, Connected, Disconnected, Error(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleConfig {
    pub default_type: ConsoleType,
    pub timeout_secs: u64,
    pub max_sessions: usize,
    pub scrollback_lines: usize,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self { default_type: ConsoleType::Serial, timeout_secs: 3600, max_sessions: 5, scrollback_lines: 10000 }
    }
}

impl ConsoleManager {
    pub fn new() -> Self { Self { sessions: Vec::new(), config: ConsoleConfig::default() } }

    pub fn open_session(&mut self, vm_name: &str, namespace: &str, console_type: ConsoleType) -> String {
        let id = format!("console-{}", Utc::now().timestamp_micros());
        self.sessions.push(ConsoleSession {
            id: id.clone(), vm_name: vm_name.to_string(), namespace: namespace.to_string(),
            console_type, status: ConsoleStatus::Connecting,
            started_at: Utc::now(), last_activity: Utc::now(),
        });
        id
    }

    pub fn close_session(&mut self, id: &str) {
        if let Some(s) = self.sessions.iter_mut().find(|s| s.id == id) {
            s.status = ConsoleStatus::Disconnected;
        }
    }

    pub fn active_sessions(&self) -> Vec<&ConsoleSession> {
        self.sessions.iter().filter(|s| s.status == ConsoleStatus::Connected || s.status == ConsoleStatus::Connecting).collect()
    }

    pub fn get_session(&self, id: &str) -> Option<&ConsoleSession> { self.sessions.iter().find(|s| s.id == id) }
}

impl Default for ConsoleManager {
    fn default() -> Self { Self::new() }
}
