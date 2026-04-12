// GPU Resource Management - GPU allocation and monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::GPUVendor;

/// GPU resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUResource {
    pub id: String,
    pub vendor: GPUVendor,
    pub model: String,
    pub pci_address: String,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub utilization_percent: f64,
    pub temperature_celsius: Option<u32>,
    pub power_usage_watts: Option<u32>,
    pub allocated_to: Option<String>,
}

impl GPUResource {
    pub fn new(
        id: impl Into<String>,
        vendor: GPUVendor,
        model: impl Into<String>,
        pci_address: impl Into<String>,
        memory_mb: u64,
    ) -> Self {
        Self {
            id: id.into(),
            vendor,
            model: model.into(),
            pci_address: pci_address.into(),
            memory_total_mb: memory_mb,
            memory_used_mb: 0,
            utilization_percent: 0.0,
            temperature_celsius: None,
            power_usage_watts: None,
            allocated_to: None,
        }
    }

    pub fn allocate(&mut self, workload_id: impl Into<String>) {
        self.allocated_to = Some(workload_id.into());
    }

    pub fn deallocate(&mut self) {
        self.allocated_to = None;
        self.memory_used_mb = 0;
        self.utilization_percent = 0.0;
    }

    pub fn is_allocated(&self) -> bool {
        self.allocated_to.is_some()
    }

    pub fn is_available(&self) -> bool {
        !self.is_allocated()
    }

    pub fn memory_free_mb(&self) -> u64 {
        self.memory_total_mb.saturating_sub(self.memory_used_mb)
    }

    pub fn memory_usage_percent(&self) -> f64 {
        if self.memory_total_mb == 0 {
            0.0
        } else {
            (self.memory_used_mb as f64 / self.memory_total_mb as f64) * 100.0
        }
    }

    pub fn update_metrics(&mut self, used_mb: u64, utilization: f64) {
        self.memory_used_mb = used_mb;
        self.utilization_percent = utilization;
    }

    pub fn update_thermal(&mut self, temp_c: u32, power_w: u32) {
        self.temperature_celsius = Some(temp_c);
        self.power_usage_watts = Some(power_w);
    }
}

/// GPU pool
pub struct GPUPool {
    gpus: HashMap<String, GPUResource>,
}

impl GPUPool {
    pub fn new() -> Self {
        Self {
            gpus: HashMap::new(),
        }
    }

    pub fn add_gpu(&mut self, gpu: GPUResource) {
        self.gpus.insert(gpu.id.clone(), gpu);
    }

    pub fn remove_gpu(&mut self, gpu_id: &str) -> bool {
        self.gpus.remove(gpu_id).is_some()
    }

    pub fn get_gpu(&self, gpu_id: &str) -> Option<&GPUResource> {
        self.gpus.get(gpu_id)
    }

    pub fn get_gpu_mut(&mut self, gpu_id: &str) -> Option<&mut GPUResource> {
        self.gpus.get_mut(gpu_id)
    }

    pub fn list_gpus(&self) -> Vec<&GPUResource> {
        self.gpus.values().collect()
    }

    pub fn available_gpus(&self) -> Vec<&GPUResource> {
        self.gpus.values().filter(|g| g.is_available()).collect()
    }

    pub fn allocated_gpus(&self) -> Vec<&GPUResource> {
        self.gpus.values().filter(|g| g.is_allocated()).collect()
    }

    pub fn gpus_by_vendor(&self, vendor: GPUVendor) -> Vec<&GPUResource> {
        self.gpus.values().filter(|g| g.vendor == vendor).collect()
    }

    pub fn total_count(&self) -> usize {
        self.gpus.len()
    }

    pub fn available_count(&self) -> usize {
        self.available_gpus().len()
    }

    pub fn allocated_count(&self) -> usize {
        self.allocated_gpus().len()
    }

    pub fn total_memory_mb(&self) -> u64 {
        self.gpus.values().map(|g| g.memory_total_mb).sum()
    }

    pub fn available_memory_mb(&self) -> u64 {
        self.available_gpus()
            .iter()
            .map(|g| g.memory_total_mb)
            .sum()
    }

    pub fn allocate_gpu(
        &mut self,
        workload_id: &str,
        requirements: &GPUAllocationRequest,
    ) -> Option<String> {
        // Find suitable GPU
        let gpu = self
            .available_gpus()
            .into_iter()
            .filter(|g| {
                if let Some(vendor) = &requirements.preferred_vendor {
                    g.vendor == *vendor
                } else {
                    true
                }
            })
            .find(|g| g.memory_total_mb >= requirements.min_memory_mb);

        if let Some(gpu) = gpu {
            let gpu_id = gpu.id.clone();
            if let Some(gpu_mut) = self.get_gpu_mut(&gpu_id) {
                gpu_mut.allocate(workload_id);
                return Some(gpu_id);
            }
        }

        None
    }

    pub fn deallocate_gpu(&mut self, gpu_id: &str) {
        if let Some(gpu) = self.get_gpu_mut(gpu_id) {
            gpu.deallocate();
        }
    }
}

