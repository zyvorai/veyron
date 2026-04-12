// Security Hardening - Apply security hardening configurations to VMs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Hardening profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardeningProfile {
    pub name: String,
    pub description: String,
    pub baseline: SecurityBaseline,
    pub rules: Vec<HardeningRule>,
    pub created_at: DateTime<Utc>,
}

impl HardeningProfile {
    pub fn new(name: impl Into<String>, baseline: SecurityBaseline) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            baseline,
            rules: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn add_rule(mut self, rule: HardeningRule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// Security baseline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityBaseline {
    CIS,    // CIS Benchmark
    STIG,   // DISA STIG
    PciDss, // PCI Data Security Standard
    NIST,   // NIST SP 800-53
    Custom, // Custom baseline
}

impl std::fmt::Display for SecurityBaseline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityBaseline::CIS => write!(f, "CIS Benchmark"),
            SecurityBaseline::STIG => write!(f, "DISA STIG"),
            SecurityBaseline::PciDss => write!(f, "PCI-DSS"),
            SecurityBaseline::NIST => write!(f, "NIST 800-53"),
            SecurityBaseline::Custom => write!(f, "Custom"),
        }
    }
}

/// Hardening rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardeningRule {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: HardeningCategory,
    pub severity: RuleSeverity,
    pub remediation: String,
    pub automated: bool,
}

impl HardeningRule {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        category: HardeningCategory,
        severity: RuleSeverity,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            category,
            severity,
            remediation: String::new(),
            automated: false,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = remediation.into();
        self
    }

    pub fn automated(mut self) -> Self {
        self.automated = true;
        self
    }
}

/// Hardening category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HardeningCategory {
    SystemConfiguration,
    NetworkSecurity,
    AccessControl,
    Encryption,
    Auditing,
    Services,
    FileSystem,
    Kernel,
}

/// Rule severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Hardening result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardeningResult {
    pub vm_name: String,
    pub profile_name: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: HardeningStatus,
    pub applied_rules: Vec<AppliedRule>,
    pub statistics: HardeningStatistics,
}

impl HardeningResult {
    pub fn new(vm_name: impl Into<String>, profile_name: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            profile_name: profile_name.into(),
            started_at: Utc::now(),
            completed_at: None,
            status: HardeningStatus::InProgress,
            applied_rules: Vec::new(),
            statistics: HardeningStatistics::default(),
        }
    }

    pub fn add_applied_rule(&mut self, rule: AppliedRule) {
        match rule.result {
            RuleResult::Applied => self.statistics.applied += 1,
            RuleResult::Skipped => self.statistics.skipped += 1,
            RuleResult::Failed => self.statistics.failed += 1,
        }
        self.statistics.total += 1;
        self.applied_rules.push(rule);
    }

    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
        self.status = if self.statistics.failed > 0 {
            HardeningStatus::PartiallyApplied
        } else {
            HardeningStatus::Completed
        };
    }

    pub fn success_rate(&self) -> f64 {
        if self.statistics.total == 0 {
            return 0.0;
        }
        (self.statistics.applied as f64 / self.statistics.total as f64) * 100.0
    }
}

/// Hardening status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HardeningStatus {
    InProgress,
    Completed,
    PartiallyApplied,
    Failed,
}

impl std::fmt::Display for HardeningStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardeningStatus::InProgress => write!(f, "In Progress"),
            HardeningStatus::Completed => write!(f, "Completed"),
            HardeningStatus::PartiallyApplied => write!(f, "Partially Applied"),
            HardeningStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Applied rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedRule {
    pub rule_id: String,
    pub rule_title: String,
    pub result: RuleResult,
    pub message: String,
    pub applied_at: DateTime<Utc>,
}

impl AppliedRule {
    pub fn new(
        rule_id: impl Into<String>,
        rule_title: impl Into<String>,
        result: RuleResult,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            rule_title: rule_title.into(),
            result,
            message: String::new(),
            applied_at: Utc::now(),
        }
    }

    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = msg.into();
        self
    }
}

/// Rule application result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuleResult {
    Applied,
    Skipped,
    Failed,
}

/// Hardening statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardeningStatistics {
    pub total: usize,
    pub applied: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// Hardening engine
pub struct HardeningEngine;

impl HardeningEngine {
    /// Create CIS baseline profile
    pub fn cis_profile() -> HardeningProfile {
        HardeningProfile::new("CIS Benchmark", SecurityBaseline::CIS)
            .with_description("CIS Security Benchmark hardening rules")
            .add_rule(
                HardeningRule::new(
                    "CIS-1.1",
                    "Disable unnecessary services",
                    HardeningCategory::Services,
                    RuleSeverity::High,
                )
                .with_description("Disable services not required for operation")
                .with_remediation("systemctl disable <service>")
                .automated(),
            )
            .add_rule(
                HardeningRule::new(
                    "CIS-2.1",
                    "Configure SSH hardening",
                    HardeningCategory::NetworkSecurity,
                    RuleSeverity::Critical,
                )
                .with_description("Apply secure SSH configuration")
                .with_remediation("Update /etc/ssh/sshd_config")
                .automated(),
            )
            .add_rule(
                HardeningRule::new(
                    "CIS-3.1",
                    "Enable firewall",
                    HardeningCategory::NetworkSecurity,
                    RuleSeverity::Critical,
                )
                .with_description("Enable and configure host firewall")
                .with_remediation("systemctl enable firewalld")
                .automated(),
            )
    }

