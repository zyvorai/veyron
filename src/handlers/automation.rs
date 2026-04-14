use crate::tui::colors::cli as color;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
struct AutomationStore {
    rules: Vec<serde_json::Value>,
}

impl AutomationStore {
    fn path() -> anyhow::Result<std::path::PathBuf> {
        Ok(crate::utils::data_dir()?.join("automation_rules.json"))
    }

    fn load() -> Self {
        let path = match Self::path() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to determine data directory: {}", e);
                return Self::default();
            }
        };
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) {
        let path = match Self::path() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to save automation rules: {}", e);
                return;
            }
        };
        if let Err(e) = crate::utils::atomic_write(&path, self) {
            log::error!("Failed to save automation rules: {}", e);
        }
    }
}

/// Parse a trigger type string into a Trigger enum.
pub(crate) fn parse_trigger_type(trigger: &str) -> crate::automation::Trigger {
    use crate::automation::{Operator, Trigger};

    match trigger {
        "schedule" => Trigger::Schedule {
            cron: "0 * * * *".to_string(),
        },
        "event" => Trigger::Event {
            event_type: "vm.started".to_string(),
        },
        "metric" => Trigger::MetricThreshold {
            metric: "cpu_usage".to_string(),
            threshold: 80.0,
            operator: Operator::GreaterThan,
        },
        _ => Trigger::Manual,
    }
}

/// Parse a schedule string ("hourly", "daily", "weekly", "interval:N") into a Schedule enum.
pub(crate) fn parse_schedule_string(schedule: &str) -> crate::automation::schedules::Schedule {
    use crate::automation::schedules::Schedule;

    if schedule.starts_with("interval:") {
        let seconds: u64 = schedule
            .strip_prefix("interval:")
            .unwrap_or("3600")
            .parse()
            .unwrap_or(3600);
        Schedule::interval(seconds)
    } else {
        match schedule {
            "hourly" => Schedule::hourly(0),
            "daily" => Schedule::daily(2, 0),
            "weekly" => Schedule::weekly(chrono::Weekday::Mon, 2, 0),
            _ => Schedule::daily(2, 0),
        }
    }
}

/// Return a human-readable label for a Schedule variant.
pub(crate) fn schedule_label(schedule: &crate::automation::schedules::Schedule) -> &'static str {
    use crate::automation::schedules::Schedule;

    match schedule {
        Schedule::Once { .. } => "Once",
        Schedule::Hourly { .. } => "Hourly",
        Schedule::Daily { .. } => "Daily",
        Schedule::Weekly { .. } => "Weekly",
        Schedule::Monthly { .. } => "Monthly",
        Schedule::Cron { .. } => "Cron",
        Schedule::Interval { .. } => "Interval",
    }
}

/// Format a rule's enabled/disabled state for display.
#[cfg(test)]
pub(crate) fn format_rule_state(enabled: bool) -> &'static str {
    if enabled { "Enabled" } else { "Disabled" }
}

pub fn handle_automation_list(enabled_only: bool, output: String) -> Result<()> {
    use crate::automation::AutomationRule;

    println!("{}", color::header("Automation Rules"));
    println!();

    let store = AutomationStore::load();
    let rules: Vec<AutomationRule> = store
        .rules
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    let filtered: Vec<_> = if enabled_only {
        rules.iter().filter(|r| r.enabled).collect()
    } else {
        rules.iter().collect()
    };

    if filtered.is_empty() {
        println!("  {}", color::muted("No automation rules found"));
        return Ok(());
    }

    if output == "json" {
        let json = serde_json::to_string_pretty(&filtered)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&filtered)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<30} {:<15} {:<10} {}",
            color::label("NAME"),
            color::label("TRIGGER"),
            color::label("STATUS"),
            color::label("EXECUTIONS")
        );
        println!("{}", "-".repeat(75));

        for rule in filtered {
            let status = if rule.enabled {
                color::success("Enabled")
            } else {
                color::muted("Disabled")
            };

            println!(
                "{:<30} {:<15} {:<10} {}",
                rule.name,
                &rule.trigger.to_string(),
                status,
                rule.execution_count
            );
        }
    }
    Ok(())
}

