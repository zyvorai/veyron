// Cost Optimization - Identify cost-saving opportunities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub id: String,
    pub vm_name: String,
    pub recommendation_type: RecommendationType,
    pub priority: Priority,
    pub potential_savings: f64,
    pub savings_percent: f64,
    pub description: String,
    pub action_items: Vec<String>,
    pub impact: Impact,
    pub generated_at: DateTime<Utc>,
}

impl OptimizationRecommendation {
    pub fn new(
        vm_name: impl Into<String>,
        recommendation_type: RecommendationType,
        potential_savings: f64,
    ) -> Self {
        let vm = vm_name.into();
        let id = format!("rec-{}-{}", vm, Utc::now().timestamp());

        Self {
            id,
            vm_name: vm,
            recommendation_type,
            priority: Priority::Medium,
            potential_savings,
            savings_percent: 0.0,
            description: String::new(),
            action_items: Vec::new(),
            impact: Impact::Low,
            generated_at: Utc::now(),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn add_action(mut self, action: impl Into<String>) -> Self {
        self.action_items.push(action.into());
        self
    }

    pub fn with_impact(mut self, impact: Impact) -> Self {
        self.impact = impact;
        self
    }

    pub fn with_savings_percent(mut self, percent: f64) -> Self {
        self.savings_percent = percent;
        self
    }
}

/// Recommendation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationType {
    RightSize,        // Reduce CPU/memory to match usage
    Shutdown,         // Stop idle VMs
    Schedule,         // Schedule start/stop times
    Storage,          // Optimize storage (remove unused, compress)
    InstanceType,     // Switch to cheaper instance type
    SpotInstance,     // Use spot/preemptible instances
    ReservedCapacity, // Purchase reserved capacity
    SnapshotCleanup,  // Remove old snapshots
}

impl std::fmt::Display for RecommendationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecommendationType::RightSize => write!(f, "Right-size Resources"),
            RecommendationType::Shutdown => write!(f, "Shutdown Idle VM"),
            RecommendationType::Schedule => write!(f, "Schedule Start/Stop"),
            RecommendationType::Storage => write!(f, "Optimize Storage"),
            RecommendationType::InstanceType => write!(f, "Change Instance Type"),
            RecommendationType::SpotInstance => write!(f, "Use Spot Instances"),
            RecommendationType::ReservedCapacity => write!(f, "Reserved Capacity"),
            RecommendationType::SnapshotCleanup => write!(f, "Snapshot Cleanup"),
        }
    }
}

/// Recommendation priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "Low"),
            Priority::Medium => write!(f, "Medium"),
            Priority::High => write!(f, "High"),
            Priority::Critical => write!(f, "Critical"),
        }
    }
}

/// Impact level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Impact {
    Low,    // Minimal performance impact
    Medium, // Noticeable but acceptable
    High,   // Significant impact, needs testing
}

/// Waste detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasteReport {
    pub vm_name: String,
    pub waste_type: WasteType,
    pub severity: WasteSeverity,
    pub monthly_waste: f64,
    pub details: String,
}

impl WasteReport {
    pub fn new(vm_name: impl Into<String>, waste_type: WasteType, monthly_waste: f64) -> Self {
        let severity = if monthly_waste > 100.0 {
            WasteSeverity::High
        } else if monthly_waste > 50.0 {
            WasteSeverity::Medium
        } else {
            WasteSeverity::Low
        };

        Self {
            vm_name: vm_name.into(),
            waste_type,
            severity,
            monthly_waste,
            details: String::new(),
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = details.into();
        self
    }
}

/// Waste type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WasteType {
    IdleVM,            // VM with very low utilization
    OversizedVM,       // CPU/memory much higher than needed
    UnattachedStorage, // Storage not attached to any VM
    OldSnapshots,      // Snapshots older than retention policy
    ZombieResources,   // Resources from deleted VMs
}

impl std::fmt::Display for WasteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasteType::IdleVM => write!(f, "Idle VM"),
            WasteType::OversizedVM => write!(f, "Oversized VM"),
            WasteType::UnattachedStorage => write!(f, "Unattached Storage"),
            WasteType::OldSnapshots => write!(f, "Old Snapshots"),
            WasteType::ZombieResources => write!(f, "Zombie Resources"),
        }
    }
}

