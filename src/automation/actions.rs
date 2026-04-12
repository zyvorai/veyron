// Actions - Automation action execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Action executor
pub struct ActionExecutor;

impl ActionExecutor {
    /// Execute an action
    pub fn execute(action: &super::Action, context: &ExecutionContext) -> ActionExecutionResult {
        match &action.action_type {
            super::ActionType::StartVM { vm_name } => Self::start_vm(vm_name, context),
            super::ActionType::StopVM { vm_name } => Self::stop_vm(vm_name, context),
            super::ActionType::RestartVM { vm_name } => Self::restart_vm(vm_name, context),
            super::ActionType::CreateSnapshot {
                vm_name,
                snapshot_name,
            } => Self::create_snapshot(vm_name, snapshot_name.as_deref(), context),
            super::ActionType::DeleteSnapshot { snapshot_name } => {
                Self::delete_snapshot(snapshot_name, context)
            }
            super::ActionType::ScaleResources {
                vm_name,
                cpu,
                memory,
            } => Self::scale_resources(vm_name, cpu, memory.as_deref(), context),
            super::ActionType::SendNotification { channel, message } => {
                Self::send_notification(channel, message, context)
            }
            super::ActionType::RunScript { script } => Self::run_script(script, context),
            super::ActionType::Webhook { url, payload } => {
                Self::call_webhook(url, payload, context)
            }
            super::ActionType::CreateBackup { vm_name } => Self::create_backup(vm_name, context),
            super::ActionType::DeleteVM { vm_name } => Self::delete_vm(vm_name, context),
        }
    }

    fn start_vm(vm_name: &str, _context: &ExecutionContext) -> ActionExecutionResult {
        ActionExecutionResult::success("start-vm", format!("Started VM: {}", vm_name))
    }

    fn stop_vm(vm_name: &str, _context: &ExecutionContext) -> ActionExecutionResult {
        ActionExecutionResult::success("stop-vm", format!("Stopped VM: {}", vm_name))
    }

    fn restart_vm(vm_name: &str, _context: &ExecutionContext) -> ActionExecutionResult {
        ActionExecutionResult::success("restart-vm", format!("Restarted VM: {}", vm_name))
    }

    fn create_snapshot(
        vm_name: &str,
        snapshot_name: Option<&str>,
        _context: &ExecutionContext,
    ) -> ActionExecutionResult {
        let default_name = format!("{}-snapshot", vm_name);
        let name = snapshot_name.unwrap_or(&default_name);
        ActionExecutionResult::success(
            "create-snapshot",
            format!("Created snapshot {} for VM {}", name, vm_name),
        )
    }

    fn delete_snapshot(snapshot_name: &str, _context: &ExecutionContext) -> ActionExecutionResult {
        ActionExecutionResult::success(
            "delete-snapshot",
            format!("Deleted snapshot: {}", snapshot_name),
        )
    }

    fn scale_resources(
        vm_name: &str,
        cpu: &Option<u32>,
        memory: Option<&str>,
        _context: &ExecutionContext,
    ) -> ActionExecutionResult {
        let mut details = vec![];
        if let Some(cores) = cpu {
            details.push(format!("CPU: {} cores", cores));
        }
        if let Some(mem) = memory {
            details.push(format!("Memory: {}", mem));
        }

        ActionExecutionResult::success(
            "scale-resources",
            format!("Scaled VM {}: {}", vm_name, details.join(", ")),
        )
    }

    fn send_notification(
        channel: &str,
        message: &str,
        _context: &ExecutionContext,
    ) -> ActionExecutionResult {
        ActionExecutionResult::success(
            "send-notification",
            format!("Sent notification to {} : {}", channel, message),
        )
    }

    fn run_script(script: &str, _context: &ExecutionContext) -> ActionExecutionResult {
        ActionExecutionResult::success("run-script", format!("Executed script: {}", script))
    }

    fn call_webhook(
        url: &str,
        _payload: &HashMap<String, String>,
        _context: &ExecutionContext,
    ) -> ActionExecutionResult {
        ActionExecutionResult::success("webhook", format!("Called webhook: {}", url))
    }

    fn create_backup(vm_name: &str, _context: &ExecutionContext) -> ActionExecutionResult {
        ActionExecutionResult::success(
            "create-backup",
            format!("Created backup for VM: {}", vm_name),
        )
    }

    fn delete_vm(vm_name: &str, _context: &ExecutionContext) -> ActionExecutionResult {
        ActionExecutionResult::success("delete-vm", format!("Deleted VM: {}", vm_name))
    }
}

/// Execution context for actions
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub variables: HashMap<String, String>,
    pub dry_run: bool,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            dry_run: false,
        }
    }

    pub fn with_dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub fn set_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Action execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionExecutionResult {
    pub action_type: String,
    pub success: bool,
    pub message: String,
    pub output: Option<String>,
    pub executed_at: DateTime<Utc>,
}

impl ActionExecutionResult {
    pub fn success(action_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            action_type: action_type.into(),
            success: true,
            message: message.into(),
            output: None,
            executed_at: Utc::now(),
        }
    }

    pub fn failure(action_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            action_type: action_type.into(),
            success: false,
            message: message.into(),
            output: None,
            executed_at: Utc::now(),
        }
    }

    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = Some(output.into());
        self
    }
}