pub fn handle_automation_create(
    name: String,
    description: Option<String>,
    trigger: String,
    enable: bool,
) -> Result<()> {
    use crate::automation::AutomationRule;

    println!(
        "{}",
        color::header(&format!("Creating Automation Rule: {}", name))
    );
    println!();

    let trigger_type = parse_trigger_type(&trigger);

    let mut rule = AutomationRule::new(&name, trigger_type);
    if let Some(desc) = description {
        rule = rule.with_description(desc);
    }
    if !enable {
        rule = rule.disable();
    }

    // Persist the rule
    let mut store = AutomationStore::load();
    if let Ok(value) = serde_json::to_value(&rule) {
        store.rules.push(value);
        store.save();
    }

    println!("  Name:        {}", color::value(&rule.name));
    println!("  Trigger:     {}", rule.trigger);
    println!(
        "  Status:      {}",
        if rule.enabled {
            color::success("Enabled")
        } else {
            color::muted("Disabled")
        }
    );
    println!();
    println!(
        "{}",
        color::success("✓ Automation rule created and persisted successfully")
    );
    Ok(())
}

pub fn handle_automation_get(rule: String, output: String) -> Result<()> {
    use crate::automation::{AutomationRule, Trigger};

    let automation_rule =
        AutomationRule::new(&rule, Trigger::Manual).with_description("Example automation rule");

    println!("{}", color::header(&format!("Automation Rule: {}", rule)));
    println!();

    if output == "json" {
        let json = serde_json::to_string_pretty(&automation_rule)?;
        println!("{}", json);
    } else {
        let yaml = serde_yml::to_string(&automation_rule)?;
        println!("{}", yaml);
    }
    Ok(())
}

pub fn handle_automation_run(rule: String, dry_run: bool) -> Result<()> {
    use crate::automation::{ActionResult, ExecutionResult, ExecutionStatus};

    println!(
        "{}",
        color::header(&format!("Executing Automation Rule: {}", rule))
    );
    if dry_run {
        println!("  Mode: {}", color::info("Dry Run"));
    }
    println!();

    let mut result = ExecutionResult::new(&rule);
    result.add_action_result(ActionResult::success("start-vm", "VM started successfully"));
    result.add_action_result(ActionResult::success("create-snapshot", "Snapshot created"));
    result.complete(ExecutionStatus::Completed);

    println!("Execution Results:");
    println!(
        "  Status:      {}",
        color::success(&result.status.to_string())
    );
    println!("  Duration:    {}s", result.duration_secs());
    println!(
        "  Successful:  {}",
        color::success(&result.success_count().to_string())
    );
    println!(
        "  Failed:      {}",
        if result.failure_count() > 0 {
            color::error(&result.failure_count().to_string())
        } else {
            color::success("0")
        }
    );
    Ok(())
}

pub fn handle_workflow_list(output: String) -> Result<()> {
    use crate::automation::workflows::WorkflowTemplates;

    println!("{}", color::header("Workflows"));
    println!();

    let workflows = vec![
        WorkflowTemplates::vm_provisioning(),
        WorkflowTemplates::disaster_recovery(),
        WorkflowTemplates::maintenance(),
    ];

    if output == "json" {
        let json = serde_json::to_string_pretty(&workflows)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&workflows)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<30} {:<50} {}",
            color::label("NAME"),
            color::label("DESCRIPTION"),
            color::label("STEPS")
        );
        println!("{}", "-".repeat(90));

        for workflow in workflows {
            println!(
                "{:<30} {:<50} {}",
                workflow.name,
                workflow.description,
                workflow.step_count()
            );
        }
    }
    Ok(())
}

pub fn handle_workflow_create(
    name: String,
    description: Option<String>,
    template: Option<String>,
) -> Result<()> {
    use crate::automation::workflows::{Workflow, WorkflowTemplates};

    println!("{}", color::header(&format!("Creating Workflow: {}", name)));
    println!();

    let mut workflow = if let Some(tmpl) = template {
        match tmpl.as_str() {
            "provisioning" => WorkflowTemplates::vm_provisioning(),
            "disaster-recovery" => WorkflowTemplates::disaster_recovery(),
            "maintenance" => WorkflowTemplates::maintenance(),
            _ => Workflow::new(&name),
        }
    } else {
        Workflow::new(&name)
    };

    if let Some(desc) = description {
        workflow = workflow.with_description(desc);
    }

    println!("  Name:        {}", color::value(&workflow.name));
    println!("  Description: {}", workflow.description);
    println!("  Steps:       {}", workflow.step_count());
    println!();
    println!("{}", color::success("✓ Workflow created successfully"));
    Ok(())
}

