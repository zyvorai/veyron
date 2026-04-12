// AI/ML Operations - GPU acceleration and ML workload management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod gpu;
pub mod inference;
pub mod models;
pub mod training;

/// GPU configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUConfig {
    pub vendor: GPUVendor,
    pub model: String,
    pub count: u32,
    pub memory_gb: u32,
    pub passthrough: bool,
    pub vgpu_profile: Option<String>,
}

impl GPUConfig {
    pub fn new(vendor: GPUVendor, model: impl Into<String>, count: u32) -> Self {
        Self {
            vendor,
            model: model.into(),
            count,
            memory_gb: 0,
            passthrough: true,
            vgpu_profile: None,
        }
    }

    pub fn with_memory(mut self, memory_gb: u32) -> Self {
        self.memory_gb = memory_gb;
        self
    }

    pub fn with_vgpu(mut self, profile: impl Into<String>) -> Self {
        self.passthrough = false;
        self.vgpu_profile = Some(profile.into());
        self
    }

    pub fn is_vgpu(&self) -> bool {
        self.vgpu_profile.is_some()
    }
}

/// GPU vendor
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GPUVendor {
    NVIDIA,
    AMD,
    Intel,
}

impl std::fmt::Display for GPUVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GPUVendor::NVIDIA => write!(f, "NVIDIA"),
            GPUVendor::AMD => write!(f, "AMD"),
            GPUVendor::Intel => write!(f, "Intel"),
        }
    }
}

/// ML workload type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkloadType {
    Training,
    Inference,
    FineTuning,
    DataProcessing,
}

/// ML framework
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MLFramework {
    PyTorch,
    TensorFlow,
    JAX,
    ONNX,
    ScikitLearn,
    XGBoost,
}

impl std::fmt::Display for MLFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MLFramework::PyTorch => write!(f, "PyTorch"),
            MLFramework::TensorFlow => write!(f, "TensorFlow"),
            MLFramework::JAX => write!(f, "JAX"),
            MLFramework::ONNX => write!(f, "ONNX"),
            MLFramework::ScikitLearn => write!(f, "scikit-learn"),
            MLFramework::XGBoost => write!(f, "XGBoost"),
        }
    }
}

/// ML workload specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLWorkload {
    pub id: String,
    pub name: String,
    pub workload_type: WorkloadType,
    pub framework: MLFramework,
    pub gpu_config: Option<GPUConfig>,
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_gb: u32,
    pub distributed: bool,
    pub replicas: u32,
    pub created_at: DateTime<Utc>,
    pub status: WorkloadStatus,
}

impl MLWorkload {
    pub fn new(
        name: impl Into<String>,
        workload_type: WorkloadType,
        framework: MLFramework,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "ml-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            workload_type,
            framework,
            gpu_config: None,
            cpu_cores: 4,
            memory_gb: 16,
            storage_gb: 100,
            distributed: false,
            replicas: 1,
            created_at: Utc::now(),
            status: WorkloadStatus::Pending,
        }
    }

    pub fn with_gpu(mut self, config: GPUConfig) -> Self {
        self.gpu_config = Some(config);
        self
    }

    pub fn with_resources(mut self, cpu_cores: u32, memory_gb: u32, storage_gb: u32) -> Self {
        self.cpu_cores = cpu_cores;
        self.memory_gb = memory_gb;
        self.storage_gb = storage_gb;
        self
    }

    pub fn with_distributed(mut self, replicas: u32) -> Self {
        self.distributed = true;
        self.replicas = replicas;
        self
    }

    pub fn requires_gpu(&self) -> bool {
        self.gpu_config.is_some()
    }

    pub fn total_cpu_cores(&self) -> u32 {
        self.cpu_cores * self.replicas
    }

    pub fn total_memory_gb(&self) -> u32 {
        self.memory_gb * self.replicas
    }

    pub fn update_status(&mut self, status: WorkloadStatus) {
        self.status = status;
    }
}