/// Waste severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasteSeverity {
    Low,
    Medium,
    High,
}

/// Optimization engine
pub struct OptimizationEngine;

impl OptimizationEngine {
    /// Analyze VM for right-sizing opportunities
    pub fn analyze_right_sizing(
        vm_name: &str,
        current_cpu: u32,
        current_memory_gb: u32,
        avg_cpu_usage_percent: f64,
        avg_memory_usage_percent: f64,
        current_monthly_cost: f64,
    ) -> Option<OptimizationRecommendation> {
        // If CPU usage is < 30% and memory < 40%, recommend right-sizing
        if avg_cpu_usage_percent < 30.0 || avg_memory_usage_percent < 40.0 {
            let recommended_cpu =
                ((current_cpu as f64 * avg_cpu_usage_percent / 100.0).ceil() as u32).max(1);
            let recommended_memory = ((current_memory_gb as f64 * avg_memory_usage_percent / 100.0)
                .ceil() as u32)
                .max(1);

            let savings_percent = ((current_cpu - recommended_cpu) as f64 / current_cpu as f64
                + (current_memory_gb - recommended_memory) as f64 / current_memory_gb as f64)
                / 2.0
                * 100.0;
            let potential_savings = current_monthly_cost * (savings_percent / 100.0);

            Some(
                OptimizationRecommendation::new(
                    vm_name,
                    RecommendationType::RightSize,
                    potential_savings,
                )
                .with_priority(Priority::High)
                .with_savings_percent(savings_percent)
                .with_description(format!(
                    "VM is oversized. Reduce from {}C/{}GB to {}C/{}GB",
                    current_cpu, current_memory_gb, recommended_cpu, recommended_memory
                ))
                .add_action(format!(
                    "Reduce CPU from {} to {} cores",
                    current_cpu, recommended_cpu
                ))
                .add_action(format!(
                    "Reduce memory from {}GB to {}GB",
                    current_memory_gb, recommended_memory
                ))
                .with_impact(Impact::Medium),
            )
        } else {
            None
        }
    }

    /// Detect idle VMs
    pub fn detect_idle_vm(
        vm_name: &str,
        avg_cpu_usage_percent: f64,
        days_running: u32,
        monthly_cost: f64,
    ) -> Option<OptimizationRecommendation> {
        // If average CPU < 5% for more than 7 days, consider idle
        if avg_cpu_usage_percent < 5.0 && days_running > 7 {
            Some(
                OptimizationRecommendation::new(
                    vm_name,
                    RecommendationType::Shutdown,
                    monthly_cost,
                )
                .with_priority(Priority::Critical)
                .with_savings_percent(100.0)
                .with_description(format!(
                    "VM has been idle ({}% CPU) for {} days",
                    avg_cpu_usage_percent, days_running
                ))
                .add_action("Verify if VM is still needed")
                .add_action("Shutdown VM if not required")
                .add_action("Consider scheduled start/stop if periodically needed")
                .with_impact(Impact::High),
            )
        } else {
            None
        }
    }

    /// Recommend scheduling for non-production VMs
    pub fn recommend_scheduling(
        vm_name: &str,
        is_production: bool,
        monthly_cost: f64,
    ) -> Option<OptimizationRecommendation> {
        if !is_production {
            // Assume 40% savings by running only business hours (8h/day, 5 days/week)
            let savings_percent = 60.0;
            let potential_savings = monthly_cost * (savings_percent / 100.0);

            Some(
                OptimizationRecommendation::new(
                    vm_name,
                    RecommendationType::Schedule,
                    potential_savings,
                )
                .with_priority(Priority::Medium)
                .with_savings_percent(savings_percent)
                .with_description("Non-production VM running 24/7".to_string())
                .add_action("Schedule VM to run only during business hours")
                .add_action("Auto-shutdown at 6 PM weekdays")
                .add_action("Auto-shutdown on weekends")
                .with_impact(Impact::Low),
            )
        } else {
            None
        }
    }

