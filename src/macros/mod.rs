// Macro System - Record and playback user actions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroManager {
    pub macros: Vec<Macro>,
    pub recording: Option<MacroRecording>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub description: String,
    pub actions: Vec<MacroAction>,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAction {
    pub action_type: ActionType,
    pub target: String,
    pub parameters: std::collections::HashMap<String, String>,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    StartVm,
    StopVm,
    RestartVm,
    DeleteVm,
    CreateSnapshot,
    RestoreSnapshot,
    MigrateVm,
    ScaleResources,
    NavigateView,
    ExecuteCommand,
    Wait,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroRecording {
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub actions: Vec<MacroAction>,
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub macro_id: String,
    pub current_step: usize,
    pub total_steps: usize,
    pub status: PlaybackStatus,
    pub started_at: DateTime<Utc>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackStatus {
    Running,
    Paused,
    Completed,
    Failed,
}

impl MacroManager {
    pub fn new() -> Self {
        Self {
            macros: Vec::new(),
            recording: None,
        }
    }

    pub fn start_recording(&mut self, name: &str) {
        self.recording = Some(MacroRecording {
            name: name.to_string(),
            started_at: Utc::now(),
            actions: Vec::new(),
        });
    }

    pub fn record_action(&mut self, action: MacroAction) {
        if let Some(ref mut recording) = self.recording {
            recording.actions.push(action);
        }
    }

    pub fn stop_recording(&mut self, description: &str) -> Option<String> {
        if let Some(recording) = self.recording.take() {
            let id = format!("macro-{}", Utc::now().timestamp_micros());
            let m = Macro {
                id: id.clone(),
                name: recording.name,
                description: description.to_string(),
                actions: recording.actions,
                created_at: Utc::now(),
                last_run: None,
                run_count: 0,
                tags: Vec::new(),
            };
            self.macros.push(m);
            Some(id)
        } else {
            None
        }
    }

    pub fn cancel_recording(&mut self) {
        self.recording = None;
    }

    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    pub fn get_macro(&self, id: &str) -> Option<&Macro> {
        self.macros.iter().find(|m| m.id == id)
    }

    pub fn delete_macro(&mut self, id: &str) -> bool {
        let len = self.macros.len();
        self.macros.retain(|m| m.id != id);
        self.macros.len() < len
    }

    pub fn play(&mut self, id: &str) -> Option<PlaybackState> {
        if let Some(m) = self.macros.iter_mut().find(|m| m.id == id) {
            m.run_count += 1;
            m.last_run = Some(Utc::now());
            Some(PlaybackState {
                macro_id: id.to_string(),
                current_step: 0,
                total_steps: m.actions.len(),
                status: PlaybackStatus::Running,
                started_at: Utc::now(),
                errors: Vec::new(),
            })
        } else {
            None
        }
    }

    pub fn list(&self) -> &[Macro] {
        &self.macros
    }
}

impl Default for MacroManager {
    fn default() -> Self {
        Self::new()
    }
}