/// Workload status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkloadStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for WorkloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkloadStatus::Pending => write!(f, "Pending"),
            WorkloadStatus::Running => write!(f, "Running"),
            WorkloadStatus::Completed => write!(f, "Completed"),
            WorkloadStatus::Failed => write!(f, "Failed"),
            WorkloadStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub min_cpu_cores: u32,
    pub min_memory_gb: u32,
    pub min_gpus: u32,
    pub preferred_gpu_vendor: Option<GPUVendor>,
    pub min_gpu_memory_gb: u32,
}

impl ResourceRequirements {
    pub fn new(cpu: u32, memory: u32) -> Self {
        Self {
            min_cpu_cores: cpu,
            min_memory_gb: memory,
            min_gpus: 0,
            preferred_gpu_vendor: None,
            min_gpu_memory_gb: 0,
        }
    }

    pub fn with_gpu(mut self, count: u32, memory_gb: u32, vendor: Option<GPUVendor>) -> Self {
        self.min_gpus = count;
        self.min_gpu_memory_gb = memory_gb;
        self.preferred_gpu_vendor = vendor;
        self
    }

    pub fn requires_gpu(&self) -> bool {
        self.min_gpus > 0
    }
}

/// ML workload manager
pub struct MLWorkloadManager {
    workloads: HashMap<String, MLWorkload>,
}

impl MLWorkloadManager {
    pub fn new() -> Self {
        Self {
            workloads: HashMap::new(),
        }
    }

    pub fn add_workload(&mut self, workload: MLWorkload) -> String {
        let id = workload.id.clone();
        self.workloads.insert(id.clone(), workload);
        id
    }

    pub fn get_workload(&self, id: &str) -> Option<&MLWorkload> {
        self.workloads.get(id)
    }

    pub fn get_workload_mut(&mut self, id: &str) -> Option<&mut MLWorkload> {
        self.workloads.get_mut(id)
    }

    pub fn remove_workload(&mut self, id: &str) -> bool {
        self.workloads.remove(id).is_some()
    }

    pub fn list_workloads(&self) -> Vec<&MLWorkload> {
        self.workloads.values().collect()
    }

    pub fn by_type(&self, workload_type: WorkloadType) -> Vec<&MLWorkload> {
        self.workloads
            .values()
            .filter(|w| w.workload_type == workload_type)
            .collect()
    }

    pub fn by_framework(&self, framework: MLFramework) -> Vec<&MLWorkload> {
        self.workloads
            .values()
            .filter(|w| w.framework == framework)
            .collect()
    }

    pub fn by_status(&self, status: WorkloadStatus) -> Vec<&MLWorkload> {
        self.workloads
            .values()
            .filter(|w| w.status == status)
            .collect()
    }

    pub fn running_workloads(&self) -> Vec<&MLWorkload> {
        self.by_status(WorkloadStatus::Running)
    }

    pub fn gpu_workloads(&self) -> Vec<&MLWorkload> {
        self.workloads
            .values()
            .filter(|w| w.requires_gpu())
            .collect()
    }

    pub fn workload_count(&self) -> usize {
        self.workloads.len()
    }

    pub fn running_count(&self) -> usize {
        self.running_workloads().len()
    }

    pub fn total_gpu_allocation(&self) -> u32 {
        self.gpu_workloads()
            .iter()
            .filter_map(|w| {
                w.gpu_config
                    .as_ref()
                    .map(|config| config.count * w.replicas)
            })
            .sum()
    }
}

impl Default for MLWorkloadManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_config() {
        let gpu = GPUConfig::new(GPUVendor::NVIDIA, "A100", 2).with_memory(80);

