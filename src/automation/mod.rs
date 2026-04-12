// Automation & Orchestration - Workflow engine and event-driven automation

use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod actions;
pub mod schedules;
pub mod triggers;
pub mod workflows;

/// Automation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub trigger: Trigger,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
    pub created_at: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
    pub execution_count: u64,
}

impl AutomationRule {
    pub fn new(name: impl Into<String>, trigger: Trigger) -> Self {
        let name_str = name.into();
        let id = format!(
            "rule-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            description: String::new(),
            enabled: true,
            trigger,
            conditions: Vec::new(),
            actions: Vec::new(),
            created_at: Utc::now(),
            last_triggered: None,
            execution_count: 0,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn add_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn add_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Check if all conditions are met
    pub fn evaluate_conditions(&self, context: &AutomationContext) -> bool {
        if self.conditions.is_empty() {
            return true;
        }

        self.conditions
            .iter()
            .all(|condition| condition.evaluate(context))
    }

    pub fn record_execution(&mut self) {
        self.last_triggered = Some(Utc::now());
        self.execution_count += 1;
    }
}

/// Automation trigger
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Trigger {
    VMStateChange {
        state: String,
    }, // VM state changed to specific state
    MetricThreshold {
        metric: String,
        threshold: f64,
        operator: Operator,
    },
    Schedule {
        cron: String,
    }, // Cron-based schedule
    Event {
        event_type: String,
    }, // Custom event
    Manual, // Manually triggered
    Webhook {
        url: String,
    }, // Webhook trigger
}

impl std::fmt::Display for Trigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Trigger::VMStateChange { state } => write!(f, "VMStateChange({})", state),
            Trigger::MetricThreshold {
                metric, threshold, ..
            } => write!(f, "MetricThreshold({} @ {})", metric, threshold),
            Trigger::Schedule { cron } => write!(f, "Schedule({})", cron),
            Trigger::Event { event_type } => write!(f, "Event({})", event_type),
            Trigger::Manual => write!(f, "Manual"),
            Trigger::Webhook { url } => write!(f, "Webhook({})", url),
        }
    }
}

/// Comparison operator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Operator {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Automation condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub condition_type: ConditionType,
    pub negate: bool, // If true, condition is negated
}

impl Condition {
    pub fn new(condition_type: ConditionType) -> Self {
        Self {
            condition_type,
            negate: false,
        }
    }

    pub fn negate(mut self) -> Self {
        self.negate = true;
        self
    }

    pub fn evaluate(&self, context: &AutomationContext) -> bool {
        let result = match &self.condition_type {
            ConditionType::VMExists { vm_name } => context.vm_exists(vm_name),
            ConditionType::VMState { vm_name, state } => context.vm_state_matches(vm_name, state),
            ConditionType::TimeRange {
                start_hour,
                end_hour,
            } => {
                let current_hour = Utc::now().hour();
                if start_hour <= end_hour {
                    current_hour >= *start_hour && current_hour < *end_hour
                } else {
                    // Midnight-crossing range (e.g., 22:00 to 06:00)
                    current_hour >= *start_hour || current_hour < *end_hour
                }
            }
            ConditionType::DayOfWeek { days } => {
                let current_day = Utc::now().weekday().number_from_monday();
                days.contains(&current_day)
            }
            ConditionType::Label { key, value } => context.has_label(key, value),
            ConditionType::Namespace { namespace } => context.namespace_matches(namespace),
        };

        if self.negate {
            !result
        } else {
            result
        }
    }
}

/// Condition type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    VMExists { vm_name: String },
    VMState { vm_name: String, state: String },
    TimeRange { start_hour: u32, end_hour: u32 },
    DayOfWeek { days: Vec<u32> }, // 1=Monday, 7=Sunday
    Label { key: String, value: String },
    Namespace { namespace: String },
}

/// Automation action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: ActionType,
    pub on_failure: FailurePolicy,
}

impl Action {
    pub fn new(action_type: ActionType) -> Self {
        Self {
            action_type,
            on_failure: FailurePolicy::Continue,
        }
    }

    pub fn with_failure_policy(mut self, policy: FailurePolicy) -> Self {
        self.on_failure = policy;
        self
    }
}

