// Training Job Management - ML training orchestration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{MLFramework, WorkloadStatus};

/// Training job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    pub id: String,
    pub name: String,
    pub framework: MLFramework,
    pub model_name: String,
    pub dataset_path: String,
    pub output_path: String,
    pub hyperparameters: HashMap<String, String>,
    pub epochs: u32,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub distributed: bool,
    pub num_workers: u32,
    pub status: WorkloadStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metrics: TrainingMetrics,
}

impl TrainingJob {
    pub fn new(
        name: impl Into<String>,
        framework: MLFramework,
        model_name: impl Into<String>,
        dataset_path: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "train-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            framework,
            model_name: model_name.into(),
            dataset_path: dataset_path.into(),
            output_path: String::new(),
            hyperparameters: HashMap::new(),
            epochs: 10,
            batch_size: 32,
            learning_rate: 0.001,
            distributed: false,
            num_workers: 1,
            status: WorkloadStatus::Pending,
            started_at: None,
            completed_at: None,
            metrics: TrainingMetrics::new(),
        }
    }

    pub fn with_output(mut self, path: impl Into<String>) -> Self {
        self.output_path = path.into();
        self
    }

    pub fn with_hyperparameters(
        mut self,
        epochs: u32,
        batch_size: u32,
        learning_rate: f64,
    ) -> Self {
        self.epochs = epochs;
        self.batch_size = batch_size;
        self.learning_rate = learning_rate;
        self
    }

    pub fn with_distributed(mut self, num_workers: u32) -> Self {
        self.distributed = true;
        self.num_workers = num_workers;
        self
    }

    pub fn add_hyperparameter(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.hyperparameters.insert(key.into(), value.into());
    }

    pub fn start(&mut self) {
        self.status = WorkloadStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self, final_accuracy: f64) {
        self.status = WorkloadStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.metrics.final_accuracy = Some(final_accuracy);
    }

    pub fn fail(&mut self) {
        self.status = WorkloadStatus::Failed;
        self.completed_at = Some(Utc::now());
    }

    pub fn duration_seconds(&self) -> Option<i64> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end.signed_duration_since(start).num_seconds()),
            _ => None,
        }
    }
}

/// Training metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub current_epoch: u32,
    pub current_loss: f64,
    pub current_accuracy: f64,
    pub final_accuracy: Option<f64>,
    pub samples_processed: u64,
}

impl TrainingMetrics {
    pub fn new() -> Self {
        Self {
            current_epoch: 0,
            current_loss: 0.0,
            current_accuracy: 0.0,
            final_accuracy: None,
            samples_processed: 0,
        }
    }

    pub fn update(&mut self, epoch: u32, loss: f64, accuracy: f64) {
        self.current_epoch = epoch;
        self.current_loss = loss;
        self.current_accuracy = accuracy;
    }
}

impl Default for TrainingMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Training job manager
pub struct TrainingJobManager {
    jobs: HashMap<String, TrainingJob>,
}

impl TrainingJobManager {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
        }
    }

    pub fn create_job(&mut self, job: TrainingJob) -> String {
        let id = job.id.clone();
        self.jobs.insert(id.clone(), job);
        id
    }

    pub fn get_job(&self, id: &str) -> Option<&TrainingJob> {
        self.jobs.get(id)
    }

    pub fn get_job_mut(&mut self, id: &str) -> Option<&mut TrainingJob> {
        self.jobs.get_mut(id)
    }

    pub fn list_jobs(&self) -> Vec<&TrainingJob> {
        self.jobs.values().collect()
    }

    pub fn by_status(&self, status: WorkloadStatus) -> Vec<&TrainingJob> {
        self.jobs.values().filter(|j| j.status == status).collect()
    }

    pub fn running_jobs(&self) -> Vec<&TrainingJob> {
        self.by_status(WorkloadStatus::Running)
    }

    pub fn completed_jobs(&self) -> Vec<&TrainingJob> {
        self.by_status(WorkloadStatus::Completed)
    }

    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }
}

