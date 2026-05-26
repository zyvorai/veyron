// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Security & Compliance - VM security management and compliance checking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod audit;
pub mod compliance;
pub mod hardening;
pub mod posture;
pub mod rbac_visualizer;
pub mod scan;
pub mod vulnerability;

/// Security assessment for a VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAssessment {
    pub vm_name: String,
    pub assessed_at: DateTime<Utc>,
    pub overall_score: u8, // 0-100
    pub risk_level: RiskLevel,
    pub vulnerabilities: Vec<Vulnerability>,
    pub compliance_status: ComplianceStatus,
    pub recommendations: Vec<SecurityRecommendation>,
}

impl SecurityAssessment {
    pub fn new(vm_name: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            assessed_at: Utc::now(),
            overall_score: 0,
            risk_level: RiskLevel::Unknown,
            vulnerabilities: Vec::new(),
            compliance_status: ComplianceStatus::default(),
            recommendations: Vec::new(),
        }
    }

    pub fn calculate_score(&mut self) {
        let critical_vulns = self
            .vulnerabilities
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .count();
        let high_vulns = self
            .vulnerabilities
            .iter()
            .filter(|v| v.severity == Severity::High)
            .count();
        let medium_vulns = self
            .vulnerabilities
            .iter()
            .filter(|v| v.severity == Severity::Medium)
            .count();

        // Calculate score: start at 100, deduct points for vulnerabilities
        let mut score: i32 = 100;
        score = score.saturating_sub((critical_vulns as i32).saturating_mul(20));
        score = score.saturating_sub((high_vulns as i32).saturating_mul(10));
        score = score.saturating_sub((medium_vulns as i32).saturating_mul(5));

        self.overall_score = score.max(0) as u8;

        self.risk_level = match self.overall_score {
            90..=100 => RiskLevel::Low,
            70..=89 => RiskLevel::Medium,
            50..=69 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };
    }

    pub fn add_vulnerability(&mut self, vuln: Vulnerability) {
        self.vulnerabilities.push(vuln);
    }

    pub fn add_recommendation(&mut self, rec: SecurityRecommendation) {
        self.recommendations.push(rec);
    }

    pub fn critical_count(&self) -> usize {
        self.vulnerabilities
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .count()
    }

    pub fn high_count(&self) -> usize {
        self.vulnerabilities
            .iter()
            .filter(|v| v.severity == Severity::High)
            .count()
    }
}

/// Risk level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
    Unknown,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Critical => write!(f, "Critical"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Vulnerability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub cvss_score: f32,
    pub cve_id: Option<String>,
    pub affected_package: Option<String>,
    pub fixed_version: Option<String>,
    pub discovered_at: DateTime<Utc>,
}

impl Vulnerability {
    pub fn new(id: impl Into<String>, title: impl Into<String>, severity: Severity) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            severity,
            cvss_score: 0.0,
            cve_id: None,
            affected_package: None,
            fixed_version: None,
            discovered_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_cvss(mut self, score: f32) -> Self {
        self.cvss_score = score;
        self
    }

    pub fn with_cve(mut self, cve: impl Into<String>) -> Self {
        self.cve_id = Some(cve.into());
        self
    }

    pub fn with_package(mut self, package: impl Into<String>, fixed: Option<String>) -> Self {
        self.affected_package = Some(package.into());
        self.fixed_version = fixed;
        self
    }
}

/// Severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "Critical"),
            Severity::High => write!(f, "High"),
            Severity::Medium => write!(f, "Medium"),
            Severity::Low => write!(f, "Low"),
            Severity::Info => write!(f, "Info"),
        }
    }
}

/// Security recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRecommendation {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub category: RecommendationCategory,
    pub remediation_steps: Vec<String>,
}

impl SecurityRecommendation {
    pub fn new(
        title: impl Into<String>,
        category: RecommendationCategory,
        priority: Priority,
    ) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            priority,
            category,
            remediation_steps: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn add_step(mut self, step: impl Into<String>) -> Self {
        self.remediation_steps.push(step.into());
        self
    }
}

/// Recommendation category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationCategory {
    Patching,
    Configuration,
    AccessControl,
    Network,
    Encryption,
    Monitoring,
}

/// Priority level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

/// Compliance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub frameworks: Vec<ComplianceFramework>,
    pub overall_compliant: bool,
    pub compliance_percentage: u8,
}

impl ComplianceStatus {
    pub fn new() -> Self {
        Self {
            frameworks: Vec::new(),
            overall_compliant: false,
            compliance_percentage: 0,
        }
    }

