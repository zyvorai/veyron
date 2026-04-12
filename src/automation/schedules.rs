// Schedules - Scheduled automation tasks

use chrono::{DateTime, NaiveTime, Utc, Weekday};
use serde::{Deserialize, Serialize};

/// Scheduled task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schedule: Schedule,
    pub rule_id: String,
    pub enabled: bool,
    pub next_run: Option<DateTime<Utc>>,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u64,
    pub created_at: DateTime<Utc>,
}

impl ScheduledTask {
    pub fn new(name: impl Into<String>, schedule: Schedule, rule_id: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "task-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            description: String::new(),
            schedule,
            rule_id: rule_id.into(),
            enabled: true,
            next_run: None,
            last_run: None,
            run_count: 0,
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn calculate_next_run(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if !self.enabled {
            return None;
        }

        self.schedule.next_run_time(from)
    }

    pub fn record_run(&mut self) {
        self.last_run = Some(Utc::now());
        self.run_count += 1;
    }

    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }

        if let Some(next_run) = self.next_run {
            next_run <= now
        } else {
            false
        }
    }
}

/// Schedule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Schedule {
    Once { at: DateTime<Utc> },
    Hourly { minute: u32 },
    Daily { time: NaiveTime },
    Weekly { weekday: Weekday, time: NaiveTime },
    Monthly { day: u32, time: NaiveTime },
    Cron { expression: String },
    Interval { seconds: u64 },
}

impl Schedule {
    pub fn once(at: DateTime<Utc>) -> Self {
        Schedule::Once { at }
    }

    pub fn hourly(minute: u32) -> Self {
        Schedule::Hourly { minute }
    }

    pub fn daily(hour: u32, minute: u32) -> Self {
        Schedule::Daily {
            time: NaiveTime::from_hms_opt(hour.min(23), minute.min(59), 0)
                .unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
        }
    }

    pub fn weekly(weekday: Weekday, hour: u32, minute: u32) -> Self {
        Schedule::Weekly {
            weekday,
            time: NaiveTime::from_hms_opt(hour.min(23), minute.min(59), 0)
                .unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
        }
    }

    pub fn monthly(day: u32, hour: u32, minute: u32) -> Self {
        Schedule::Monthly {
            day,
            time: NaiveTime::from_hms_opt(hour.min(23), minute.min(59), 0)
                .unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
        }
    }

    pub fn interval(seconds: u64) -> Self {
        Schedule::Interval { seconds }
    }

    /// Calculate next run time
    pub fn next_run_time(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Schedule::Once { at } => {
                if *at > from {
                    Some(*at)
                } else {
                    None
                }
            }
            Schedule::Hourly { minute } => Some(crate::utils::schedule::next_hourly(from, *minute)),
            Schedule::Daily { time } => Some(crate::utils::schedule::next_daily(from, time)),
            Schedule::Weekly { weekday, time } => {
                Some(crate::utils::schedule::next_weekly(from, *weekday, time))
            }
            Schedule::Monthly { day, time } => {
                Some(crate::utils::schedule::next_monthly(from, *day, time))
            }
            Schedule::Interval { seconds } => {
                Some(from + chrono::TimeDelta::seconds(*seconds as i64))
            }
            Schedule::Cron { expression } => crate::utils::cron::next_cron_time(from, expression),
        }
    }
}

/// Schedule manager
pub struct ScheduleManager {
    tasks: Vec<ScheduledTask>,
}

impl ScheduleManager {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn add_task(&mut self, task: ScheduledTask) {
        self.tasks.push(task);
    }