impl Default for TrainingJobManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_job() {
        let job = TrainingJob::new(
            "image-classifier",
            MLFramework::PyTorch,
            "resnet50",
            "/data/imagenet",
        )
        .with_output("/models/resnet50")
        .with_hyperparameters(100, 64, 0.01);

        assert_eq!(job.name, "image-classifier");
        assert_eq!(job.framework, MLFramework::PyTorch);
        assert_eq!(job.model_name, "resnet50");
        assert_eq!(job.epochs, 100);
        assert_eq!(job.batch_size, 64);
        assert_eq!(job.learning_rate, 0.01);
    }

    #[test]
    fn test_job_distributed() {
        let job = TrainingJob::new(
            "distributed-training",
            MLFramework::TensorFlow,
            "bert",
            "/data/corpus",
        )
        .with_distributed(4);

        assert!(job.distributed);
        assert_eq!(job.num_workers, 4);
    }

    #[test]
    fn test_job_lifecycle() {
        let mut job = TrainingJob::new("test", MLFramework::PyTorch, "model", "/data");

        assert_eq!(job.status, WorkloadStatus::Pending);

        job.start();
        assert_eq!(job.status, WorkloadStatus::Running);
        assert!(job.started_at.is_some());

        job.complete(0.95);
        assert_eq!(job.status, WorkloadStatus::Completed);
        assert!(job.completed_at.is_some());
        assert_eq!(job.metrics.final_accuracy, Some(0.95));
    }

    #[test]
    fn test_job_failure() {
        let mut job = TrainingJob::new("test", MLFramework::PyTorch, "model", "/data");

        job.start();
        job.fail();

        assert_eq!(job.status, WorkloadStatus::Failed);
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_job_duration() {
        let mut job = TrainingJob::new("test", MLFramework::PyTorch, "model", "/data");

        job.start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        job.complete(0.90);

        let duration = job.duration_seconds();
        assert!(duration.is_some());
        assert!(duration.unwrap() >= 0);
    }

    #[test]
    fn test_training_metrics() {
        let mut metrics = TrainingMetrics::new();

        metrics.update(5, 0.25, 0.85);

        assert_eq!(metrics.current_epoch, 5);
        assert_eq!(metrics.current_loss, 0.25);
        assert_eq!(metrics.current_accuracy, 0.85);
    }

    #[test]
    fn test_job_hyperparameters() {
        let mut job = TrainingJob::new("test", MLFramework::PyTorch, "model", "/data");

        job.add_hyperparameter("optimizer", "adam");
        job.add_hyperparameter("momentum", "0.9");

        assert_eq!(
            job.hyperparameters.get("optimizer"),
            Some(&"adam".to_string())
        );
        assert_eq!(
            job.hyperparameters.get("momentum"),
            Some(&"0.9".to_string())
        );
    }

    #[test]
    fn test_training_job_manager() {
        let mut manager = TrainingJobManager::new();

        let job = TrainingJob::new("test", MLFramework::PyTorch, "model", "/data");
        let id = manager.create_job(job);

        assert_eq!(manager.job_count(), 1);
        assert!(manager.get_job(&id).is_some());
    }

    #[test]
    fn test_manager_by_status() {
        let mut manager = TrainingJobManager::new();

        let mut job1 = TrainingJob::new("job1", MLFramework::PyTorch, "model1", "/data");
        job1.start();

        let job2 = TrainingJob::new("job2", MLFramework::TensorFlow, "model2", "/data");

        manager.create_job(job1);
        manager.create_job(job2);

        assert_eq!(manager.running_jobs().len(), 1);
        assert_eq!(manager.by_status(WorkloadStatus::Pending).len(), 1);
    }

    #[test]
    fn test_manager_completed_jobs() {
        let mut manager = TrainingJobManager::new();

        let mut job = TrainingJob::new("completed", MLFramework::PyTorch, "model", "/data");
        job.start();
        job.complete(0.92);

        manager.create_job(job);

        assert_eq!(manager.completed_jobs().len(), 1);
    }
}