    pub fn add_framework(&mut self, framework: ComplianceFramework) {
        self.frameworks.push(framework);
        self.calculate_overall();
    }

    fn calculate_overall(&mut self) {
        if self.frameworks.is_empty() {
            return;
        }

        let total_controls: usize = self.frameworks.iter().map(|f| f.total_controls).sum();
        let passed_controls: usize = self.frameworks.iter().map(|f| f.passed_controls).sum();

        self.compliance_percentage = if total_controls > 0 {
            ((passed_controls as f64 / total_controls as f64) * 100.0).round() as u8
        } else {
            0
        };

        self.overall_compliant = self.frameworks.iter().all(|f| f.compliant);
    }
}

impl Default for ComplianceStatus {
    fn default() -> Self {
        Self::new()
    }
}

/// Compliance framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFramework {
    pub name: String,
    pub version: String,
    pub total_controls: usize,
    pub passed_controls: usize,
    pub failed_controls: usize,
    pub compliant: bool,
}

impl ComplianceFramework {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            total_controls: 0,
            passed_controls: 0,
            failed_controls: 0,
            compliant: false,
        }
    }

    pub fn with_results(mut self, total: usize, passed: usize, failed: usize) -> Self {
        if passed + failed > total {
            log::warn!(
                "ComplianceFramework '{}': passed ({}) + failed ({}) exceeds total ({}); clamping",
                self.name,
                passed,
                failed,
                total
            );
            self.total_controls = passed + failed;
        } else {
            self.total_controls = total;
        }
        self.passed_controls = passed;
        self.failed_controls = failed;
        self.compliant = failed == 0;
        self
    }

    pub fn compliance_rate(&self) -> f64 {
        if self.total_controls == 0 {
            return 0.0;
        }
        (self.passed_controls as f64 / self.total_controls as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_assessment() {
        let mut assessment = SecurityAssessment::new("test-vm");

        assessment.add_vulnerability(
            Vulnerability::new("VULN-001", "Critical vulnerability", Severity::Critical)
                .with_cvss(9.8),
        );

        assessment.calculate_score();

        assert_eq!(assessment.overall_score, 80); // 100 - 20
        assert_eq!(assessment.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_vulnerability() {
        let vuln = Vulnerability::new("CVE-2024-001", "SQL Injection", Severity::High)
            .with_description("Critical SQL injection vulnerability")
            .with_cvss(8.5)
            .with_cve("CVE-2024-001")
            .with_package("postgresql", Some("14.5".to_string()));

        assert_eq!(vuln.severity, Severity::High);
        assert_eq!(vuln.cvss_score, 8.5);
        assert_eq!(vuln.cve_id, Some("CVE-2024-001".to_string()));
    }

    #[test]
    fn test_risk_level_calculation() {
        let mut assessment = SecurityAssessment::new("test-vm");

        // Low risk
        assessment.overall_score = 95;
        assessment.calculate_score();

        // Add critical vulnerability
        assessment.add_vulnerability(Vulnerability::new("V1", "Critical", Severity::Critical));
        assessment.calculate_score();
        assert_eq!(assessment.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }

    #[test]
    fn test_compliance_status() {
        let mut status = ComplianceStatus::new();

        let framework = ComplianceFramework::new("PCI-DSS", "4.0").with_results(100, 95, 5);

        status.add_framework(framework);

        assert_eq!(status.compliance_percentage, 95);
        assert!(!status.overall_compliant);
    }

    #[test]
    fn test_compliance_framework() {
        let framework = ComplianceFramework::new("CIS", "1.0").with_results(50, 50, 0);

        assert!(framework.compliant);
        assert_eq!(framework.compliance_rate(), 100.0);
    }

    #[test]
    fn test_security_recommendation() {
        let rec = SecurityRecommendation::new(
            "Update OpenSSL",
            RecommendationCategory::Patching,
            Priority::Critical,
        )
        .with_description("OpenSSL has critical vulnerabilities")
        .add_step("Update to version 3.0.8")
        .add_step("Restart affected services");

        assert_eq!(rec.priority, Priority::Critical);
        assert_eq!(rec.remediation_steps.len(), 2);
    }

    #[test]
    fn test_vulnerability_counts() {
        let mut assessment = SecurityAssessment::new("test-vm");

        assessment.add_vulnerability(Vulnerability::new("V1", "Critical", Severity::Critical));
        assessment.add_vulnerability(Vulnerability::new("V2", "High", Severity::High));
        assessment.add_vulnerability(Vulnerability::new("V3", "High", Severity::High));

        assert_eq!(assessment.critical_count(), 1);
        assert_eq!(assessment.high_count(), 2);
    }
}