    /// Detect storage waste
    pub fn detect_storage_waste(storage_gb: u32, monthly_cost: f64) -> WasteReport {
        WasteReport::new("storage", WasteType::UnattachedStorage, monthly_cost).with_details(
            format!(
                "{}GB unattached storage costing ${:.2}/month",
                storage_gb, monthly_cost
            ),
        )
    }

    /// Detect old snapshots
    pub fn detect_old_snapshots(
        snapshot_count: usize,
        days_old: u32,
        monthly_cost: f64,
    ) -> Option<WasteReport> {
        if days_old > 90 && snapshot_count > 0 {
            Some(
                WasteReport::new("snapshots", WasteType::OldSnapshots, monthly_cost).with_details(
                    format!(
                        "{} snapshots older than {} days, costing ${:.2}/month",
                        snapshot_count, days_old, monthly_cost
                    ),
                ),
            )
        } else {
            None
        }
    }

    /// Generate comprehensive optimization report
    pub fn generate_report(vm_name: &str) -> OptimizationReport {
        let mut report = OptimizationReport::new(vm_name);

        // Example recommendations
        if let Some(rec) = Self::analyze_right_sizing(vm_name, 4, 8, 25.0, 35.0, 100.0) {
            report.add_recommendation(rec);
        }

        if let Some(rec) = Self::recommend_scheduling(vm_name, false, 100.0) {
            report.add_recommendation(rec);
        }

        report.calculate_totals();
        report
    }
}

/// Optimization report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub vm_name: String,
    pub generated_at: DateTime<Utc>,
    pub recommendations: Vec<OptimizationRecommendation>,
    pub waste_reports: Vec<WasteReport>,
    pub total_potential_savings: f64,
    pub total_waste: f64,
}