    pub fn remove_task(&mut self, task_id: &str) -> bool {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == task_id) {
            self.tasks.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_task(&self, task_id: &str) -> Option<&ScheduledTask> {
        self.tasks.iter().find(|t| t.id == task_id)
    }

    pub fn get_task_mut(&mut self, task_id: &str) -> Option<&mut ScheduledTask> {
        self.tasks.iter_mut().find(|t| t.id == task_id)
    }

    pub fn due_tasks(&self, now: DateTime<Utc>) -> Vec<&ScheduledTask> {
        self.tasks.iter().filter(|t| t.is_due(now)).collect()
    }

    pub fn enabled_tasks(&self) -> Vec<&ScheduledTask> {
        self.tasks.iter().filter(|t| t.enabled).collect()
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Update next run times for all tasks
    pub fn update_next_runs(&mut self, now: DateTime<Utc>) {
        for task in &mut self.tasks {
            task.next_run = task.calculate_next_run(now);
        }
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
    fn test_scheduled_task() {
        let schedule = Schedule::daily(2, 0);
        let task = ScheduledTask::new("Daily Backup", schedule, "rule-123")
            .with_description("Backup all VMs daily");

        assert_eq!(task.name, "Daily Backup");
        assert!(task.enabled);
        assert_eq!(task.run_count, 0);
    }

    #[test]
    fn test_schedule_hourly() {
        let schedule = Schedule::hourly(30);
        let now = Utc::now().with_minute(0).unwrap();
        let next = schedule.next_run_time(now).unwrap();

        assert_eq!(next.minute(), 30);
    }

    #[test]
    fn test_schedule_daily() {
        let schedule = Schedule::daily(2, 30);
        let now = Utc::now();
        let next = schedule.next_run_time(now);

        assert!(next.is_some());
    }

    #[test]
    fn test_schedule_weekly() {
        let schedule = Schedule::weekly(Weekday::Mon, 9, 0);
        let now = Utc::now();
        let next = schedule.next_run_time(now);

        assert!(next.is_some());
    }

    #[test]
    fn test_schedule_interval() {
        let schedule = Schedule::interval(3600); // 1 hour
        let now = Utc::now();
        let next = schedule.next_run_time(now).unwrap();

        let diff = next.signed_duration_since(now).num_seconds();
        assert_eq!(diff, 3600);
    }

    #[test]
    fn test_schedule_once() {
        let future = Utc::now() + chrono::TimeDelta::hours(1);
        let schedule = Schedule::once(future);

        let now = Utc::now();
        let next = schedule.next_run_time(now);

        assert!(next.is_some());
        assert!(next.unwrap() > now);
    }

    #[test]
    fn test_schedule_once_past() {
        let past = Utc::now() - chrono::TimeDelta::hours(1);
        let schedule = Schedule::once(past);

        let now = Utc::now();
        let next = schedule.next_run_time(now);

        assert!(next.is_none());
    }

    #[test]
    fn test_task_execution_tracking() {
        let schedule = Schedule::interval(60);
        let mut task = ScheduledTask::new("Test Task", schedule, "rule-1");

        assert_eq!(task.run_count, 0);
        assert!(task.last_run.is_none());

        task.record_run();
        assert_eq!(task.run_count, 1);
        assert!(task.last_run.is_some());

        task.record_run();
        assert_eq!(task.run_count, 2);
    }

    #[test]
    fn test_task_is_due() {
        let schedule = Schedule::interval(60);
        let mut task = ScheduledTask::new("Test Task", schedule, "rule-1");

        let now = Utc::now();
        task.next_run = Some(now - chrono::TimeDelta::seconds(1));

        assert!(task.is_due(now));

        task.next_run = Some(now + chrono::TimeDelta::seconds(1));
        assert!(!task.is_due(now));
    }

    #[test]
    fn test_disabled_task_not_due() {
        let schedule = Schedule::interval(60);
        let mut task = ScheduledTask::new("Test Task", schedule, "rule-1").disable();

        let now = Utc::now();
        task.next_run = Some(now - chrono::TimeDelta::seconds(1));

        assert!(!task.is_due(now));
    }

    #[test]
    fn test_schedule_manager() {
        let mut manager = ScheduleManager::new();

        let task1 = ScheduledTask::new("Task 1", Schedule::interval(60), "rule-1");
        let task2 = ScheduledTask::new("Task 2", Schedule::interval(120), "rule-2");

        let id1 = task1.id.clone();

        manager.add_task(task1);
        manager.add_task(task2);

        assert_eq!(manager.task_count(), 2);
        assert!(manager.get_task(&id1).is_some());

        assert!(manager.remove_task(&id1));
        assert_eq!(manager.task_count(), 1);
    }

    #[test]
    fn test_schedule_manager_due_tasks() {
        let mut manager = ScheduleManager::new();

        let now = Utc::now();

        let mut task1 = ScheduledTask::new("Task 1", Schedule::interval(60), "rule-1");
        task1.next_run = Some(now - chrono::TimeDelta::seconds(1)); // Due

        let mut task2 = ScheduledTask::new("Task 2", Schedule::interval(120), "rule-2");
        task2.next_run = Some(now + chrono::TimeDelta::seconds(60)); // Not due

        manager.add_task(task1);
        manager.add_task(task2);

        let due = manager.due_tasks(now);
        assert_eq!(due.len(), 1);
    }

    #[test]
    fn test_schedule_manager_enabled_tasks() {
        let mut manager = ScheduleManager::new();

        manager.add_task(ScheduledTask::new(
            "Enabled",
            Schedule::interval(60),
            "rule-1",
        ));
        manager
            .add_task(ScheduledTask::new("Disabled", Schedule::interval(60), "rule-2").disable());

        let enabled = manager.enabled_tasks();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_update_next_runs() {
        let mut manager = ScheduleManager::new();

        manager.add_task(ScheduledTask::new(
            "Task 1",
            Schedule::interval(60),
            "rule-1",
        ));

        let now = Utc::now();
        manager.update_next_runs(now);

        let tasks = manager.enabled_tasks();
        assert!(tasks[0].next_run.is_some());
    }

    #[test]
    fn test_schedule_monthly() {
        let schedule = Schedule::monthly(15, 10, 0);
        let now = Utc::now();
        let next = schedule.next_run_time(now);

        assert!(next.is_some());
    }
}