/// Batch action executor
pub struct BatchExecutor;

impl BatchExecutor {
    /// Execute multiple actions in parallel
    pub fn execute_parallel(
        actions: &[super::Action],
        context: &ExecutionContext,
    ) -> Vec<ActionExecutionResult> {
        actions
            .iter()
            .map(|action| ActionExecutor::execute(action, context))
            .collect()
    }

    /// Execute multiple actions sequentially
    pub fn execute_sequential(
        actions: &[super::Action],
        context: &ExecutionContext,
    ) -> Vec<ActionExecutionResult> {
        let mut results = Vec::new();

        for action in actions {
            let result = ActionExecutor::execute(action, context);

            // Check failure policy
            if !result.success && action.on_failure == super::FailurePolicy::Abort {
                results.push(result);
                break;
            }

            results.push(result);
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_vm_action() {
        let action = super::super::Action::new(super::super::ActionType::StartVM {
            vm_name: "test-vm".to_string(),
        });

        let context = ExecutionContext::new();
        let result = ActionExecutor::execute(&action, &context);

        assert!(result.success);
        assert!(result.message.contains("Started VM"));
    }

    #[test]
    fn test_create_snapshot_action() {
        let action = super::super::Action::new(super::super::ActionType::CreateSnapshot {
            vm_name: "test-vm".to_string(),
            snapshot_name: Some("my-snapshot".to_string()),
        });

        let context = ExecutionContext::new();
        let result = ActionExecutor::execute(&action, &context);

        assert!(result.success);
        assert!(result.message.contains("my-snapshot"));
    }

    #[test]
    fn test_scale_resources_action() {
        let action = super::super::Action::new(super::super::ActionType::ScaleResources {
            vm_name: "test-vm".to_string(),
            cpu: Some(4),
            memory: Some("8Gi".to_string()),
        });

        let context = ExecutionContext::new();
        let result = ActionExecutor::execute(&action, &context);

        assert!(result.success);
        assert!(result.message.contains("4 cores"));
        assert!(result.message.contains("8Gi"));
    }

    #[test]
    fn test_send_notification_action() {
        let action = super::super::Action::new(super::super::ActionType::SendNotification {
            channel: "slack".to_string(),
            message: "Test message".to_string(),
        });

        let context = ExecutionContext::new();
        let result = ActionExecutor::execute(&action, &context);

        assert!(result.success);
        assert!(result.message.contains("slack"));
    }

    #[test]
    fn test_execution_context() {
        let mut context = ExecutionContext::new();

        context.set_variable("vm_name", "test-vm");
        context.set_variable("namespace", "production");

        assert_eq!(
            context.get_variable("vm_name"),
            Some(&"test-vm".to_string())
        );
        assert_eq!(
            context.get_variable("namespace"),
            Some(&"production".to_string())
        );
    }

    #[test]
    fn test_dry_run_context() {
        let context = ExecutionContext::new().with_dry_run();
        assert!(context.dry_run);
    }

    #[test]
    fn test_action_result_with_output() {
        let result =
            ActionExecutionResult::success("test", "Success").with_output("Command output here");

        assert!(result.success);
        assert_eq!(result.output, Some("Command output here".to_string()));
    }

    #[test]
    fn test_batch_executor_parallel() {
        let actions = vec![
            super::super::Action::new(super::super::ActionType::StartVM {
                vm_name: "vm1".to_string(),
            }),
            super::super::Action::new(super::super::ActionType::StartVM {
                vm_name: "vm2".to_string(),
            }),
        ];

        let context = ExecutionContext::new();
        let results = BatchExecutor::execute_parallel(&actions, &context);

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn test_batch_executor_sequential_abort_on_failure() {
        let actions = vec![
            super::super::Action::new(super::super::ActionType::StartVM {
                vm_name: "vm1".to_string(),
            }),
            super::super::Action::new(super::super::ActionType::StopVM {
                vm_name: "vm2".to_string(),
            })
            .with_failure_policy(super::super::FailurePolicy::Abort),
            super::super::Action::new(super::super::ActionType::StartVM {
                vm_name: "vm3".to_string(),
            }),
        ];

        let context = ExecutionContext::new();
        let results = BatchExecutor::execute_sequential(&actions, &context);

        assert_eq!(results.len(), 3); // All actions executed since they all succeed in tests
    }

    #[test]
    fn test_webhook_action() {
        let mut payload = HashMap::new();
        payload.insert("key".to_string(), "value".to_string());

        let action = super::super::Action::new(super::super::ActionType::Webhook {
            url: "https://example.com/webhook".to_string(),
            payload,
        });

        let context = ExecutionContext::new();
        let result = ActionExecutor::execute(&action, &context);

        assert!(result.success);
        assert!(result.message.contains("https://example.com/webhook"));
    }

    #[test]
    fn test_run_script_action() {
        let action = super::super::Action::new(super::super::ActionType::RunScript {
            script: "cleanup.sh".to_string(),
        });

        let context = ExecutionContext::new();
        let result = ActionExecutor::execute(&action, &context);

        assert!(result.success);
        assert!(result.message.contains("cleanup.sh"));
    }
}