impl OptimizationReport {
    pub fn new(vm_name: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            generated_at: Utc::now(),
            recommendations: Vec::new(),
            waste_reports: Vec::new(),
            total_potential_savings: 0.0,
            total_waste: 0.0,
        }
    }

    pub fn add_recommendation(&mut self, rec: OptimizationRecommendation) {
        self.total_potential_savings += rec.potential_savings;
        self.recommendations.push(rec);
    }

    pub fn add_waste_report(&mut self, report: WasteReport) {
        self.total_waste += report.monthly_waste;
        self.waste_reports.push(report);
    }

    pub fn calculate_totals(&mut self) {
        self.total_potential_savings = self
            .recommendations
            .iter()
            .map(|r| r.potential_savings)
            .sum();

        self.total_waste = self.waste_reports.iter().map(|w| w.monthly_waste).sum();
    }

    pub fn high_priority_recommendations(&self) -> Vec<&OptimizationRecommendation> {
        self.recommendations
            .iter()
            .filter(|r| r.priority >= Priority::High)
            .collect()
    }

    pub fn recommendation_count(&self) -> usize {
        self.recommendations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_recommendation() {
        let rec = OptimizationRecommendation::new("test-vm", RecommendationType::RightSize, 50.0)
            .with_priority(Priority::High)
            .with_description("Reduce resources")
            .add_action("Reduce CPU from 4 to 2")
            .with_impact(Impact::Medium)
            .with_savings_percent(25.0);

        assert_eq!(rec.vm_name, "test-vm");
        assert_eq!(rec.potential_savings, 50.0);
        assert_eq!(rec.priority, Priority::High);
        assert_eq!(rec.action_items.len(), 1);
    }

    #[test]
    fn test_right_sizing_analysis() {
        // VM with 4 cores, 8GB, but only using 25% CPU and 35% memory
        let rec = OptimizationEngine::analyze_right_sizing("oversized-vm", 4, 8, 25.0, 35.0, 100.0);

        assert!(rec.is_some());
        let rec = rec.unwrap();
        assert_eq!(rec.recommendation_type, RecommendationType::RightSize);
        assert!(rec.potential_savings > 0.0);
    }

    #[test]
    fn test_right_sizing_no_recommendation() {
        // VM with good utilization
        let rec = OptimizationEngine::analyze_right_sizing("good-vm", 4, 8, 75.0, 80.0, 100.0);

        assert!(rec.is_none());
    }

    #[test]
    fn test_idle_vm_detection() {
        let rec = OptimizationEngine::detect_idle_vm("idle-vm", 2.0, 10, 100.0);

        assert!(rec.is_some());
        let rec = rec.unwrap();
        assert_eq!(rec.recommendation_type, RecommendationType::Shutdown);
        assert_eq!(rec.priority, Priority::Critical);
        assert_eq!(rec.savings_percent, 100.0);
    }

    #[test]
    fn test_idle_vm_no_detection() {
        // Active VM
        let rec = OptimizationEngine::detect_idle_vm("active-vm", 50.0, 10, 100.0);
        assert!(rec.is_none());

        // Idle but recent
        let rec = OptimizationEngine::detect_idle_vm("recent-vm", 2.0, 5, 100.0);
        assert!(rec.is_none());
    }

    #[test]
    fn test_scheduling_recommendation() {
        let rec = OptimizationEngine::recommend_scheduling("dev-vm", false, 100.0);

        assert!(rec.is_some());
        let rec = rec.unwrap();
        assert_eq!(rec.recommendation_type, RecommendationType::Schedule);
        assert_eq!(rec.savings_percent, 60.0);
    }

    #[test]
    fn test_no_scheduling_for_production() {
        let rec = OptimizationEngine::recommend_scheduling("prod-vm", true, 100.0);
        assert!(rec.is_none());
    }

    #[test]
    fn test_waste_report() {
        let waste = WasteReport::new("test-vm", WasteType::IdleVM, 120.0)
            .with_details("VM idle for 30 days");

        assert_eq!(waste.severity, WasteSeverity::High);
        assert_eq!(waste.monthly_waste, 120.0);
    }

    #[test]
    fn test_waste_severity() {
        let low = WasteReport::new("vm1", WasteType::IdleVM, 30.0);
        assert_eq!(low.severity, WasteSeverity::Low);

        let medium = WasteReport::new("vm2", WasteType::IdleVM, 75.0);
        assert_eq!(medium.severity, WasteSeverity::Medium);

        let high = WasteReport::new("vm3", WasteType::IdleVM, 150.0);
        assert_eq!(high.severity, WasteSeverity::High);
    }

    #[test]
    fn test_old_snapshots_detection() {
        let waste = OptimizationEngine::detect_old_snapshots(5, 120, 25.0);

        assert!(waste.is_some());
        let waste = waste.unwrap();
        assert_eq!(waste.waste_type, WasteType::OldSnapshots);
    }

    #[test]
    fn test_optimization_report() {
        let mut report = OptimizationReport::new("test-vm");

        report.add_recommendation(OptimizationRecommendation::new(
            "test-vm",
            RecommendationType::RightSize,
            50.0,
        ));
        report.add_recommendation(OptimizationRecommendation::new(
            "test-vm",
            RecommendationType::Schedule,
            30.0,
        ));

        report.calculate_totals();

        assert_eq!(report.recommendation_count(), 2);
        assert_eq!(report.total_potential_savings, 80.0);
    }

    #[test]
    fn test_high_priority_recommendations() {
        let mut report = OptimizationReport::new("test-vm");

        report.add_recommendation(
            OptimizationRecommendation::new("test-vm", RecommendationType::RightSize, 50.0)
                .with_priority(Priority::High),
        );
        report.add_recommendation(
            OptimizationRecommendation::new("test-vm", RecommendationType::Schedule, 30.0)
                .with_priority(Priority::Low),
        );

        let high_priority = report.high_priority_recommendations();
        assert_eq!(high_priority.len(), 1);
    }

    #[test]
    fn test_recommendation_type_display() {
        assert_eq!(
            RecommendationType::RightSize.to_string(),
            "Right-size Resources"
        );
        assert_eq!(RecommendationType::Shutdown.to_string(), "Shutdown Idle VM");
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Medium);
        assert!(Priority::Medium > Priority::Low);
    }
}
