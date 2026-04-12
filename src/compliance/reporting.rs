use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ComplianceFramework;

/// Compliance report type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    Summary,
    Detailed,
    Executive,
    Technical,
    AuditReadiness,
}

/// Compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub id: String,
    pub name: String,
    pub report_type: ReportType,
    pub framework: ComplianceFramework,
    pub generated_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub overall_score: f64,
    pub sections: Vec<ReportSection>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub summary: String,
    pub metrics: HashMap<String, f64>,
    pub findings: Vec<String>,
}

impl ComplianceReport {
    pub fn new(
        name: impl Into<String>,
        report_type: ReportType,
        framework: ComplianceFramework,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "report-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            report_type,
            framework,
            generated_at: Utc::now(),
            period_start,
            period_end,
            overall_score: 0.0,
            sections: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn with_score(mut self, score: f64) -> Self {
        self.overall_score = score;
        self
    }

    pub fn add_section(&mut self, section: ReportSection) {
        self.sections.push(section);
    }

    pub fn add_recommendation(&mut self, recommendation: impl Into<String>) {
        self.recommendations.push(recommendation.into());
    }

    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    pub fn is_passing(&self) -> bool {
        self.overall_score >= 80.0
    }
}

impl ReportSection {
    pub fn new(title: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            summary: summary.into(),
            metrics: HashMap::new(),
            findings: Vec::new(),
        }
    }

    pub fn add_metric(&mut self, key: impl Into<String>, value: f64) {
        self.metrics.insert(key.into(), value);
    }

    pub fn add_finding(&mut self, finding: impl Into<String>) {
        self.findings.push(finding.into());
    }
}

