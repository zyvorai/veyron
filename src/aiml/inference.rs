// Inference Serving - ML model serving and inference

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{MLFramework, WorkloadStatus};

/// Inference endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceEndpoint {
    pub id: String,
    pub name: String,
    pub model_id: String,
    pub framework: MLFramework,
    pub endpoint_url: String,
    pub replicas: u32,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub autoscaling: bool,
    pub batch_size: u32,
    pub timeout_ms: u64,
    pub status: WorkloadStatus,
    pub created_at: DateTime<Utc>,
    pub metrics: InferenceMetrics,
}

impl InferenceEndpoint {
    pub fn new(
        name: impl Into<String>,
        model_id: impl Into<String>,
        framework: MLFramework,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "inference-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            model_id: model_id.into(),
            framework,
            endpoint_url: String::new(),
            replicas: 1,
            min_replicas: 1,
            max_replicas: 10,
            autoscaling: false,
            batch_size: 1,
            timeout_ms: 30000,
            status: WorkloadStatus::Pending,
            created_at: Utc::now(),
            metrics: InferenceMetrics::new(),
        }
    }

    pub fn with_endpoint(mut self, url: impl Into<String>) -> Self {
        self.endpoint_url = url.into();
        self
    }

    pub fn with_replicas(mut self, replicas: u32) -> Self {
        self.replicas = replicas;
        self
    }

    pub fn with_autoscaling(mut self, min: u32, max: u32) -> Self {
        self.autoscaling = true;
        self.min_replicas = min;
        self.max_replicas = max;
        self
    }

    pub fn with_batch_size(mut self, batch_size: u32) -> Self {
        self.batch_size = batch_size;
        self
    }

    pub fn deploy(&mut self) {
        self.status = WorkloadStatus::Running;
    }

    pub fn update_status(&mut self, status: WorkloadStatus) {
        self.status = status;
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, WorkloadStatus::Running)
    }
}

/// Inference metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub throughput_rps: f64,
}

impl InferenceMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            throughput_rps: 0.0,
        }
    }

    pub fn record_request(&mut self, success: bool, latency_ms: f64) {
        self.total_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }

        // Running average
        self.avg_latency_ms = (self.avg_latency_ms * (self.total_requests - 1) as f64 + latency_ms)
            / self.total_requests as f64;

        // Track approximate percentiles using max-biased estimation.
        // P95/P99 converge toward the high end of observed latencies.
        if self.total_requests == 1 {
            self.p95_latency_ms = latency_ms;
            self.p99_latency_ms = latency_ms;
        } else {
            // Exponential moving approach: slowly converge toward the target percentile
            if latency_ms > self.p95_latency_ms {
                self.p95_latency_ms += (latency_ms - self.p95_latency_ms) * 0.05;
            } else {
                self.p95_latency_ms -= (self.p95_latency_ms - latency_ms) * 0.005;
            }
            if latency_ms > self.p99_latency_ms {
                self.p99_latency_ms += (latency_ms - self.p99_latency_ms) * 0.01;
            } else {
                self.p99_latency_ms -= (self.p99_latency_ms - latency_ms) * 0.001;
            }
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }
}

impl Default for InferenceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Inference service manager
pub struct InferenceServiceManager {
    endpoints: HashMap<String, InferenceEndpoint>,
}

impl InferenceServiceManager {
    pub fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
        }
    }

    pub fn create_endpoint(&mut self, endpoint: InferenceEndpoint) -> String {
        let id = endpoint.id.clone();
        self.endpoints.insert(id.clone(), endpoint);
        id
    }

    pub fn get_endpoint(&self, id: &str) -> Option<&InferenceEndpoint> {
        self.endpoints.get(id)
    }

    pub fn get_endpoint_mut(&mut self, id: &str) -> Option<&mut InferenceEndpoint> {
        self.endpoints.get_mut(id)
    }

    pub fn remove_endpoint(&mut self, id: &str) -> bool {
        self.endpoints.remove(id).is_some()
    }

    pub fn list_endpoints(&self) -> Vec<&InferenceEndpoint> {
        self.endpoints.values().collect()
    }

    pub fn running_endpoints(&self) -> Vec<&InferenceEndpoint> {
        self.endpoints.values().filter(|e| e.is_running()).collect()
    }

    pub fn by_model(&self, model_id: &str) -> Vec<&InferenceEndpoint> {
        self.endpoints
            .values()
            .filter(|e| e.model_id == model_id)
            .collect()
    }

    pub fn with_autoscaling(&self) -> Vec<&InferenceEndpoint> {
        self.endpoints.values().filter(|e| e.autoscaling).collect()
    }

    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    pub fn running_count(&self) -> usize {
        self.running_endpoints().len()
    }

    pub fn total_replicas(&self) -> u32 {
        self.endpoints.values().map(|e| e.replicas).sum()
    }

    pub fn total_requests(&self) -> u64 {
        self.endpoints
            .values()
            .map(|e| e.metrics.total_requests)
            .sum()
    }
}

impl Default for InferenceServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_endpoint() {
        let endpoint =
            InferenceEndpoint::new("image-classifier", "model-123", MLFramework::PyTorch)
                .with_endpoint("http://localhost:8080/predict")
                .with_replicas(3)
                .with_batch_size(16);