/// Action type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    StartVM {
        vm_name: String,
    },
    StopVM {
        vm_name: String,
    },
    RestartVM {
        vm_name: String,
    },
    CreateSnapshot {
        vm_name: String,
        snapshot_name: Option<String>,
    },
    DeleteSnapshot {
        snapshot_name: String,
    },
    ScaleResources {
        vm_name: String,
        cpu: Option<u32>,
        memory: Option<String>,
    },
    SendNotification {
        channel: String,
        message: String,
    },
    RunScript {
        script: String,
    },
    Webhook {
        url: String,
        payload: HashMap<String, String>,
    },
    CreateBackup {
        vm_name: String,
    },
    DeleteVM {
        vm_name: String,
    },
}

/// Failure policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FailurePolicy {
    Continue, // Continue with next action
    Abort,    // Stop execution
    Retry { max_attempts: u32 },
}

/// Automation context
#[derive(Debug, Clone)]
pub struct AutomationContext {
    pub vm_name: Option<String>,
    pub namespace: String,
    pub labels: HashMap<String, String>,
    pub vm_state: Option<String>,
    pub metrics: HashMap<String, f64>,
    pub event_data: HashMap<String, String>,
}

impl AutomationContext {
    pub fn new() -> Self {
        Self {
            vm_name: None,
            namespace: "default".to_string(),
            labels: HashMap::new(),
            vm_state: None,
            metrics: HashMap::new(),
            event_data: HashMap::new(),
        }
    }

    pub fn with_vm(mut self, vm_name: impl Into<String>) -> Self {
        self.vm_name = Some(vm_name.into());
        self
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.vm_state = Some(state.into());
        self
    }

    pub fn add_label(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.labels.insert(key.into(), value.into());
    }

    pub fn add_metric(&mut self, key: impl Into<String>, value: f64) {
        self.metrics.insert(key.into(), value);
    }

    fn vm_exists(&self, vm_name: &str) -> bool {
        self.vm_name.as_deref() == Some(vm_name)
    }

    fn vm_state_matches(&self, vm_name: &str, state: &str) -> bool {
        self.vm_name.as_deref() == Some(vm_name) && self.vm_state.as_deref() == Some(state)
    }

    fn has_label(&self, key: &str, value: &str) -> bool {
        self.labels.get(key).map(|v| v == value).unwrap_or(false)
    }

    fn namespace_matches(&self, namespace: &str) -> bool {
        self.namespace == namespace
    }
}

impl Default for AutomationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Automation execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub rule_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
    pub action_results: Vec<ActionResult>,
    pub error_message: Option<String>,
}

impl ExecutionResult {
    pub fn new(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            started_at: Utc::now(),
            completed_at: None,
            status: ExecutionStatus::Running,
            action_results: Vec::new(),
            error_message: None,
        }
    }

    pub fn add_action_result(&mut self, result: ActionResult) {
        self.action_results.push(result);
    }

    pub fn complete(&mut self, status: ExecutionStatus) {
        self.completed_at = Some(Utc::now());
        self.status = status;
    }

    pub fn duration_secs(&self) -> i64 {
        match self.completed_at {
            Some(completed) => completed
                .signed_duration_since(self.started_at)
                .num_seconds(),
            None => Utc::now()
                .signed_duration_since(self.started_at)
                .num_seconds(),
        }
    }

    pub fn success_count(&self) -> usize {
        self.action_results.iter().filter(|r| r.success).count()
    }

    pub fn failure_count(&self) -> usize {
        self.action_results.iter().filter(|r| !r.success).count()
    }
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    PartialSuccess,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Running => write!(f, "Running"),
            ExecutionStatus::Completed => write!(f, "Completed"),
            ExecutionStatus::Failed => write!(f, "Failed"),
            ExecutionStatus::PartialSuccess => write!(f, "Partial Success"),
        }
    }
}

/// Action execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_name: String,
    pub success: bool,
    pub message: String,
    pub executed_at: DateTime<Utc>,
}

