// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// Backup Scheduling - Automated backup scheduling

use chrono::{DateTime, NaiveTime, Utc, Weekday};
use serde::{Deserialize, Serialize};

/// Backup schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
    pub name: String,
    pub vm_selector: VMSelector,
    pub schedule_type: ScheduleType,
    pub enabled: bool,
    pub next_run: Option<DateTime<Utc>>,
    pub last_run: Option<DateTime<Utc>>,
}

impl BackupSchedule {
    pub fn new(name: impl Into<String>, schedule_type: ScheduleType) -> Self {
        Self {
            name: name.into(),
            vm_selector: VMSelector::All,
            schedule_type,
            enabled: true,
            next_run: None,
            last_run: None,
        }
    }

    pub fn with_selector(mut self, selector: VMSelector) -> Self {
        self.vm_selector = selector;
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Calculate next run time
    pub fn calculate_next_run(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if !self.enabled {
            return None;
        }

        match &self.schedule_type {
            ScheduleType::Hourly { minute } => {
                Some(crate::utils::schedule::next_hourly(from, *minute))
            }
            ScheduleType::Daily { time } => Some(crate::utils::schedule::next_daily(from, time)),
            ScheduleType::Weekly { weekday, time } => {
                Some(crate::utils::schedule::next_weekly(from, *weekday, time))
            }
            ScheduleType::Monthly { day, time } => {
                Some(crate::utils::schedule::next_monthly(from, *day, time))
            }
            ScheduleType::Cron { expression } => {
                crate::utils::cron::next_cron_time(from, expression)
            }
        }
    }
}

/// Schedule type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleType {
    Hourly { minute: u32 },
    Daily { time: NaiveTime },
    Weekly { weekday: Weekday, time: NaiveTime },
    Monthly { day: u32, time: NaiveTime },
    Cron { expression: String },
}

impl ScheduleType {
    pub fn hourly(minute: u32) -> Self {
        if minute > 59 {
            log::warn!(
                "Hourly schedule minute {} is out of range 0-59, clamping",
                minute
            );
        }
        ScheduleType::Hourly {
            minute: minute.min(59),
        }
    }

    pub fn daily(hour: u32, minute: u32) -> Self {
        // SAFETY: from_hms_opt(0,0,0) is always valid, so the fallback never fails
        const MIDNIGHT: NaiveTime = match NaiveTime::from_hms_opt(0, 0, 0) {
            Some(t) => t,
            None => unreachable!(),
        };
        ScheduleType::Daily {
            time: NaiveTime::from_hms_opt(hour.min(23), minute.min(59), 0).unwrap_or(MIDNIGHT),
        }
    }

    pub fn weekly(weekday: Weekday, hour: u32, minute: u32) -> Self {
        const MIDNIGHT: NaiveTime = match NaiveTime::from_hms_opt(0, 0, 0) {
            Some(t) => t,
            None => unreachable!(),
        };
        ScheduleType::Weekly {
            weekday,
            time: NaiveTime::from_hms_opt(hour.min(23), minute.min(59), 0).unwrap_or(MIDNIGHT),
        }
    }

    pub fn monthly(day: u32, hour: u32, minute: u32) -> Self {
        const MIDNIGHT: NaiveTime = match NaiveTime::from_hms_opt(0, 0, 0) {
            Some(t) => t,
            None => unreachable!(),
        };
        if day == 0 || day > 31 {
            log::warn!(
                "Monthly schedule day {} is out of range 1-31, clamping",
                day
            );
        }
        ScheduleType::Monthly {
            day: day.clamp(1, 31),
            time: NaiveTime::from_hms_opt(hour.min(23), minute.min(59), 0).unwrap_or(MIDNIGHT),
        }
    }
}

/// VM selector for scheduled backups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VMSelector {
    All,
    ByName(Vec<String>),
    ByLabel { key: String, value: String },
}

/// Schedule manager
#[derive(Serialize, Deserialize)]
pub struct ScheduleManager {
    schedules: Vec<BackupSchedule>,
}

impl ScheduleManager {
    fn persistence_path() -> anyhow::Result<std::path::PathBuf> {
        Ok(crate::utils::data_dir()?.join("backup_schedules.json"))
    }

