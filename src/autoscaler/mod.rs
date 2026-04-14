// Auto-Scaler - Autonomous resource scaling for KubeVirt VMs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Auto-scaling engine that monitors VMs and scales resources automatically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScaler {
    pub policies: Vec<AutoScalingPolicy>,
    pub policy_versions: Vec<PolicyVersion>,
    pub active_scalings: HashMap<String, ActiveScaling>,
    pub scaling_history: Vec<ScalingEvent>,
    pub config: AutoScalerConfig,
    #[serde(skip)]
    pub last_evaluation: Option<DateTime<Utc>>,
}

/// Configuration for the auto-scaler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalerConfig {
    pub enabled: bool,
    pub evaluation_interval_secs: u64,
    pub cooldown_period_secs: u64,
    pub dry_run: bool,
    pub auto_apply: bool,
    pub max_cpu_cores: u32,
    pub max_memory_gi: u32,
    pub min_cpu_cores: u32,
    pub min_memory_gi: u32,
    pub cpu_cost_per_core_hour: f64,
    pub memory_cost_per_gi_hour: f64,
}

impl Default for AutoScalerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            evaluation_interval_secs: 300,
            cooldown_period_secs: 600,
            dry_run: true,
            auto_apply: false,
            max_cpu_cores: 64,
            max_memory_gi: 256,
            min_cpu_cores: 1,
            min_memory_gi: 1,
            cpu_cost_per_core_hour: 0.0416,
            memory_cost_per_gi_hour: 0.0104,
        }
    }
}

/// Auto-scaling policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalingPolicy {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub target: ScalingTarget,
    pub cpu_threshold: ThresholdConfig,
    pub memory_threshold: ThresholdConfig,
    pub schedule: Option<PolicySchedule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Scaling target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingTarget {
    SingleVm { name: String, namespace: String },
    Namespace { namespace: String },
    LabelSelector { selector: HashMap<String, String> },
}

/// Threshold configuration for scaling decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub scale_up_factor: f64,
    pub scale_down_factor: f64,
    pub min_samples: usize,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            scale_up_threshold: 80.0,
            scale_down_threshold: 20.0,
            scale_up_factor: 1.5,
            scale_down_factor: 0.75,
            min_samples: 5,
        }
    }
}

/// Schedule for time-based policy activation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicySchedule {
    pub time_windows: Vec<TimeWindow>,
    pub active_days: Vec<DayOfWeek>,
    pub timezone: String,
    pub blackout_periods: Vec<DateRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeWindow {
    pub start_hour: u8,
    pub start_minute: u8,
    pub end_hour: u8,
    pub end_minute: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub description: String,
}

impl PolicySchedule {
    pub fn business_hours() -> Self {
        Self {
            time_windows: vec![TimeWindow {
                start_hour: 9,
                start_minute: 0,
                end_hour: 17,
                end_minute: 0,
            }],
            active_days: vec![
                DayOfWeek::Monday,
                DayOfWeek::Tuesday,
                DayOfWeek::Wednesday,
                DayOfWeek::Thursday,
                DayOfWeek::Friday,
            ],
            timezone: "UTC".to_string(),
            blackout_periods: Vec::new(),
        }
    }

    pub fn always() -> Self {
        Self {
            time_windows: Vec::new(),
            active_days: Vec::new(),
            timezone: "UTC".to_string(),
            blackout_periods: Vec::new(),
        }
    }
}

/// Active scaling operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveScaling {
    pub vm_name: String,
    pub namespace: String,
    pub started_at: DateTime<Utc>,
    pub direction: ScalingDirection,
    pub original_cpu: u32,
    pub original_memory: u32,
    pub target_cpu: u32,
    pub target_memory: u32,
    pub status: ScalingStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
    RolledBack,
}

/// Scaling event for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingEvent {
    pub id: String,
    pub vm_name: String,
    pub namespace: String,
    pub timestamp: DateTime<Utc>,
    pub direction: ScalingDirection,
    pub policy_name: String,
    pub original_cpu: u32,
    pub original_memory: u32,
    pub new_cpu: u32,
    pub new_memory: u32,
    pub reason: String,
    pub success: bool,
    pub cost_impact: f64,
}

/// Scaling recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingRecommendation {
    pub vm_name: String,
    pub namespace: String,
    pub reason: String,
    pub current_cpu: u32,
    pub recommended_cpu: u32,
    pub current_memory: u32,
    pub recommended_memory: u32,
    pub confidence: f64,
    pub potential_savings: f64,
}

/// Policy version for tracking changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyVersion {
    pub policy_id: String,
    pub version: u32,
    pub changed_at: DateTime<Utc>,
    pub changed_by: String,
    pub changes: String,
}