impl ActionResult {
    pub fn success(action_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            action_name: action_name.into(),
            success: true,
            message: message.into(),
            executed_at: Utc::now(),
        }
    }

    pub fn failure(action_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            action_name: action_name.into(),
            success: false,
            message: message.into(),
            executed_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automation_rule() {
        let rule = AutomationRule::new("Auto Stop Idle VMs", Trigger::Manual)
            .with_description("Stop VMs with low CPU usage")
            .add_condition(Condition::new(ConditionType::TimeRange {
                start_hour: 18,
                end_hour: 8,
            }))
            .add_action(Action::new(ActionType::StopVM {
                vm_name: "test-vm".to_string(),
            }));

        assert_eq!(rule.name, "Auto Stop Idle VMs");
        assert!(rule.enabled);
        assert_eq!(rule.conditions.len(), 1);
        assert_eq!(rule.actions.len(), 1);
    }

    #[test]
    fn test_trigger_types() {
        let state_trigger = Trigger::VMStateChange {
            state: "Running".to_string(),
        };
        assert!(matches!(state_trigger, Trigger::VMStateChange { .. }));

        let metric_trigger = Trigger::MetricThreshold {
            metric: "cpu".to_string(),
            threshold: 80.0,
            operator: Operator::GreaterThan,
        };
        assert!(matches!(metric_trigger, Trigger::MetricThreshold { .. }));
    }

    #[test]
    fn test_condition_evaluation() {
        let context = AutomationContext::new()
            .with_vm("test-vm")
            .with_namespace("default");

        let condition = Condition::new(ConditionType::VMExists {
            vm_name: "test-vm".to_string(),
        });

        assert!(condition.evaluate(&context));

        let negated = condition.negate();
        assert!(!negated.evaluate(&context));
    }

    #[test]
    fn test_time_range_condition() {
        let condition = Condition::new(ConditionType::TimeRange {
            start_hour: 0,
            end_hour: 24,
        });

        let context = AutomationContext::new();
        assert!(condition.evaluate(&context)); // Should always be true for 0-24 range
    }

    #[test]
    fn test_label_condition() {
        let mut context = AutomationContext::new();
        context.add_label("env", "production");

        let condition = Condition::new(ConditionType::Label {
            key: "env".to_string(),
            value: "production".to_string(),
        });

        assert!(condition.evaluate(&context));

        let wrong_condition = Condition::new(ConditionType::Label {
            key: "env".to_string(),
            value: "development".to_string(),
        });

        assert!(!wrong_condition.evaluate(&context));
    }

    #[test]
    fn test_action_types() {
        let start_action = Action::new(ActionType::StartVM {
            vm_name: "test-vm".to_string(),
        });
        assert_eq!(start_action.on_failure, FailurePolicy::Continue);

        let action_with_retry = Action::new(ActionType::RestartVM {
            vm_name: "test-vm".to_string(),
        })
        .with_failure_policy(FailurePolicy::Retry { max_attempts: 3 });

        assert!(matches!(
            action_with_retry.on_failure,
            FailurePolicy::Retry { .. }
        ));
    }

    #[test]
    fn test_execution_result() {
        let mut result = ExecutionResult::new("rule-123");

        result.add_action_result(ActionResult::success("start-vm", "VM started successfully"));
        result.add_action_result(ActionResult::failure("stop-vm", "VM not found"));

        assert_eq!(result.success_count(), 1);
        assert_eq!(result.failure_count(), 1);

        result.complete(ExecutionStatus::PartialSuccess);
        assert_eq!(result.status, ExecutionStatus::PartialSuccess);
        assert!(result.completed_at.is_some());
    }

    #[test]
    fn test_automation_context() {
        let mut context = AutomationContext::new()
            .with_vm("test-vm")
            .with_namespace("production")
            .with_state("Running");

        context.add_label("tier", "frontend");
        context.add_metric("cpu_usage", 75.5);

        assert_eq!(context.vm_name, Some("test-vm".to_string()));
        assert_eq!(context.namespace, "production");
        assert_eq!(context.vm_state, Some("Running".to_string()));
        assert_eq!(context.labels.get("tier"), Some(&"frontend".to_string()));
        assert_eq!(context.metrics.get("cpu_usage"), Some(&75.5));
    }

    #[test]
    fn test_rule_execution_tracking() {
        let mut rule = AutomationRule::new("Test Rule", Trigger::Manual);

        assert_eq!(rule.execution_count, 0);
        assert!(rule.last_triggered.is_none());

        rule.record_execution();
        assert_eq!(rule.execution_count, 1);
        assert!(rule.last_triggered.is_some());

        rule.record_execution();
        assert_eq!(rule.execution_count, 2);
    }

    #[test]
    fn test_execution_duration() {
        let mut result = ExecutionResult::new("rule-123");
        std::thread::sleep(std::time::Duration::from_millis(10));
        result.complete(ExecutionStatus::Completed);

        assert!(result.duration_secs() >= 0);
    }

    #[test]
    fn test_operator_types() {
        assert!(matches!(Operator::GreaterThan, Operator::GreaterThan));
        assert!(matches!(Operator::LessThan, Operator::LessThan));
        assert!(matches!(Operator::Equal, Operator::Equal));
    }
}