    pub fn load() -> Self {
        let path = match Self::persistence_path() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to determine data directory: {}", e);
                return Self::new();
            }
        };
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(manager) => return manager,
                    Err(e) => log::warn!("Failed to parse backup schedules: {}", e),
                },
                Err(e) => log::warn!("Failed to read backup schedules: {}", e),
            }
        }
        Self::new()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::persistence_path()?;
        crate::utils::atomic_write(&path, self)
    }

    pub fn new() -> Self {
        Self {
            schedules: Vec::new(),
        }
    }

    pub fn add_schedule(&mut self, schedule: BackupSchedule) {
        self.schedules.push(schedule);
        if let Err(e) = self.save() {
            log::warn!("Failed to persist backup schedules: {}", e);
        }
    }

    pub fn remove_schedule(&mut self, name: &str) -> bool {
        if let Some(pos) = self.schedules.iter().position(|s| s.name == name) {
            self.schedules.remove(pos);
            if let Err(e) = self.save() {
                log::warn!("Failed to persist backup schedules: {}", e);
            }
            true
        } else {
            false
        }
    }

    pub fn get_schedule(&self, name: &str) -> Option<&BackupSchedule> {
        self.schedules.iter().find(|s| s.name == name)
    }

    /// Get schedules due to run
    pub fn get_due_schedules(&self, now: DateTime<Utc>) -> Vec<&BackupSchedule> {
        self.schedules
            .iter()
            // Only include schedules that have a calculated next_run time that has passed
            .filter(|s| s.enabled && s.next_run.map(|next| next <= now).unwrap_or(false))
            .collect()
    }

    /// Get all enabled schedules
    pub fn get_enabled(&self) -> Vec<&BackupSchedule> {
        self.schedules.iter().filter(|s| s.enabled).collect()
    }

    pub fn schedule_count(&self) -> usize {
        self.schedules.len()
    }
}

impl Default for ScheduleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_backup_schedule() {
        let schedule = BackupSchedule::new("daily-backup", ScheduleType::daily(2, 0))
            .with_selector(VMSelector::All);

        assert_eq!(schedule.name, "daily-backup");
        assert!(schedule.enabled);
    }

    #[test]
    fn test_schedule_types() {
        let hourly = ScheduleType::hourly(30);
        match hourly {
            ScheduleType::Hourly { minute } => assert_eq!(minute, 30),
            _ => panic!("Expected Hourly"),
        }

        let daily = ScheduleType::daily(2, 30);
        match daily {
            ScheduleType::Daily { time } => {
                assert_eq!(time.hour(), 2);
                assert_eq!(time.minute(), 30);
            }
            _ => panic!("Expected Daily"),
        }

        let weekly = ScheduleType::weekly(Weekday::Sun, 3, 0);
        match weekly {
            ScheduleType::Weekly { weekday, time } => {
                assert_eq!(weekday, Weekday::Sun);
                assert_eq!(time.hour(), 3);
            }
            _ => panic!("Expected Weekly"),
        }
    }

    #[test]
    fn test_vm_selector() {
        let all = VMSelector::All;
        match all {
            VMSelector::All => {}
            _ => panic!("Expected All"),
        }

        let by_name = VMSelector::ByName(vec!["vm1".to_string(), "vm2".to_string()]);
        match by_name {
            VMSelector::ByName(names) => assert_eq!(names.len(), 2),
            _ => panic!("Expected ByName"),
        }

        let by_label = VMSelector::ByLabel {
            key: "env".to_string(),
            value: "prod".to_string(),
        };
        match by_label {
            VMSelector::ByLabel { key, value } => {
                assert_eq!(key, "env");
                assert_eq!(value, "prod");
            }
            _ => panic!("Expected ByLabel"),
        }
    }

    #[test]
    fn test_schedule_manager() {
        let mut manager = ScheduleManager::new();

        let schedule = BackupSchedule::new("test-schedule", ScheduleType::daily(2, 0));
        manager.add_schedule(schedule);

        assert_eq!(manager.schedule_count(), 1);
        assert!(manager.get_schedule("test-schedule").is_some());

        assert!(manager.remove_schedule("test-schedule"));
        assert_eq!(manager.schedule_count(), 0);
    }

    #[test]
    fn test_calculate_next_run_hourly() {
        let schedule = BackupSchedule::new("hourly", ScheduleType::hourly(30));

        let now = Utc::now().with_minute(0).unwrap();
        let next = schedule.calculate_next_run(now).unwrap();

        assert_eq!(next.minute(), 30);
    }

    #[test]
    fn test_disabled_schedule() {
        let schedule = BackupSchedule::new("disabled", ScheduleType::daily(2, 0)).disable();

        assert!(!schedule.enabled);
        assert!(schedule.calculate_next_run(Utc::now()).is_none());
    }

    #[test]
    fn test_get_enabled_schedules() {
        let mut manager = ScheduleManager::new();

        manager.add_schedule(BackupSchedule::new("enabled1", ScheduleType::daily(2, 0)));
        manager.add_schedule(BackupSchedule::new("disabled", ScheduleType::daily(3, 0)).disable());
        manager.add_schedule(BackupSchedule::new("enabled2", ScheduleType::daily(4, 0)));

        let enabled = manager.get_enabled();
        assert_eq!(enabled.len(), 2);
    }
}