impl Default for GPUPool {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU allocation request
#[derive(Debug, Clone)]
pub struct GPUAllocationRequest {
    pub preferred_vendor: Option<GPUVendor>,
    pub min_memory_mb: u64,
    pub count: u32,
}

impl GPUAllocationRequest {
    pub fn new(count: u32) -> Self {
        Self {
            preferred_vendor: None,
            min_memory_mb: 0,
            count,
        }
    }

    pub fn with_vendor(mut self, vendor: GPUVendor) -> Self {
        self.preferred_vendor = Some(vendor);
        self
    }

    pub fn with_memory(mut self, memory_mb: u64) -> Self {
        self.min_memory_mb = memory_mb;
        self
    }
}

/// GPU metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUMetrics {
    pub gpu_id: String,
    pub utilization_percent: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub temperature_celsius: u32,
    pub power_usage_watts: u32,
    pub collected_at: DateTime<Utc>,
}

impl GPUMetrics {
    pub fn from_resource(resource: &GPUResource) -> Self {
        Self {
            gpu_id: resource.id.clone(),
            utilization_percent: resource.utilization_percent,
            memory_used_mb: resource.memory_used_mb,
            memory_total_mb: resource.memory_total_mb,
            temperature_celsius: resource.temperature_celsius.unwrap_or(0),
            power_usage_watts: resource.power_usage_watts.unwrap_or(0),
            collected_at: Utc::now(),
        }
    }