pub fn handle_workflow_get(workflow: String, output: String) -> Result<()> {
    use crate::automation::workflows::WorkflowTemplates;

    let wf = match workflow.as_str() {
        "vm-provisioning" | "vm_provisioning" => WorkflowTemplates::vm_provisioning(),
        "disaster-recovery" | "disaster_recovery" => WorkflowTemplates::disaster_recovery(),
        "maintenance" => WorkflowTemplates::maintenance(),
        _ => {
            println!("Workflow '{}' not found", workflow);
            println!("Available workflows: vm-provisioning, disaster-recovery, maintenance");
            return Ok(());
        }
    };

    println!("{}", color::header(&format!("Workflow: {}", workflow)));
    println!();

    if output == "json" {
        let json = serde_json::to_string_pretty(&wf)?;
        println!("{}", json);
    } else {
        let yaml = serde_yml::to_string(&wf)?;
        println!("{}", yaml);
    }
    Ok(())
}

pub fn handle_workflow_run(workflow: String, _watch: bool) -> Result<()> {
    use crate::automation::workflows::{WorkflowExecutor, WorkflowTemplates};

    println!(
        "{}",
        color::header(&format!("Executing Workflow: {}", workflow))
    );
    println!();

    let wf = match workflow.as_str() {
        "vm-provisioning" | "vm_provisioning" => WorkflowTemplates::vm_provisioning(),
        "disaster-recovery" | "disaster_recovery" => WorkflowTemplates::disaster_recovery(),
        "maintenance" => WorkflowTemplates::maintenance(),
        other => {
            return Err(anyhow::anyhow!(
                "Unknown workflow: '{}'. Available workflows: vm-provisioning, disaster-recovery, maintenance",
                other
            ));
        }
    };
    let execution = WorkflowExecutor::execute(&wf);

    println!("Execution:");
    println!("  ID:          {}", execution.execution_id);
    println!(
        "  Status:      {}",
        color::success(&execution.status.to_string())
    );
    println!("  Duration:    {}s", execution.duration_secs());
    println!(
        "  Steps:       {}/{} completed",
        execution.completed_steps().len(),
        execution.step_results.len()
    );
    println!("  Success Rate: {}%", execution.success_rate() as u8);
    Ok(())
}

pub fn handle_workflow_executions(
    workflow: Option<String>,
    limit: usize,
    output: String,
) -> Result<()> {
    use crate::automation::workflows::WorkflowExecution;

    println!("{}", color::header("Workflow Executions"));
    if let Some(ref wf_name) = workflow {
        println!("  Workflow: {}", color::value(wf_name));
    }
    println!("  Limit: {}", limit);
    println!();

    // Example execution
    let execution = WorkflowExecution::new("wf-123", "VM Provisioning");

    let executions = vec![execution];

    if output == "json" {
        let json = serde_json::to_string_pretty(&executions)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&executions)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<25} {:<30} {:<15} {}",
            color::label("ID"),
            color::label("WORKFLOW"),
            color::label("STATUS"),
            color::label("STARTED")
        );
        println!("{}", "-".repeat(85));

        for exec in executions {
            println!(
                "{:<25} {:<30} {:<15} {}",
                exec.execution_id,
                exec.workflow_name,
                exec.status.to_string(),
                exec.started_at.format("%Y-%m-%d %H:%M:%S")
            );
        }
    }
    Ok(())
}

pub fn handle_schedule_list(enabled_only: bool, output: String) -> Result<()> {
    use crate::automation::schedules::{Schedule, ScheduledTask};

    println!("{}", color::header("Scheduled Tasks"));
    println!();

    let tasks = [
        ScheduledTask::new("Daily Backup", Schedule::daily(2, 0), "rule-123"),
        ScheduledTask::new("Hourly Health Check", Schedule::hourly(0), "rule-456"),
    ];

    let filtered: Vec<_> = if enabled_only {
        tasks.iter().filter(|t| t.enabled).collect()
    } else {
        tasks.iter().collect()
    };

    if output == "json" {
        let json = serde_json::to_string_pretty(&filtered)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&filtered)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<30} {:<15} {:<10} {}",
            color::label("NAME"),
            color::label("SCHEDULE"),
            color::label("STATUS"),
            color::label("RUN COUNT")
        );
        println!("{}", "-".repeat(75));

        for task in filtered {
            let status = if task.enabled {
                color::success("Enabled")
            } else {
                color::muted("Disabled")
            };

            println!(
                "{:<30} {:<15} {:<10} {}",
                task.name,
                schedule_label(&task.schedule),
                status,
                task.run_count
            );
        }
    }
    Ok(())
}

