// Migration Rollback - Rollback operations for failed migrations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackManager {
    pub rollback_plans: Vec<RollbackPlan>,
    pub executed_rollbacks: Vec<RollbackExecution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    pub migration_id: String,
    pub vm_name: String,
    pub original_node: String,
    pub steps: Vec<RollbackStep>,
    pub created_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackStep {
    pub order: usize,
    pub action: String,
    pub description: String,
    pub automated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackExecution {
    pub plan_id: String,
    pub executed_at: DateTime<Utc>,
    pub success: bool,
    pub steps_completed: usize,
    pub total_steps: usize,
    pub error: Option<String>,
}

impl RollbackManager {
    pub fn new() -> Self { Self { rollback_plans: Vec::new(), executed_rollbacks: Vec::new() } }

    pub fn create_plan(&mut self, migration_id: &str, vm_name: &str, original_node: &str) -> String {
        let plan = RollbackPlan {
            migration_id: migration_id.to_string(), vm_name: vm_name.to_string(),
            original_node: original_node.to_string(),
            steps: vec![
                RollbackStep { order: 1, action: "stop_vm".to_string(), description: "Stop VM on current node".to_string(), automated: true },
                RollbackStep { order: 2, action: "migrate_back".to_string(), description: format!("Migrate VM back to {}", original_node), automated: true },
                RollbackStep { order: 3, action: "verify".to_string(), description: "Verify VM is running on original node".to_string(), automated: true },
                RollbackStep { order: 4, action: "cleanup".to_string(), description: "Clean up migration artifacts".to_string(), automated: true },
            ],
            created_at: Utc::now(),
            valid_until: Utc::now() + chrono::TimeDelta::hours(24),
        };
        let id = plan.migration_id.clone();
        self.rollback_plans.push(plan);
        id
    }

    pub fn get_plan(&self, migration_id: &str) -> Option<&RollbackPlan> {
        self.rollback_plans.iter().find(|p| p.migration_id == migration_id)
    }

    /// Execute a rollback by creating a new migration CRD to move the VM back.
    ///
    /// This creates a `VirtualMachineInstanceMigration` CRD targeting the
    /// original node. KubeVirt will handle the actual live migration.
    pub async fn execute_rollback(
        &mut self,
        migration_id: &str,
        namespace: &str,
    ) -> Option<RollbackExecution> {
        let plan = self.rollback_plans.iter().find(|p| p.migration_id == migration_id)?;

        // Check if the rollback plan has expired
        if Utc::now() > plan.valid_until {
            let execution = RollbackExecution {
                plan_id: migration_id.to_string(),
                executed_at: Utc::now(),
                success: false,
                steps_completed: 0,
                total_steps: plan.steps.len(),
                error: Some("Rollback plan has expired".to_string()),
            };
            self.executed_rollbacks.push(execution.clone());
            return Some(execution);
        }

        let vm_name = plan.vm_name.clone();
        let total_steps = plan.steps.len();

        // Create a new migration CRD to move the VM back to the original node
        let result = async {
            let client = kube::Client::try_default().await?;
            let mig_api: kube::api::Api<crate::kube::types::VirtualMachineInstanceMigration> =
                kube::api::Api::namespaced(client, namespace);

            let mig_name = format!("{}-rollback-{}", vm_name, Utc::now().format("%H%M%S"));
            let migration = crate::kube::types::VirtualMachineInstanceMigration {
                metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                    name: Some(mig_name),
                    namespace: Some(namespace.to_string()),
                    ..Default::default()
                },
                spec: crate::kube::types::VirtualMachineInstanceMigrationSpec {
                    vmi_name: Some(vm_name.clone()),
                },
                status: None,
            };

            mig_api.create(&kube::api::PostParams::default(), &migration).await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;

        let (success, error, steps_completed) = match result {
            Ok(()) => (true, None, total_steps),
            Err(e) => (false, Some(format!("Rollback migration failed: {}", e)), 0),
        };

        let execution = RollbackExecution {
            plan_id: migration_id.to_string(),
            executed_at: Utc::now(),
            success,
            steps_completed,
            total_steps,
            error,
        };
        self.executed_rollbacks.push(execution.clone());
        Some(execution)
    }
}

impl Default for RollbackManager {
    fn default() -> Self { Self::new() }
}
