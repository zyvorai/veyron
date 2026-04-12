// Notifications - In-app notification system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::generate_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationManager {
    pub notifications: Vec<Notification>,
    pub max_notifications: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub message: String,
    pub notification_type: NotificationType,
    pub created_at: DateTime<Utc>,
    pub duration_secs: u64,
    pub source: String,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationType { Success, Info, Warning, Error }

impl Notification {
    pub fn new(message: &str, notification_type: NotificationType) -> Self {
        Self {
            id: generate_id("notif", "event"),
            message: message.to_string(), notification_type, created_at: Utc::now(),
            duration_secs: 10, source: String::new(), read: false,
        }
    }

    pub fn with_source(mut self, source: &str) -> Self { self.source = source.to_string(); self }
    pub fn with_duration(mut self, secs: u64) -> Self { self.duration_secs = secs; self }

    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.created_at).num_seconds();
        elapsed > self.duration_secs as i64
    }

    pub fn remaining_ratio(&self) -> f32 {
        let elapsed = Utc::now().signed_duration_since(self.created_at).num_seconds() as f32;
        (1.0 - elapsed / self.duration_secs as f32).max(0.0)
    }
}

impl NotificationManager {
    pub fn new(max: usize) -> Self {
        Self { notifications: Vec::new(), max_notifications: max }
    }

    pub fn notify(&mut self, notification: Notification) {
        self.notifications.push(notification);
        if self.notifications.len() > self.max_notifications {
            let excess = self.notifications.len() - self.max_notifications;
            self.notifications.drain(0..excess);
        }
    }

    pub fn success(&mut self, msg: &str) { self.notify(Notification::new(msg, NotificationType::Success)); }
    pub fn info(&mut self, msg: &str) { self.notify(Notification::new(msg, NotificationType::Info)); }
    pub fn warning(&mut self, msg: &str) { self.notify(Notification::new(msg, NotificationType::Warning)); }
    pub fn error(&mut self, msg: &str) { self.notify(Notification::new(msg, NotificationType::Error)); }

    pub fn unread(&self) -> Vec<&Notification> { self.notifications.iter().filter(|n| !n.read && !n.is_expired()).collect() }
    pub fn unread_count(&self) -> usize { self.notifications.iter().filter(|n| !n.read && !n.is_expired()).count() }
    pub fn mark_read(&mut self, id: &str) { if let Some(n) = self.notifications.iter_mut().find(|n| n.id == id) { n.read = true; } }
    pub fn mark_all_read(&mut self) { for n in &mut self.notifications { n.read = true; } }
    pub fn cleanup_expired(&mut self) { self.notifications.retain(|n| !n.is_expired()); }
    pub fn clear(&mut self) { self.notifications.clear(); }
    pub fn active(&self) -> Vec<&Notification> { self.notifications.iter().filter(|n| !n.is_expired()).collect() }
}

impl Default for NotificationManager {
    fn default() -> Self { Self::new(100) }
}