impl AutoScaler {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            policy_versions: Vec::new(),
            active_scalings: HashMap::new(),
            scaling_history: Vec::new(),
            config: AutoScalerConfig::default(),
            last_evaluation: None,
        }
    }

    pub fn add_policy(&mut self, policy: AutoScalingPolicy) {
        self.policies.push(policy);
    }

    pub fn remove_policy(&mut self, policy_id: &str) -> bool {
        let len = self.policies.len();
        self.policies.retain(|p| p.id != policy_id);
        self.policies.len() < len
    }

    pub fn get_policy(&self, policy_id: &str) -> Option<&AutoScalingPolicy> {
        self.policies.iter().find(|p| p.id == policy_id)
    }

    pub fn enabled_policies(&self) -> Vec<&AutoScalingPolicy> {
        self.policies.iter().filter(|p| p.enabled).collect()
    }

    pub fn evaluate(
        &mut self,
        vm_metrics: &HashMap<String, VmMetrics>,
    ) -> Vec<ScalingRecommendation> {
        if !self.config.enabled {
            return Vec::new();
        }

        let mut recommendations = Vec::new();

        for policy in &self.policies {
            if !policy.enabled {
                continue;
            }

            let target_vms = self.get_target_vms(policy, vm_metrics);

            for (vm_name, metrics) in target_vms {
                if let Some(rec) = self.evaluate_vm(&vm_name, metrics, policy) {
                    recommendations.push(rec);
                }
            }
        }

        self.last_evaluation = Some(Utc::now());
        recommendations
    }

    fn get_target_vms<'a>(
        &self,
        policy: &AutoScalingPolicy,
        metrics: &'a HashMap<String, VmMetrics>,
    ) -> Vec<(String, &'a VmMetrics)> {
        match &policy.target {
            ScalingTarget::SingleVm { name, .. } => metrics
                .get(name)
                .map(|m| vec![(name.clone(), m)])
                .unwrap_or_default(),
            ScalingTarget::Namespace { namespace } => metrics
                .iter()
                .filter(|(_, m)| m.namespace == *namespace)
                .map(|(k, v)| (k.clone(), v))
                .collect(),
            ScalingTarget::LabelSelector { .. } => {
                metrics.iter().map(|(k, v)| (k.clone(), v)).collect()
            }
        }
    }

    fn evaluate_vm(
        &self,
        vm_name: &str,
        metrics: &VmMetrics,
        policy: &AutoScalingPolicy,
    ) -> Option<ScalingRecommendation> {
        let avg_cpu = if metrics.cpu_samples.is_empty() {
            0.0
        } else {
            metrics.cpu_samples.iter().sum::<f64>() / metrics.cpu_samples.len() as f64
        };
        let avg_memory = if metrics.memory_samples.is_empty() {
            0.0
        } else {
            metrics.memory_samples.iter().sum::<f64>() / metrics.memory_samples.len() as f64
        };

        let cpu_scale = if avg_cpu > policy.cpu_threshold.scale_up_threshold {
            Some((ScalingDirection::Up, policy.cpu_threshold.scale_up_factor))
        } else if avg_cpu < policy.cpu_threshold.scale_down_threshold {
            Some((
                ScalingDirection::Down,
                policy.cpu_threshold.scale_down_factor,
            ))
        } else {
            None
        };

        let mem_scale = if avg_memory > policy.memory_threshold.scale_up_threshold {
            Some((
                ScalingDirection::Up,
                policy.memory_threshold.scale_up_factor,
            ))
        } else if avg_memory < policy.memory_threshold.scale_down_threshold {
            Some((
                ScalingDirection::Down,
                policy.memory_threshold.scale_down_factor,
            ))
        } else {
            None
        };

        if cpu_scale.is_none() && mem_scale.is_none() {
            return None;
        }

        let recommended_cpu = match &cpu_scale {
            Some((_, factor)) => ((metrics.current_cpu as f64 * factor).ceil() as u32)
                .clamp(self.config.min_cpu_cores, self.config.max_cpu_cores),
            None => metrics.current_cpu,
        };

        let recommended_memory = match &mem_scale {
            Some((_, factor)) => ((metrics.current_memory as f64 * factor).ceil() as u32)
                .clamp(self.config.min_memory_gi, self.config.max_memory_gi),
            None => metrics.current_memory,
        };

        if recommended_cpu == metrics.current_cpu && recommended_memory == metrics.current_memory {
            return None;
        }

        let cpu_cost_diff = (recommended_cpu as f64 - metrics.current_cpu as f64)
            * self.config.cpu_cost_per_core_hour
            * 730.0;
        let mem_cost_diff = (recommended_memory as f64 - metrics.current_memory as f64)
            * self.config.memory_cost_per_gi_hour
            * 730.0;

        Some(ScalingRecommendation {
            vm_name: vm_name.to_string(),
            namespace: metrics.namespace.clone(),
            reason: format!("CPU avg: {:.1}%, Memory avg: {:.1}%", avg_cpu, avg_memory),
            current_cpu: metrics.current_cpu,
            recommended_cpu,
            current_memory: metrics.current_memory,
            recommended_memory,
            confidence: 0.85,
            potential_savings: -(cpu_cost_diff + mem_cost_diff),
        })
    }

    pub fn record_event(&mut self, event: ScalingEvent) {
        self.scaling_history.push(event);
        if self.scaling_history.len() > 1000 {
            log::warn!("Scaling history exceeded 1,000 entries, trimming to 900");
            let excess = self.scaling_history.len().saturating_sub(900);
            if excess > 0 {
                self.scaling_history.drain(0..excess);
            }
        }
        // Cap policy versions
        if self.policy_versions.len() > 500 {
            log::warn!("Policy version history exceeded 500 entries, trimming to 400");
            let excess = self.policy_versions.len().saturating_sub(400);
            if excess > 0 {
                self.policy_versions.drain(0..excess);
            }
        }
    }

    pub fn recent_events(&self, limit: usize) -> Vec<&ScalingEvent> {
        self.scaling_history.iter().rev().take(limit).collect()
    }
}

impl Default for AutoScaler {
    fn default() -> Self {
        Self::new()
    }
}

/// VM metrics for scaling decisions
#[derive(Debug, Clone)]
pub struct VmMetrics {
    pub namespace: String,
    pub current_cpu: u32,
    pub current_memory: u32,
    pub cpu_samples: Vec<f64>,
    pub memory_samples: Vec<f64>,
}
