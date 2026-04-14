// Session Sharing - Share TUI/Web sessions with other users

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::generate_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManager {
    pub sessions: Vec<SharedSession>,
    pub config: SharingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSession {
    pub id: String,
    pub owner: String,
    pub participants: Vec<Participant>,
    pub permissions: SessionPermissions,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub namespace: String,
    pub view_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub user_id: String,
    pub display_name: String,
    pub role: ParticipantRole,
    pub joined_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParticipantRole {
    Owner,
    Editor,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPermissions {
    pub can_modify: bool,
    pub can_execute_actions: bool,
    pub can_invite: bool,
    pub max_participants: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Paused,
    Expired,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingConfig {
    pub enabled: bool,
    pub default_expiry_hours: u64,
    pub max_sessions: usize,
    pub require_auth: bool,
}

impl Default for SharingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_expiry_hours: 24,
            max_sessions: 10,
            require_auth: true,
        }
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            config: SharingConfig::default(),
        }
    }

    pub fn create_session(&mut self, owner: &str, namespace: &str) -> String {
        let id = generate_id("sess", owner);
        let session = SharedSession {
            id: id.clone(),
            owner: owner.to_string(),
            participants: vec![Participant {
                user_id: owner.to_string(),
                display_name: owner.to_string(),
                role: ParticipantRole::Owner,
                joined_at: Utc::now(),
                last_active: Utc::now(),
            }],
            permissions: SessionPermissions {
                can_modify: true,
                can_execute_actions: true,
                can_invite: true,
                max_participants: 10,
            },
            status: SessionStatus::Active,
            created_at: Utc::now(),
            expires_at: Some(
                Utc::now() + chrono::TimeDelta::hours(self.config.default_expiry_hours as i64),
            ),
            namespace: namespace.to_string(),
            view_state: String::new(),
        };
        self.sessions.push(session);
        id
    }

    pub fn join_session(&mut self, session_id: &str, user_id: &str, name: &str) -> bool {
        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id && s.status == SessionStatus::Active)
        {
            if session.participants.len() >= session.permissions.max_participants {
                return false;
            }
            session.participants.push(Participant {
                user_id: user_id.to_string(),
                display_name: name.to_string(),
                role: ParticipantRole::Viewer,
                joined_at: Utc::now(),
                last_active: Utc::now(),
            });
            true
        } else {
            false
        }
    }

    pub fn close_session(&mut self, session_id: &str) {
        if let Some(s) = self.sessions.iter_mut().find(|s| s.id == session_id) {
            s.status = SessionStatus::Closed;
        }
    }

    pub fn active_sessions(&self) -> Vec<&SharedSession> {
        self.sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Active)
            .collect()
    }

    pub fn get_session(&self, id: &str) -> Option<&SharedSession> {
        self.sessions.iter().find(|s| s.id == id)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