    pub fn memory_usage_percent(&self) -> f64 {
        if self.memory_total_mb == 0 {
            0.0
        } else {
            (self.memory_used_mb as f64 / self.memory_total_mb as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_resource() {
        let gpu = GPUResource::new("gpu-0", GPUVendor::NVIDIA, "A100", "0000:00:00.0", 81920);

        assert_eq!(gpu.id, "gpu-0");
        assert_eq!(gpu.vendor, GPUVendor::NVIDIA);
        assert_eq!(gpu.model, "A100");
        assert_eq!(gpu.memory_total_mb, 81920);
        assert!(!gpu.is_allocated());
        assert!(gpu.is_available());
    }

    #[test]
    fn test_gpu_allocation() {
        let mut gpu = GPUResource::new("gpu-0", GPUVendor::NVIDIA, "V100", "0000:00:00.0", 32768);

        gpu.allocate("workload-1");

        assert!(gpu.is_allocated());
        assert!(!gpu.is_available());
        assert_eq!(gpu.allocated_to, Some("workload-1".to_string()));
    }

    #[test]
    fn test_gpu_deallocation() {
        let mut gpu = GPUResource::new("gpu-0", GPUVendor::NVIDIA, "A100", "0000:00:00.0", 81920);

        gpu.allocate("workload-1");
        gpu.update_metrics(40960, 75.0);

        gpu.deallocate();

        assert!(gpu.is_available());
        assert_eq!(gpu.allocated_to, None);
        assert_eq!(gpu.memory_used_mb, 0);
        assert_eq!(gpu.utilization_percent, 0.0);
    }

    #[test]
    fn test_gpu_memory_metrics() {
        let mut gpu = GPUResource::new("gpu-0", GPUVendor::NVIDIA, "A100", "0000:00:00.0", 81920);

        gpu.update_metrics(40960, 50.0);

        assert_eq!(gpu.memory_free_mb(), 40960);
        assert_eq!(gpu.memory_usage_percent(), 50.0);
    }

    #[test]
    fn test_gpu_thermal_metrics() {
        let mut gpu = GPUResource::new("gpu-0", GPUVendor::NVIDIA, "A100", "0000:00:00.0", 81920);

        gpu.update_thermal(75, 300);

        assert_eq!(gpu.temperature_celsius, Some(75));
        assert_eq!(gpu.power_usage_watts, Some(300));
    }

    #[test]
    fn test_gpu_pool() {
        let mut pool = GPUPool::new();

        let gpu1 = GPUResource::new("gpu-0", GPUVendor::NVIDIA, "A100", "0000:00:00.0", 81920);
        let gpu2 = GPUResource::new("gpu-1", GPUVendor::NVIDIA, "V100", "0000:01:00.0", 32768);

        pool.add_gpu(gpu1);
        pool.add_gpu(gpu2);

        assert_eq!(pool.total_count(), 2);
        assert_eq!(pool.available_count(), 2);
    }

    #[test]
    fn test_pool_available_gpus() {
        let mut pool = GPUPool::new();

        let mut gpu1 = GPUResource::new("gpu-0", GPUVendor::NVIDIA, "A100", "0000:00:00.0", 81920);
        gpu1.allocate("workload-1");

        let gpu2 = GPUResource::new("gpu-1", GPUVendor::NVIDIA, "V100", "0000:01:00.0", 32768);

        pool.add_gpu(gpu1);
        pool.add_gpu(gpu2);

        assert_eq!(pool.available_count(), 1);
        assert_eq!(pool.allocated_count(), 1);
    }

    #[test]
    fn test_pool_by_vendor() {
        let mut pool = GPUPool::new();

        pool.add_gpu(GPUResource::new(
            "gpu-0",
            GPUVendor::NVIDIA,
            "A100",
            "0000:00:00.0",
            81920,
        ));
        pool.add_gpu(GPUResource::new(
            "gpu-1",
            GPUVendor::AMD,
            "MI100",
            "0000:01:00.0",
            32768,
        ));
        pool.add_gpu(GPUResource::new(
            "gpu-2",
            GPUVendor::NVIDIA,
            "V100",
            "0000:02:00.0",
            32768,
        ));

        let nvidia_gpus = pool.gpus_by_vendor(GPUVendor::NVIDIA);
        assert_eq!(nvidia_gpus.len(), 2);
    }

    #[test]
    fn test_pool_total_memory() {
        let mut pool = GPUPool::new();

        pool.add_gpu(GPUResource::new(
            "gpu-0",
            GPUVendor::NVIDIA,
            "A100",
            "0000:00:00.0",
            81920,
        ));
        pool.add_gpu(GPUResource::new(
            "gpu-1",
            GPUVendor::NVIDIA,
            "V100",
            "0000:01:00.0",
            32768,
        ));

        assert_eq!(pool.total_memory_mb(), 114688);
    }

    #[test]
    fn test_pool_allocate_gpu() {
        let mut pool = GPUPool::new();

        pool.add_gpu(GPUResource::new(
            "gpu-0",
            GPUVendor::NVIDIA,
            "A100",
            "0000:00:00.0",
            81920,
        ));
        pool.add_gpu(GPUResource::new(
            "gpu-1",
            GPUVendor::NVIDIA,
            "V100",
            "0000:01:00.0",
            32768,
        ));

        let request = GPUAllocationRequest::new(1).with_memory(40000);

        let allocated = pool.allocate_gpu("workload-1", &request);

        assert!(allocated.is_some());
        assert_eq!(pool.available_count(), 1);
    }

    #[test]
    fn test_pool_allocate_with_vendor_preference() {
        let mut pool = GPUPool::new();

        pool.add_gpu(GPUResource::new(
            "gpu-0",
            GPUVendor::AMD,
            "MI100",
            "0000:00:00.0",
            32768,
        ));
        pool.add_gpu(GPUResource::new(
            "gpu-1",
            GPUVendor::NVIDIA,
            "V100",
            "0000:01:00.0",
            32768,
        ));

        let request = GPUAllocationRequest::new(1).with_vendor(GPUVendor::NVIDIA);

        let allocated = pool.allocate_gpu("workload-1", &request);

        assert!(allocated.is_some());
        assert_eq!(allocated.unwrap(), "gpu-1");
    }

    #[test]
    fn test_pool_deallocate() {
        let mut pool = GPUPool::new();

        let mut gpu = GPUResource::new("gpu-0", GPUVendor::NVIDIA, "A100", "0000:00:00.0", 81920);
        gpu.allocate("workload-1");

        pool.add_gpu(gpu);

        pool.deallocate_gpu("gpu-0");

        let gpu = pool.get_gpu("gpu-0").unwrap();
        assert!(gpu.is_available());
    }

    #[test]
    fn test_pool_remove_gpu() {
        let mut pool = GPUPool::new();

        pool.add_gpu(GPUResource::new(
            "gpu-0",
            GPUVendor::NVIDIA,
            "A100",
            "0000:00:00.0",
            81920,
        ));

        assert!(pool.remove_gpu("gpu-0"));
        assert_eq!(pool.total_count(), 0);
    }

    #[test]
    fn test_allocation_request() {
        let request = GPUAllocationRequest::new(2)
            .with_vendor(GPUVendor::NVIDIA)
            .with_memory(81920);

        assert_eq!(request.count, 2);
        assert_eq!(request.preferred_vendor, Some(GPUVendor::NVIDIA));
        assert_eq!(request.min_memory_mb, 81920);
    }

    #[test]
    fn test_gpu_metrics() {
        let mut gpu = GPUResource::new("gpu-0", GPUVendor::NVIDIA, "A100", "0000:00:00.0", 81920);
        gpu.update_metrics(40960, 60.0);
        gpu.update_thermal(70, 280);

        let metrics = GPUMetrics::from_resource(&gpu);

        assert_eq!(metrics.gpu_id, "gpu-0");
        assert_eq!(metrics.utilization_percent, 60.0);
        assert_eq!(metrics.memory_used_mb, 40960);
        assert_eq!(metrics.temperature_celsius, 70);
        assert_eq!(metrics.memory_usage_percent(), 50.0);
    }

    #[test]
    fn test_pool_available_memory() {
        let mut pool = GPUPool::new();

        let mut gpu1 = GPUResource::new("gpu-0", GPUVendor::NVIDIA, "A100", "0000:00:00.0", 81920);
        gpu1.allocate("workload-1");

        let gpu2 = GPUResource::new("gpu-1", GPUVendor::NVIDIA, "V100", "0000:01:00.0", 32768);

        pool.add_gpu(gpu1);
        pool.add_gpu(gpu2);

        assert_eq!(pool.available_memory_mb(), 32768);
    }
}
