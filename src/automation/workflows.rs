// Workflows - Multi-step automation workflows

use super::{Action, ExecutionStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Workflow {
    pub fn new(name: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "wf-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            description: String::new(),
            steps: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self.updated_at = Utc::now();
        self
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

/// Workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_number: u32,
    pub name: String,
    pub action: Action,
    pub dependencies: Vec<u32>, // Step numbers that must complete first
    pub timeout_seconds: Option<u64>,
    pub continue_on_failure: bool,
}

impl WorkflowStep {
    pub fn new(step_number: u32, name: impl Into<String>, action: Action) -> Self {
        Self {
            step_number,
            name: name.into(),
            action,
            dependencies: Vec::new(),
            timeout_seconds: None,
            continue_on_failure: false,
        }
    }

    pub fn depends_on(mut self, step_number: u32) -> Self {
        self.dependencies.push(step_number);
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    pub fn continue_on_failure(mut self) -> Self {
        self.continue_on_failure = true;
        self
    }

    pub fn can_execute(&self, completed_steps: &[u32]) -> bool {
        self.dependencies
            .iter()
            .all(|dep| completed_steps.contains(dep))
    }
}

/// Workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub execution_id: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
    pub step_results: Vec<StepResult>,
    pub current_step: Option<u32>,
}

impl WorkflowExecution {
    pub fn new(workflow_id: impl Into<String>, workflow_name: impl Into<String>) -> Self {
        let execution_id = format!("exec-{}", Utc::now().format("%Y%m%d-%H%M%S-%f"));

        Self {
            execution_id,
            workflow_id: workflow_id.into(),
            workflow_name: workflow_name.into(),
            started_at: Utc::now(),
            completed_at: None,
            status: ExecutionStatus::Running,
            step_results: Vec::new(),
            current_step: None,
        }
    }

    pub fn add_step_result(&mut self, result: StepResult) {
        self.step_results.push(result);
    }

    pub fn complete(&mut self, status: ExecutionStatus) {
        self.completed_at = Some(Utc::now());
        self.status = status;
        self.current_step = None;
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

    pub fn completed_steps(&self) -> Vec<u32> {
        self.step_results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.step_number)
            .collect()
    }

    pub fn failed_steps(&self) -> Vec<&StepResult> {
        self.step_results.iter().filter(|r| !r.success).collect()
    }

    pub fn success_rate(&self) -> f64 {
        if self.step_results.is_empty() {
            return 0.0;
        }

        let successful = self.step_results.iter().filter(|r| r.success).count();

        (successful as f64 / self.step_results.len() as f64) * 100.0
    }
}

/// Step execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_number: u32,
    pub step_name: String,
    pub success: bool,
    pub message: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_secs: i64,
}

impl StepResult {
    pub fn new(
        step_number: u32,
        step_name: impl Into<String>,
        success: bool,
        message: impl Into<String>,
    ) -> Self {
        let started = Utc::now();
        let completed = Utc::now();
        let duration = completed.signed_duration_since(started).num_seconds();

        Self {
            step_number,
            step_name: step_name.into(),
            success,
            message: message.into(),
            started_at: started,
            completed_at: completed,
            duration_secs: duration,
        }
    }

    pub fn with_timing(mut self, started_at: DateTime<Utc>, completed_at: DateTime<Utc>) -> Self {
        self.started_at = started_at;
        self.completed_at = completed_at;
        self.duration_secs = completed_at.signed_duration_since(started_at).num_seconds();
        self
    }
}

/// Workflow executor
pub struct WorkflowExecutor;

impl WorkflowExecutor {
    /// Execute a workflow
    pub fn execute(workflow: &Workflow) -> WorkflowExecution {
        let mut execution = WorkflowExecution::new(&workflow.id, &workflow.name);

        let mut completed_steps: Vec<u32> = Vec::new();

        for step in &workflow.steps {
            execution.current_step = Some(step.step_number);

            // Check if dependencies are met
            if !step.can_execute(&completed_steps) {
                let result =
                    StepResult::new(step.step_number, &step.name, false, "Dependencies not met");
                execution.add_step_result(result);
                continue;
            }

            // Simulate step execution
            let result = StepResult::new(
                step.step_number,
                &step.name,
                true,
                format!("Step {} executed successfully", step.step_number),
            );

            execution.add_step_result(result);
            completed_steps.push(step.step_number);
        }

        let final_status = if execution.failed_steps().is_empty() {
            ExecutionStatus::Completed
        } else if !completed_steps.is_empty() {
            ExecutionStatus::PartialSuccess
        } else {
            ExecutionStatus::Failed
        };

        execution.complete(final_status);
        execution
    }

    /// Validate workflow (check for circular dependencies, etc.)
    pub fn validate(workflow: &Workflow) -> Vec<String> {
        let mut errors = Vec::new();

        // Check for duplicate step numbers
        let mut seen_steps = std::collections::HashSet::new();
        for step in &workflow.steps {
            if !seen_steps.insert(step.step_number) {
                errors.push(format!("Duplicate step number: {}", step.step_number));
            }
        }

        // Check for invalid dependencies
        for step in &workflow.steps {
            for dep in &step.dependencies {
                if !workflow.steps.iter().any(|s| s.step_number == *dep) {
                    errors.push(format!(
                        "Step {} depends on non-existent step {}",
                        step.step_number, dep
                    ));
                }

                if dep >= &step.step_number {
                    errors.push(format!(
                        "Step {} cannot depend on step {} (circular or forward dependency)",
                        step.step_number, dep
                    ));
                }
            }
        }

        errors
    }
}

/// Pre-built workflow templates
pub struct WorkflowTemplates;

impl WorkflowTemplates {
    /// VM provisioning workflow
    pub fn vm_provisioning() -> Workflow {
        Workflow::new("VM Provisioning")
            .with_description("Complete VM provisioning workflow")
            .add_step(WorkflowStep::new(
                1,
                "Create VM",
                Action::new(super::ActionType::StartVM {
                    vm_name: "new-vm".to_string(),
                }),
            ))
            .add_step(
                WorkflowStep::new(
                    2,
                    "Configure Network",
                    Action::new(super::ActionType::RunScript {
                        script: "configure-network.sh".to_string(),
                    }),
                )
                .depends_on(1),
            )
            .add_step(
                WorkflowStep::new(
                    3,
                    "Create Snapshot",
                    Action::new(super::ActionType::CreateSnapshot {
                        vm_name: "new-vm".to_string(),
                        snapshot_name: Some("initial-snapshot".to_string()),
                    }),
                )
                .depends_on(2),
            )
    }

    /// Disaster recovery workflow
    pub fn disaster_recovery() -> Workflow {
        Workflow::new("Disaster Recovery")
            .with_description("Restore VMs from backup")
            .add_step(WorkflowStep::new(
                1,
                "Stop Running VMs",
                Action::new(super::ActionType::StopVM {
                    vm_name: "old-vm".to_string(),
                }),
            ))
            .add_step(
                WorkflowStep::new(
                    2,
                    "Restore from Backup",
                    Action::new(super::ActionType::CreateBackup {
                        vm_name: "restored-vm".to_string(),
                    }),
                )
                .depends_on(1),
            )
            .add_step(
                WorkflowStep::new(
                    3,
                    "Start Restored VM",
                    Action::new(super::ActionType::StartVM {
                        vm_name: "restored-vm".to_string(),
                    }),
                )
                .depends_on(2),
            )
    }

    /// Maintenance workflow
    pub fn maintenance() -> Workflow {
        Workflow::new("Scheduled Maintenance")
            .with_description("Perform scheduled maintenance tasks")
            .add_step(WorkflowStep::new(
                1,
                "Create Pre-Maintenance Snapshot",
                Action::new(super::ActionType::CreateSnapshot {
                    vm_name: "prod-vm".to_string(),
                    snapshot_name: Some("pre-maintenance".to_string()),
                }),
            ))
            .add_step(
                WorkflowStep::new(
                    2,
                    "Run Maintenance Script",
                    Action::new(super::ActionType::RunScript {
                        script: "maintenance.sh".to_string(),
                    }),
                )
                .depends_on(1)
                .with_timeout(3600),
            )
            .add_step(
                WorkflowStep::new(
                    3,
                    "Restart VM",
                    Action::new(super::ActionType::RestartVM {
                        vm_name: "prod-vm".to_string(),
                    }),
                )
                .depends_on(2),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow() {
        let workflow = Workflow::new("Test Workflow")
            .with_description("Test workflow description")
            .add_step(WorkflowStep::new(
                1,
                "Step 1",
                Action::new(super::super::ActionType::StartVM {
                    vm_name: "test-vm".to_string(),
                }),
            ));

        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.step_count(), 1);
    }

    #[test]
    fn test_workflow_step_dependencies() {
        let step = WorkflowStep::new(
            2,
            "Dependent Step",
            Action::new(super::super::ActionType::StopVM {
                vm_name: "test-vm".to_string(),
            }),
        )
        .depends_on(1);

        assert_eq!(step.dependencies, vec![1]);
        assert!(step.can_execute(&[1]));
        assert!(!step.can_execute(&[]));
    }

    #[test]
    fn test_workflow_execution() {
        let mut execution = WorkflowExecution::new("wf-123", "Test Workflow");

        execution.add_step_result(StepResult::new(1, "Step 1", true, "Success"));
        execution.add_step_result(StepResult::new(2, "Step 2", false, "Failed"));

        assert_eq!(execution.step_results.len(), 2);
        assert_eq!(execution.completed_steps(), vec![1]);
        assert_eq!(execution.failed_steps().len(), 1);
        assert_eq!(execution.success_rate(), 50.0);
    }

    #[test]
    fn test_workflow_executor() {
        let workflow = Workflow::new("Simple Workflow")
            .add_step(WorkflowStep::new(
                1,
                "First",
                Action::new(super::super::ActionType::StartVM {
                    vm_name: "vm1".to_string(),
                }),
            ))
            .add_step(
                WorkflowStep::new(
                    2,
                    "Second",
                    Action::new(super::super::ActionType::StopVM {
                        vm_name: "vm1".to_string(),
                    }),
                )
                .depends_on(1),
            );

        let execution = WorkflowExecutor::execute(&workflow);

        assert_eq!(execution.status, ExecutionStatus::Completed);
        assert_eq!(execution.step_results.len(), 2);
    }

    #[test]
    fn test_workflow_validation() {
        let workflow = Workflow::new("Invalid Workflow")
            .add_step(WorkflowStep::new(
                1,
                "Step 1",
                Action::new(super::super::ActionType::StartVM {
                    vm_name: "vm1".to_string(),
                }),
            ))
            .add_step(
                WorkflowStep::new(
                    2,
                    "Step 2",
                    Action::new(super::super::ActionType::StopVM {
                        vm_name: "vm1".to_string(),
                    }),
                )
                .depends_on(99),
            ); // Invalid dependency

        let errors = WorkflowExecutor::validate(&workflow);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_duplicate_step_validation() {
        let workflow = Workflow::new("Duplicate Steps")
            .add_step(WorkflowStep::new(
                1,
                "Step 1",
                Action::new(super::super::ActionType::StartVM {
                    vm_name: "vm1".to_string(),
                }),
            ))
            .add_step(WorkflowStep::new(
                1,
                "Step 1 Duplicate",
                Action::new(super::super::ActionType::StopVM {
                    vm_name: "vm1".to_string(),
                }),
            ));

        let errors = WorkflowExecutor::validate(&workflow);
        assert!(errors.iter().any(|e| e.contains("Duplicate step number")));
    }

    #[test]
    fn test_workflow_templates() {
        let provisioning = WorkflowTemplates::vm_provisioning();
        assert_eq!(provisioning.name, "VM Provisioning");
        assert!(provisioning.step_count() > 0);

        let dr = WorkflowTemplates::disaster_recovery();
        assert_eq!(dr.name, "Disaster Recovery");

        let maintenance = WorkflowTemplates::maintenance();
        assert_eq!(maintenance.name, "Scheduled Maintenance");
    }

    #[test]
    fn test_step_result_timing() {
        let started = Utc::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let completed = Utc::now();

        let result = StepResult::new(1, "Test", true, "Success").with_timing(started, completed);

        assert!(result.duration_secs >= 0);
    }

    #[test]
    fn test_workflow_execution_duration() {
        let mut execution = WorkflowExecution::new("wf-123", "Test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        execution.complete(ExecutionStatus::Completed);

        assert!(execution.duration_secs() >= 0);
    }
}
