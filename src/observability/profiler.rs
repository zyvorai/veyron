// Performance Profiler - CPU and memory profiling data collection

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfiler {
    pub profiles: Vec<Profile>,
    pub config: ProfilerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub vm_name: String,
    pub profile_type: ProfileType,
    pub started_at: DateTime<Utc>,
    pub duration_secs: u64,
    pub data_points: Vec<ProfileDataPoint>,
    pub summary: ProfileSummary,
    pub status: ProfileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileType {
    CPU,
    Memory,
    IO,
    Network,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDataPoint {
    pub timestamp: DateTime<Utc>,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub avg_cpu: f64,
    pub max_cpu: f64,
    pub avg_memory: f64,
    pub max_memory: f64,
    pub total_io_read: u64,
    pub total_io_write: u64,
    pub bottleneck: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProfileStatus {
    Recording,
    Completed,
    Analyzing,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilerConfig {
    pub sample_interval_ms: u64,
    pub max_duration_secs: u64,
    pub auto_analyze: bool,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            sample_interval_ms: 1000,
            max_duration_secs: 300,
            auto_analyze: true,
        }
    }
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            profiles: Vec::new(),
            config: ProfilerConfig::default(),
        }
    }

    pub fn start_profile(&mut self, vm_name: &str, profile_type: ProfileType) -> String {
        let id = format!("profile-{}", Utc::now().timestamp_micros());
        self.profiles.push(Profile {
            id: id.clone(),
            vm_name: vm_name.to_string(),
            profile_type,
            started_at: Utc::now(),
            duration_secs: 0,
            data_points: Vec::new(),
            summary: ProfileSummary {
                avg_cpu: 0.0,
                max_cpu: 0.0,
                avg_memory: 0.0,
                max_memory: 0.0,
                total_io_read: 0,
                total_io_write: 0,
                bottleneck: None,
            },
            status: ProfileStatus::Recording,
        });
        id
    }

    pub fn stop_profile(&mut self, id: &str) {
        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == id) {
            p.status = ProfileStatus::Analyzing;
            p.duration_secs = Utc::now().signed_duration_since(p.started_at).num_seconds() as u64;
            self.analyze_profile(id);
        }
    }

    fn analyze_profile(&mut self, id: &str) {
        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == id) {
            if !p.data_points.is_empty() {
                let n = p.data_points.len() as f64;
                p.summary.avg_cpu = p.data_points.iter().map(|d| d.cpu_percent).sum::<f64>() / n;
                p.summary.max_cpu = p
                    .data_points
                    .iter()
                    .map(|d| d.cpu_percent)
                    .fold(0.0_f64, f64::max);
                p.summary.avg_memory =
                    p.data_points.iter().map(|d| d.memory_percent).sum::<f64>() / n;
                p.summary.max_memory = p
                    .data_points
                    .iter()
                    .map(|d| d.memory_percent)
                    .fold(0.0_f64, f64::max);
                p.summary.total_io_read = p.data_points.iter().map(|d| d.io_read_bytes).sum();
                p.summary.total_io_write = p.data_points.iter().map(|d| d.io_write_bytes).sum();

                p.summary.bottleneck = if p.summary.max_cpu > 90.0 {
                    Some("CPU".to_string())
                } else if p.summary.max_memory > 90.0 {
                    Some("Memory".to_string())
                } else {
                    None
                };
            }
            p.status = ProfileStatus::Completed;
        }
    }

    pub fn get_profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }
    pub fn list(&self) -> &[Profile] {
        &self.profiles
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}