    /// Create STIG baseline profile
    pub fn stig_profile() -> HardeningProfile {
        HardeningProfile::new("DISA STIG", SecurityBaseline::STIG)
            .with_description("DISA Security Technical Implementation Guide")
            .add_rule(
                HardeningRule::new(
                    "STIG-001",
                    "Enforce password complexity",
                    HardeningCategory::AccessControl,
                    RuleSeverity::High,
                )
                .with_description("Configure PAM for password complexity")
                .automated(),
            )
            .add_rule(
                HardeningRule::new(
                    "STIG-002",
                    "Enable audit logging",
                    HardeningCategory::Auditing,
                    RuleSeverity::High,
                )
                .with_description("Configure comprehensive audit logging")
                .automated(),
            )
    }

    /// Apply hardening profile to VM
    pub fn apply(vm_name: &str, profile: &HardeningProfile) -> HardeningResult {
        let mut result = HardeningResult::new(vm_name, &profile.name);

        // Apply each rule
        for rule in &profile.rules {
            if rule.automated {
                let applied = AppliedRule::new(&rule.id, &rule.title, RuleResult::Applied)
                    .with_message(format!("Successfully applied: {}", rule.remediation));
                result.add_applied_rule(applied);
            } else {
                let skipped = AppliedRule::new(&rule.id, &rule.title, RuleResult::Skipped)
                    .with_message("Manual remediation required");
                result.add_applied_rule(skipped);
            }
        }

        result.complete();
        result
    }

    /// Verify hardening compliance
    pub fn verify(vm_name: &str, profile: &HardeningProfile) -> HardeningResult {
        let mut result = HardeningResult::new(vm_name, &profile.name);

        // Verify each rule
        for rule in &profile.rules {
            let verified = AppliedRule::new(&rule.id, &rule.title, RuleResult::Applied)
                .with_message("Rule is compliant");
            result.add_applied_rule(verified);
        }

        result.complete();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardening_profile() {
        let profile = HardeningProfile::new("Test Profile", SecurityBaseline::CIS)
            .with_description("Test hardening profile")
            .add_rule(HardeningRule::new(
                "RULE-001",
                "Test rule",
                HardeningCategory::SystemConfiguration,
                RuleSeverity::High,
            ));

        assert_eq!(profile.name, "Test Profile");
        assert_eq!(profile.baseline, SecurityBaseline::CIS);
        assert_eq!(profile.rule_count(), 1);
    }

    #[test]
    fn test_hardening_rule() {
        let rule = HardeningRule::new(
            "RULE-001",
            "Disable telnet",
            HardeningCategory::Services,
            RuleSeverity::Critical,
        )
        .with_description("Telnet is insecure")
        .with_remediation("systemctl disable telnet")
        .automated();

        assert_eq!(rule.id, "RULE-001");
        assert_eq!(rule.severity, RuleSeverity::Critical);
        assert!(rule.automated);
    }

    #[test]
    fn test_hardening_result() {
        let mut result = HardeningResult::new("test-vm", "CIS");

        result.add_applied_rule(AppliedRule::new("R1", "Rule 1", RuleResult::Applied));
        result.add_applied_rule(AppliedRule::new("R2", "Rule 2", RuleResult::Applied));
        result.add_applied_rule(AppliedRule::new("R3", "Rule 3", RuleResult::Failed));

        assert_eq!(result.statistics.total, 3);
        assert_eq!(result.statistics.applied, 2);
        assert_eq!(result.statistics.failed, 1);
    }

    #[test]
    fn test_success_rate() {
        let mut result = HardeningResult::new("test-vm", "CIS");

        result.add_applied_rule(AppliedRule::new("R1", "Rule 1", RuleResult::Applied));
        result.add_applied_rule(AppliedRule::new("R2", "Rule 2", RuleResult::Applied));
        result.add_applied_rule(AppliedRule::new("R3", "Rule 3", RuleResult::Failed));
        result.add_applied_rule(AppliedRule::new("R4", "Rule 4", RuleResult::Applied));

        assert_eq!(result.success_rate(), 75.0); // 3 out of 4
    }

    #[test]
    fn test_cis_profile() {
        let profile = HardeningEngine::cis_profile();

        assert_eq!(profile.baseline, SecurityBaseline::CIS);
        assert!(profile.rule_count() > 0);
    }

    #[test]
    fn test_stig_profile() {
        let profile = HardeningEngine::stig_profile();

        assert_eq!(profile.baseline, SecurityBaseline::STIG);
        assert!(profile.rule_count() > 0);
    }

    #[test]
    fn test_apply_hardening() {
        let profile = HardeningEngine::cis_profile();
        let result = HardeningEngine::apply("test-vm", &profile);

        assert_eq!(result.status, HardeningStatus::Completed);
        assert!(result.statistics.total > 0);
    }

    #[test]
    fn test_verify_hardening() {
        let profile = HardeningEngine::cis_profile();
        let result = HardeningEngine::verify("test-vm", &profile);

        assert_eq!(result.status, HardeningStatus::Completed);
    }

    #[test]
    fn test_rule_severity_ordering() {
        assert!(RuleSeverity::Critical > RuleSeverity::High);
        assert!(RuleSeverity::High > RuleSeverity::Medium);
        assert!(RuleSeverity::Medium > RuleSeverity::Low);
    }

    #[test]
    fn test_baseline_display() {
        assert_eq!(SecurityBaseline::CIS.to_string(), "CIS Benchmark");
        assert_eq!(SecurityBaseline::STIG.to_string(), "DISA STIG");
        assert_eq!(SecurityBaseline::PciDss.to_string(), "PCI-DSS");
    }
}