/// Report generator
pub struct ReportGenerator {
    reports: HashMap<String, ComplianceReport>,
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            reports: HashMap::new(),
        }
    }

    pub fn add_report(&mut self, report: ComplianceReport) -> String {
        let id = report.id.clone();
        self.reports.insert(id.clone(), report);
        id
    }

    pub fn get_report(&self, id: &str) -> Option<&ComplianceReport> {
        self.reports.get(id)
    }

    pub fn report_count(&self) -> usize {
        self.reports.len()
    }

    pub fn by_framework(&self, framework: &ComplianceFramework) -> Vec<&ComplianceReport> {
        self.reports
            .values()
            .filter(|r| &r.framework == framework)
            .collect()
    }

    pub fn by_type(&self, report_type: &ReportType) -> Vec<&ComplianceReport> {
        self.reports
            .values()
            .filter(|r| &r.report_type == report_type)
            .collect()
    }

    pub fn passing_reports(&self) -> Vec<&ComplianceReport> {
        self.reports.values().filter(|r| r.is_passing()).collect()
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_report() {
        let start = Utc::now() - chrono::TimeDelta::days(30);
        let end = Utc::now();

        let report = ComplianceReport::new(
            "Q1 Compliance Report",
            ReportType::Summary,
            ComplianceFramework::SOC2,
            start,
            end,
        );

        assert_eq!(report.name, "Q1 Compliance Report");
        assert_eq!(report.report_type, ReportType::Summary);
        assert_eq!(report.framework, ComplianceFramework::SOC2);
        assert_eq!(report.overall_score, 0.0);
        assert!(!report.is_passing());
    }

    #[test]
    fn test_report_builder() {
        let start = Utc::now() - chrono::TimeDelta::days(30);
        let end = Utc::now();

        let report = ComplianceReport::new(
            "Test Report",
            ReportType::Detailed,
            ComplianceFramework::HIPAA,
            start,
            end,
        )
        .with_score(85.0);

        assert_eq!(report.overall_score, 85.0);
        assert!(report.is_passing());
    }

    #[test]
    fn test_report_add_section() {
        let start = Utc::now() - chrono::TimeDelta::days(30);
        let end = Utc::now();

        let mut report = ComplianceReport::new(
            "Test",
            ReportType::Summary,
            ComplianceFramework::GDPR,
            start,
            end,
        );

        report.add_section(ReportSection::new(
            "Access Control",
            "Summary of access controls",
        ));
        report.add_section(ReportSection::new(
            "Data Protection",
            "Summary of data protection",
        ));

        assert_eq!(report.section_count(), 2);
    }

    #[test]
    fn test_report_add_recommendation() {
        let start = Utc::now() - chrono::TimeDelta::days(30);
        let end = Utc::now();

        let mut report = ComplianceReport::new(
            "Test",
            ReportType::Summary,
            ComplianceFramework::PciDss,
            start,
            end,
        );

        report.add_recommendation("Enable MFA for all users");
        report.add_recommendation("Implement regular security audits");

        assert_eq!(report.recommendations.len(), 2);
    }

    #[test]
    fn test_report_section() {
        let section = ReportSection::new("Security Controls", "Overview of security controls");

        assert_eq!(section.title, "Security Controls");
        assert_eq!(section.summary, "Overview of security controls");
        assert_eq!(section.metrics.len(), 0);
        assert_eq!(section.findings.len(), 0);
    }

    #[test]
    fn test_section_add_metric() {
        let mut section = ReportSection::new("Metrics", "Summary");

        section.add_metric("compliance_score", 85.5);
        section.add_metric("implementation_rate", 90.0);

        assert_eq!(section.metrics.len(), 2);
        assert_eq!(section.metrics.get("compliance_score"), Some(&85.5));
    }

    #[test]
    fn test_section_add_finding() {
        let mut section = ReportSection::new("Findings", "Summary");

        section.add_finding("Missing encryption on volume A");
        section.add_finding("Weak password policy detected");

        assert_eq!(section.findings.len(), 2);
    }

    #[test]
    fn test_report_generator() {
        let mut generator = ReportGenerator::new();

        let start = Utc::now() - chrono::TimeDelta::days(30);
        let end = Utc::now();

        let report = ComplianceReport::new(
            "Test",
            ReportType::Summary,
            ComplianceFramework::SOC2,
            start,
            end,
        );
        let id = generator.add_report(report);

        assert_eq!(generator.report_count(), 1);
        assert!(generator.get_report(&id).is_some());
    }

    #[test]
    fn test_generator_by_framework() {
        let mut generator = ReportGenerator::new();

        let start = Utc::now() - chrono::TimeDelta::days(30);
        let end = Utc::now();

        generator.add_report(ComplianceReport::new(
            "R1",
            ReportType::Summary,
            ComplianceFramework::SOC2,
            start,
            end,
        ));
        generator.add_report(ComplianceReport::new(
            "R2",
            ReportType::Detailed,
            ComplianceFramework::HIPAA,
            start,
            end,
        ));
        generator.add_report(ComplianceReport::new(
            "R3",
            ReportType::Summary,
            ComplianceFramework::SOC2,
            start,
            end,
        ));

        let soc2 = generator.by_framework(&ComplianceFramework::SOC2);
        assert_eq!(soc2.len(), 2);
    }

    #[test]
    fn test_generator_by_type() {
        let mut generator = ReportGenerator::new();

        let start = Utc::now() - chrono::TimeDelta::days(30);
        let end = Utc::now();

        generator.add_report(ComplianceReport::new(
            "R1",
            ReportType::Summary,
            ComplianceFramework::SOC2,
            start,
            end,
        ));
        generator.add_report(ComplianceReport::new(
            "R2",
            ReportType::Detailed,
            ComplianceFramework::HIPAA,
            start,
            end,
        ));
        generator.add_report(ComplianceReport::new(
            "R3",
            ReportType::Summary,
            ComplianceFramework::GDPR,
            start,
            end,
        ));

        let summary = generator.by_type(&ReportType::Summary);
        assert_eq!(summary.len(), 2);
    }

    #[test]
    fn test_generator_passing_reports() {
        let mut generator = ReportGenerator::new();

        let start = Utc::now() - chrono::TimeDelta::days(30);
        let end = Utc::now();

        generator.add_report(
            ComplianceReport::new(
                "R1",
                ReportType::Summary,
                ComplianceFramework::SOC2,
                start,
                end,
            )
            .with_score(85.0),
        );
        generator.add_report(
            ComplianceReport::new(
                "R2",
                ReportType::Summary,
                ComplianceFramework::HIPAA,
                start,
                end,
            )
            .with_score(75.0),
        );
        generator.add_report(
            ComplianceReport::new(
                "R3",
                ReportType::Summary,
                ComplianceFramework::GDPR,
                start,
                end,
            )
            .with_score(90.0),
        );

        let passing = generator.passing_reports();
        assert_eq!(passing.len(), 2);
    }

    #[test]
    fn test_report_type_equality() {
        assert_eq!(ReportType::Summary, ReportType::Summary);
        assert_ne!(ReportType::Summary, ReportType::Detailed);
    }
}