        assert_eq!(endpoint.name, "image-classifier");
        assert_eq!(endpoint.model_id, "model-123");
        assert_eq!(endpoint.framework, MLFramework::PyTorch);
        assert_eq!(endpoint.replicas, 3);
        assert_eq!(endpoint.batch_size, 16);
    }

    #[test]
    fn test_endpoint_autoscaling() {
        let endpoint =
            InferenceEndpoint::new("scalable-service", "model-456", MLFramework::TensorFlow)
                .with_autoscaling(2, 10);

        assert!(endpoint.autoscaling);
        assert_eq!(endpoint.min_replicas, 2);
        assert_eq!(endpoint.max_replicas, 10);
    }

    #[test]
    fn test_endpoint_deployment() {
        let mut endpoint = InferenceEndpoint::new("test", "model-1", MLFramework::PyTorch);

        assert_eq!(endpoint.status, WorkloadStatus::Pending);
        assert!(!endpoint.is_running());

        endpoint.deploy();

        assert_eq!(endpoint.status, WorkloadStatus::Running);
        assert!(endpoint.is_running());
    }

    #[test]
    fn test_endpoint_status_update() {
        let mut endpoint = InferenceEndpoint::new("test", "model-1", MLFramework::PyTorch);

        endpoint.update_status(WorkloadStatus::Running);
        assert_eq!(endpoint.status, WorkloadStatus::Running);

        endpoint.update_status(WorkloadStatus::Failed);
        assert_eq!(endpoint.status, WorkloadStatus::Failed);
    }

    #[test]
    fn test_inference_metrics() {
        let metrics = InferenceMetrics::new();

        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
    }

    #[test]
    fn test_metrics_record_request() {
        let mut metrics = InferenceMetrics::new();

        metrics.record_request(true, 15.5);
        metrics.record_request(true, 20.0);
        metrics.record_request(false, 100.0);

        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 1);
    }

    #[test]
    fn test_metrics_success_rate() {
        let mut metrics = InferenceMetrics::new();

        metrics.record_request(true, 10.0);
        metrics.record_request(true, 12.0);
        metrics.record_request(false, 50.0);
        metrics.record_request(true, 11.0);

        let success_rate = metrics.success_rate();
        assert!((success_rate - 75.0).abs() < 0.1); // 3/4 = 75%
    }

    #[test]
    fn test_metrics_avg_latency() {
        let mut metrics = InferenceMetrics::new();

        metrics.record_request(true, 10.0);
        metrics.record_request(true, 20.0);

        assert!((metrics.avg_latency_ms - 15.0).abs() < 0.1);
    }

    #[test]
    fn test_inference_service_manager() {
        let mut manager = InferenceServiceManager::new();

        let endpoint = InferenceEndpoint::new("test", "model-1", MLFramework::PyTorch);
        let id = manager.create_endpoint(endpoint);

        assert_eq!(manager.endpoint_count(), 1);
        assert!(manager.get_endpoint(&id).is_some());
    }

    #[test]
    fn test_manager_running_endpoints() {
        let mut manager = InferenceServiceManager::new();

        let mut endpoint1 = InferenceEndpoint::new("endpoint1", "model-1", MLFramework::PyTorch);
        endpoint1.deploy();

        let endpoint2 = InferenceEndpoint::new("endpoint2", "model-2", MLFramework::TensorFlow);

        manager.create_endpoint(endpoint1);
        manager.create_endpoint(endpoint2);

        assert_eq!(manager.running_count(), 1);
    }

    #[test]
    fn test_manager_by_model() {
        let mut manager = InferenceServiceManager::new();

        manager.create_endpoint(InferenceEndpoint::new(
            "endpoint1",
            "model-123",
            MLFramework::PyTorch,
        ));
        manager.create_endpoint(InferenceEndpoint::new(
            "endpoint2",
            "model-123",
            MLFramework::TensorFlow,
        ));
        manager.create_endpoint(InferenceEndpoint::new(
            "endpoint3",
            "model-456",
            MLFramework::JAX,
        ));

        let model_123_endpoints = manager.by_model("model-123");
        assert_eq!(model_123_endpoints.len(), 2);
    }

    #[test]
    fn test_manager_with_autoscaling() {
        let mut manager = InferenceServiceManager::new();

        manager.create_endpoint(
            InferenceEndpoint::new("scalable", "model-1", MLFramework::PyTorch)
                .with_autoscaling(1, 5),
        );
        manager.create_endpoint(InferenceEndpoint::new(
            "static",
            "model-2",
            MLFramework::TensorFlow,
        ));

        assert_eq!(manager.with_autoscaling().len(), 1);
    }

    #[test]
    fn test_manager_total_replicas() {
        let mut manager = InferenceServiceManager::new();

        manager.create_endpoint(
            InferenceEndpoint::new("endpoint1", "model-1", MLFramework::PyTorch).with_replicas(3),
        );
        manager.create_endpoint(
            InferenceEndpoint::new("endpoint2", "model-2", MLFramework::TensorFlow)
                .with_replicas(2),
        );

        assert_eq!(manager.total_replicas(), 5);
    }

    #[test]
    fn test_manager_total_requests() {
        let mut manager = InferenceServiceManager::new();

        let mut endpoint1 = InferenceEndpoint::new("endpoint1", "model-1", MLFramework::PyTorch);
        endpoint1.metrics.record_request(true, 10.0);
        endpoint1.metrics.record_request(true, 12.0);

        let mut endpoint2 = InferenceEndpoint::new("endpoint2", "model-2", MLFramework::TensorFlow);
        endpoint2.metrics.record_request(true, 15.0);

        manager.create_endpoint(endpoint1);
        manager.create_endpoint(endpoint2);

        assert_eq!(manager.total_requests(), 3);
    }

    #[test]
    fn test_manager_remove_endpoint() {
        let mut manager = InferenceServiceManager::new();

        let endpoint = InferenceEndpoint::new("test", "model-1", MLFramework::PyTorch);
        let id = manager.create_endpoint(endpoint);

        assert!(manager.remove_endpoint(&id));
        assert_eq!(manager.endpoint_count(), 0);
    }
}