        assert_eq!(gpu.vendor, GPUVendor::NVIDIA);
        assert_eq!(gpu.model, "A100");
        assert_eq!(gpu.count, 2);
        assert_eq!(gpu.memory_gb, 80);
        assert!(gpu.passthrough);
    }

    #[test]
    fn test_gpu_vgpu() {
        let gpu = GPUConfig::new(GPUVendor::NVIDIA, "A100", 1).with_vgpu("grid_a100-8c");

        assert!(gpu.is_vgpu());
        assert!(!gpu.passthrough);
        assert_eq!(gpu.vgpu_profile, Some("grid_a100-8c".to_string()));
    }

    #[test]
    fn test_gpu_vendor_display() {
        assert_eq!(GPUVendor::NVIDIA.to_string(), "NVIDIA");
        assert_eq!(GPUVendor::AMD.to_string(), "AMD");
        assert_eq!(GPUVendor::Intel.to_string(), "Intel");
    }

    #[test]
    fn test_ml_framework_display() {
        assert_eq!(MLFramework::PyTorch.to_string(), "PyTorch");
        assert_eq!(MLFramework::TensorFlow.to_string(), "TensorFlow");
    }

    #[test]
    fn test_ml_workload() {
        let workload =
            MLWorkload::new("training-job", WorkloadType::Training, MLFramework::PyTorch)
                .with_resources(8, 32, 200);

        assert_eq!(workload.name, "training-job");
        assert_eq!(workload.workload_type, WorkloadType::Training);
        assert_eq!(workload.framework, MLFramework::PyTorch);
        assert_eq!(workload.cpu_cores, 8);
        assert_eq!(workload.memory_gb, 32);
        assert!(!workload.requires_gpu());
    }

    #[test]
    fn test_workload_with_gpu() {
        let gpu = GPUConfig::new(GPUVendor::NVIDIA, "V100", 4).with_memory(32);

        let workload =
            MLWorkload::new("gpu-training", WorkloadType::Training, MLFramework::PyTorch)
                .with_gpu(gpu);

        assert!(workload.requires_gpu());
        assert_eq!(workload.gpu_config.as_ref().unwrap().count, 4);
    }

    #[test]
    fn test_workload_distributed() {
        let workload = MLWorkload::new(
            "distributed-training",
            WorkloadType::Training,
            MLFramework::TensorFlow,
        )
        .with_resources(8, 32, 200)
        .with_distributed(4);

        assert!(workload.distributed);
        assert_eq!(workload.replicas, 4);
        assert_eq!(workload.total_cpu_cores(), 32);
        assert_eq!(workload.total_memory_gb(), 128);
    }

    #[test]
    fn test_workload_status() {
        let mut workload = MLWorkload::new("test", WorkloadType::Training, MLFramework::PyTorch);

        assert_eq!(workload.status, WorkloadStatus::Pending);

        workload.update_status(WorkloadStatus::Running);
        assert_eq!(workload.status, WorkloadStatus::Running);
    }

    #[test]
    fn test_workload_status_display() {
        assert_eq!(WorkloadStatus::Pending.to_string(), "Pending");
        assert_eq!(WorkloadStatus::Running.to_string(), "Running");
        assert_eq!(WorkloadStatus::Completed.to_string(), "Completed");
    }

    #[test]
    fn test_resource_requirements() {
        let req = ResourceRequirements::new(16, 64).with_gpu(2, 16, Some(GPUVendor::NVIDIA));

        assert_eq!(req.min_cpu_cores, 16);
        assert_eq!(req.min_memory_gb, 64);
        assert!(req.requires_gpu());
        assert_eq!(req.min_gpus, 2);
        assert_eq!(req.preferred_gpu_vendor, Some(GPUVendor::NVIDIA));
    }

    #[test]
    fn test_resource_requirements_no_gpu() {
        let req = ResourceRequirements::new(8, 16);

        assert!(!req.requires_gpu());
        assert_eq!(req.min_gpus, 0);
    }

    #[test]
    fn test_ml_workload_manager() {
        let mut manager = MLWorkloadManager::new();

        let workload = MLWorkload::new("test", WorkloadType::Training, MLFramework::PyTorch);
        let id = manager.add_workload(workload);

        assert_eq!(manager.workload_count(), 1);
        assert!(manager.get_workload(&id).is_some());
    }

    #[test]
    fn test_manager_by_type() {
        let mut manager = MLWorkloadManager::new();

        manager.add_workload(MLWorkload::new(
            "training-1",
            WorkloadType::Training,
            MLFramework::PyTorch,
        ));
        manager.add_workload(MLWorkload::new(
            "inference-1",
            WorkloadType::Inference,
            MLFramework::TensorFlow,
        ));
        manager.add_workload(MLWorkload::new(
            "training-2",
            WorkloadType::Training,
            MLFramework::JAX,
        ));

        let training = manager.by_type(WorkloadType::Training);
        assert_eq!(training.len(), 2);
    }

    #[test]
    fn test_manager_by_framework() {
        let mut manager = MLWorkloadManager::new();

        manager.add_workload(MLWorkload::new(
            "job-1",
            WorkloadType::Training,
            MLFramework::PyTorch,
        ));
        manager.add_workload(MLWorkload::new(
            "job-2",
            WorkloadType::Training,
            MLFramework::PyTorch,
        ));
        manager.add_workload(MLWorkload::new(
            "job-3",
            WorkloadType::Inference,
            MLFramework::TensorFlow,
        ));

        let pytorch = manager.by_framework(MLFramework::PyTorch);
        assert_eq!(pytorch.len(), 2);
    }

    #[test]
    fn test_manager_by_status() {
        let mut manager = MLWorkloadManager::new();

        let mut workload1 = MLWorkload::new("job-1", WorkloadType::Training, MLFramework::PyTorch);
        workload1.update_status(WorkloadStatus::Running);

        let workload2 = MLWorkload::new("job-2", WorkloadType::Training, MLFramework::TensorFlow);

        manager.add_workload(workload1);
        manager.add_workload(workload2);

        assert_eq!(manager.running_count(), 1);
        assert_eq!(manager.by_status(WorkloadStatus::Pending).len(), 1);
    }

    #[test]
    fn test_manager_gpu_workloads() {
        let mut manager = MLWorkloadManager::new();

        let gpu = GPUConfig::new(GPUVendor::NVIDIA, "A100", 2);
        let gpu_workload =
            MLWorkload::new("gpu-job", WorkloadType::Training, MLFramework::PyTorch).with_gpu(gpu);

        let cpu_workload =
            MLWorkload::new("cpu-job", WorkloadType::Training, MLFramework::TensorFlow);

        manager.add_workload(gpu_workload);
        manager.add_workload(cpu_workload);

        assert_eq!(manager.gpu_workloads().len(), 1);
    }

    #[test]
    fn test_manager_total_gpu_allocation() {
        let mut manager = MLWorkloadManager::new();

        let gpu1 = GPUConfig::new(GPUVendor::NVIDIA, "A100", 2);
        let workload1 =
            MLWorkload::new("job-1", WorkloadType::Training, MLFramework::PyTorch).with_gpu(gpu1);

        let gpu2 = GPUConfig::new(GPUVendor::NVIDIA, "V100", 4);
        let workload2 = MLWorkload::new("job-2", WorkloadType::Training, MLFramework::TensorFlow)
            .with_gpu(gpu2);

        manager.add_workload(workload1);
        manager.add_workload(workload2);

        assert_eq!(manager.total_gpu_allocation(), 6);
    }

    #[test]
    fn test_manager_remove_workload() {
        let mut manager = MLWorkloadManager::new();

        let workload = MLWorkload::new("test", WorkloadType::Training, MLFramework::PyTorch);
        let id = manager.add_workload(workload);

        assert!(manager.remove_workload(&id));
        assert_eq!(manager.workload_count(), 0);
    }

    #[test]
    fn test_workload_type() {
        assert_eq!(WorkloadType::Training, WorkloadType::Training);
        assert_ne!(WorkloadType::Training, WorkloadType::Inference);
    }

    #[test]
    fn test_ml_framework() {
        assert_eq!(MLFramework::PyTorch, MLFramework::PyTorch);
        assert_ne!(MLFramework::PyTorch, MLFramework::TensorFlow);
    }
}