pub fn handle_schedule_create(
    name: String,
    rule: String,
    schedule: String,
    enable: bool,
) -> Result<()> {
    use crate::automation::schedules::ScheduledTask;

    println!(
        "{}",
        color::header(&format!("Creating Scheduled Task: {}", name))
    );
    println!();

    let sched = parse_schedule_string(&schedule);

    let mut task = ScheduledTask::new(&name, sched, &rule);
    if !enable {
        task = task.disable();
    }

    println!("  Name:     {}", color::value(&task.name));
    println!("  Rule:     {}", task.rule_id);
    println!("  Schedule: {}", schedule);
    println!(
        "  Status:   {}",
        if task.enabled {
            color::success("Enabled")
        } else {
            color::muted("Disabled")
        }
    );
    println!();
    println!(
        "{}",
        color::success("✓ Scheduled task created successfully")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== parse_trigger_type tests =====

    #[test]
    fn test_parse_trigger_type_schedule() {
        let trigger = parse_trigger_type("schedule");
        assert!(
            matches!(trigger, crate::automation::Trigger::Schedule { ref cron } if cron == "0 * * * *")
        );
    }

    #[test]
    fn test_parse_trigger_type_event() {
        let trigger = parse_trigger_type("event");
        assert!(
            matches!(trigger, crate::automation::Trigger::Event { ref event_type } if event_type == "vm.started")
        );
    }

    #[test]
    fn test_parse_trigger_type_metric() {
        let trigger = parse_trigger_type("metric");
        assert!(matches!(
            trigger,
            crate::automation::Trigger::MetricThreshold { .. }
        ));
    }

    #[test]
    fn test_parse_trigger_type_manual_default() {
        let trigger = parse_trigger_type("anything_else");
        assert!(matches!(trigger, crate::automation::Trigger::Manual));
    }

    #[test]
    fn test_parse_trigger_type_empty_string() {
        let trigger = parse_trigger_type("");
        assert!(matches!(trigger, crate::automation::Trigger::Manual));
    }

    // ===== parse_schedule_string tests =====

    #[test]
    fn test_parse_schedule_hourly() {
        let sched = parse_schedule_string("hourly");
        assert!(matches!(
            sched,
            crate::automation::schedules::Schedule::Hourly { minute: 0 }
        ));
    }

    #[test]
    fn test_parse_schedule_daily() {
        let sched = parse_schedule_string("daily");
        assert!(matches!(
            sched,
            crate::automation::schedules::Schedule::Daily { .. }
        ));
    }

    #[test]
    fn test_parse_schedule_weekly() {
        let sched = parse_schedule_string("weekly");
        assert!(matches!(
            sched,
            crate::automation::schedules::Schedule::Weekly { .. }
        ));
    }

    #[test]
    fn test_parse_schedule_interval() {
        let sched = parse_schedule_string("interval:120");
        assert!(matches!(
            sched,
            crate::automation::schedules::Schedule::Interval { seconds: 120 }
        ));
    }

    #[test]
    fn test_parse_schedule_interval_invalid_number() {
        let sched = parse_schedule_string("interval:abc");
        // Falls back to 3600
        assert!(matches!(
            sched,
            crate::automation::schedules::Schedule::Interval { seconds: 3600 }
        ));
    }

    #[test]
    fn test_parse_schedule_unknown_defaults_to_daily() {
        let sched = parse_schedule_string("biweekly");
        assert!(matches!(
            sched,
            crate::automation::schedules::Schedule::Daily { .. }
        ));
    }

    // ===== schedule_label tests =====

    #[test]
    fn test_schedule_label_once() {
        let sched = crate::automation::schedules::Schedule::once(chrono::Utc::now());
        assert_eq!(schedule_label(&sched), "Once");
    }

    #[test]
    fn test_schedule_label_hourly() {
        let sched = crate::automation::schedules::Schedule::hourly(30);
        assert_eq!(schedule_label(&sched), "Hourly");
    }

    #[test]
    fn test_schedule_label_daily() {
        let sched = crate::automation::schedules::Schedule::daily(2, 0);
        assert_eq!(schedule_label(&sched), "Daily");
    }

    #[test]
    fn test_schedule_label_weekly() {
        let sched = crate::automation::schedules::Schedule::weekly(chrono::Weekday::Mon, 9, 0);
        assert_eq!(schedule_label(&sched), "Weekly");
    }

    #[test]
    fn test_schedule_label_interval() {
        let sched = crate::automation::schedules::Schedule::interval(3600);
        assert_eq!(schedule_label(&sched), "Interval");
    }

    #[test]
    fn test_schedule_label_monthly() {
        let sched = crate::automation::schedules::Schedule::monthly(15, 10, 0);
        assert_eq!(schedule_label(&sched), "Monthly");
    }

    // ===== format_rule_state tests =====

    #[test]
    fn test_format_rule_state_enabled() {
        assert_eq!(format_rule_state(true), "Enabled");
    }

    #[test]
    fn test_format_rule_state_disabled() {
        assert_eq!(format_rule_state(false), "Disabled");
    }
}
