use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{ComplianceFramework, ComplianceStatus, Severity};

/// Compliance control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceControl {
    pub id: String,
    pub control_id: String,
    pub name: String,
    pub description: String,
    pub framework: ComplianceFramework,
    pub category: String,
    pub severity: Severity,
    pub status: ComplianceStatus,
    pub implemented: bool,
    pub tested: bool,
    pub test_results: Vec<TestResult>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub timestamp: DateTime<Utc>,
    pub passed: bool,
    pub score: f64,
    pub findings: Vec<String>,
}

impl ComplianceControl {
    pub fn new(
        control_id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        framework: ComplianceFramework,
    ) -> Self {
        let control_id_str = control_id.into();
        let id = format!(
            "ctrl-{}-{}",
            control_id_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            control_id: control_id_str,
            name: name.into(),
            description: description.into(),
            framework,
            category: String::new(),
            severity: Severity::Medium,
            status: ComplianceStatus::UnderReview,
            implemented: false,
            tested: false,
            test_results: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn mark_implemented(&mut self) {
        self.implemented = true;
        self.status = ComplianceStatus::PartiallyCompliant;
    }

    pub fn add_test_result(&mut self, result: TestResult) {
        self.test_results.push(result);
        self.tested = true;

        // Update status based on latest result
        if let Some(latest) = self.test_results.last() {
            if latest.passed {
                self.status = ComplianceStatus::Compliant;
            } else {
                self.status = ComplianceStatus::NonCompliant;
            }
        }
    }

    pub fn is_compliant(&self) -> bool {
        self.status == ComplianceStatus::Compliant
    }

    pub fn latest_score(&self) -> Option<f64> {
        self.test_results.last().map(|r| r.score)
    }
}

impl TestResult {
    pub fn new(passed: bool, score: f64) -> Self {
        Self {
            timestamp: Utc::now(),
            passed,
            score,
            findings: Vec::new(),
        }
    }

    pub fn with_finding(mut self, finding: impl Into<String>) -> Self {
        self.findings.push(finding.into());
        self
    }
}

/// Standards manager
pub struct StandardsManager {
    controls: HashMap<String, ComplianceControl>,
}

impl StandardsManager {
    pub fn new() -> Self {
        Self {
            controls: HashMap::new(),
        }
    }

    pub fn add_control(&mut self, control: ComplianceControl) -> String {
        let id = control.id.clone();
        self.controls.insert(id.clone(), control);
        id
    }

    pub fn get_control(&self, id: &str) -> Option<&ComplianceControl> {
        self.controls.get(id)
    }

    pub fn get_control_mut(&mut self, id: &str) -> Option<&mut ComplianceControl> {
        self.controls.get_mut(id)
    }

    pub fn control_count(&self) -> usize {
        self.controls.len()
    }

    pub fn by_framework(&self, framework: &ComplianceFramework) -> Vec<&ComplianceControl> {
        self.controls
            .values()
            .filter(|c| &c.framework == framework)
            .collect()
    }

    pub fn by_category(&self, category: &str) -> Vec<&ComplianceControl> {
        self.controls
            .values()
            .filter(|c| c.category == category)
            .collect()
    }

    pub fn implemented_controls(&self) -> Vec<&ComplianceControl> {
        self.controls.values().filter(|c| c.implemented).collect()
    }

    pub fn tested_controls(&self) -> Vec<&ComplianceControl> {
        self.controls.values().filter(|c| c.tested).collect()
    }

    pub fn compliant_controls(&self) -> Vec<&ComplianceControl> {
        self.controls
            .values()
            .filter(|c| c.is_compliant())
            .collect()
    }

    pub fn implementation_rate(&self) -> f64 {
        if self.controls.is_empty() {
            return 100.0;
        }

        let implemented = self.implemented_controls().len() as f64;
        let total = self.controls.len() as f64;
        (implemented / total) * 100.0
    }

    pub fn testing_rate(&self) -> f64 {
        if self.controls.is_empty() {
            return 100.0;
        }

        let tested = self.tested_controls().len() as f64;
        let total = self.controls.len() as f64;
        (tested / total) * 100.0
    }
}

impl Default for StandardsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_control() {
        let control = ComplianceControl::new(
            "AC-1",
            "Access Control Policy",
            "Develop and document access control policy",
            ComplianceFramework::NIST,
        );

        assert_eq!(control.control_id, "AC-1");
        assert_eq!(control.name, "Access Control Policy");
        assert_eq!(control.framework, ComplianceFramework::NIST);
        assert!(!control.implemented);
        assert!(!control.tested);
    }

    #[test]
    fn test_control_builder() {
        let control = ComplianceControl::new(
            "AC-2",
            "Account Management",
            "Description",
            ComplianceFramework::NIST,
        )
        .with_category("Access Control")
        .with_severity(Severity::High);

        assert_eq!(control.category, "Access Control");
        assert_eq!(control.severity, Severity::High);
    }

    #[test]
    fn test_control_mark_implemented() {
        let mut control = ComplianceControl::new(
            "AC-3",
            "Access Enforcement",
            "Desc",
            ComplianceFramework::NIST,
        );

        assert!(!control.implemented);
        assert_eq!(control.status, ComplianceStatus::UnderReview);

        control.mark_implemented();

        assert!(control.implemented);
        assert_eq!(control.status, ComplianceStatus::PartiallyCompliant);
    }

    #[test]
    fn test_control_add_test_result_passed() {
        let mut control = ComplianceControl::new(
            "AC-4",
            "Information Flow",
            "Desc",
            ComplianceFramework::NIST,
        );

        control.add_test_result(TestResult::new(true, 95.0));

        assert!(control.tested);
        assert_eq!(control.status, ComplianceStatus::Compliant);
        assert!(control.is_compliant());
        assert_eq!(control.latest_score(), Some(95.0));
    }

    #[test]
    fn test_control_add_test_result_failed() {
        let mut control = ComplianceControl::new(
            "AC-5",
            "Separation of Duties",
            "Desc",
            ComplianceFramework::NIST,
        );

        control.add_test_result(TestResult::new(false, 45.0));

        assert!(control.tested);
        assert_eq!(control.status, ComplianceStatus::NonCompliant);
        assert!(!control.is_compliant());
        assert_eq!(control.latest_score(), Some(45.0));
    }

    #[test]
    fn test_test_result() {
        let result = TestResult::new(true, 90.0);

        assert!(result.passed);
        assert_eq!(result.score, 90.0);
        assert_eq!(result.findings.len(), 0);
    }

    #[test]
    fn test_test_result_with_findings() {
        let result = TestResult::new(false, 60.0)
            .with_finding("Missing MFA configuration")
            .with_finding("Weak password policy");

        assert!(!result.passed);
        assert_eq!(result.findings.len(), 2);
    }

    #[test]
    fn test_standards_manager() {
        let mut manager = StandardsManager::new();

        let control =
            ComplianceControl::new("AC-1", "Access Control", "Desc", ComplianceFramework::NIST);
        let id = manager.add_control(control);

        assert_eq!(manager.control_count(), 1);
        assert!(manager.get_control(&id).is_some());
    }

    #[test]
    fn test_manager_by_framework() {
        let mut manager = StandardsManager::new();

        manager.add_control(ComplianceControl::new(
            "AC-1",
            "AC1",
            "D",
            ComplianceFramework::NIST,
        ));
        manager.add_control(ComplianceControl::new(
            "CC1",
            "CC1",
            "D",
            ComplianceFramework::SOC2,
        ));
        manager.add_control(ComplianceControl::new(
            "AC-2",
            "AC2",
            "D",
            ComplianceFramework::NIST,
        ));

        let nist = manager.by_framework(&ComplianceFramework::NIST);
        assert_eq!(nist.len(), 2);
    }

    #[test]
    fn test_manager_by_category() {
        let mut manager = StandardsManager::new();

        manager.add_control(
            ComplianceControl::new("AC-1", "AC1", "D", ComplianceFramework::NIST)
                .with_category("Access Control"),
        );
        manager.add_control(
            ComplianceControl::new("AU-1", "AU1", "D", ComplianceFramework::NIST)
                .with_category("Audit"),
        );
        manager.add_control(
            ComplianceControl::new("AC-2", "AC2", "D", ComplianceFramework::NIST)
                .with_category("Access Control"),
        );

        let access_control = manager.by_category("Access Control");
        assert_eq!(access_control.len(), 2);
    }

    #[test]
    fn test_manager_implemented_controls() {
        let mut manager = StandardsManager::new();

        let mut control1 = ComplianceControl::new("AC-1", "AC1", "D", ComplianceFramework::NIST);
        control1.mark_implemented();

        let mut control2 = ComplianceControl::new("AC-2", "AC2", "D", ComplianceFramework::NIST);
        control2.mark_implemented();

        let control3 = ComplianceControl::new("AC-3", "AC3", "D", ComplianceFramework::NIST);

        manager.add_control(control1);
        manager.add_control(control2);
        manager.add_control(control3);

        let implemented = manager.implemented_controls();
        assert_eq!(implemented.len(), 2);
    }

    #[test]
    fn test_manager_tested_controls() {
        let mut manager = StandardsManager::new();

        let mut control1 = ComplianceControl::new("AC-1", "AC1", "D", ComplianceFramework::NIST);
        control1.add_test_result(TestResult::new(true, 90.0));

        let control2 = ComplianceControl::new("AC-2", "AC2", "D", ComplianceFramework::NIST);

        manager.add_control(control1);
        manager.add_control(control2);

        let tested = manager.tested_controls();
        assert_eq!(tested.len(), 1);
    }

    #[test]
    fn test_manager_compliant_controls() {
        let mut manager = StandardsManager::new();

        let mut control1 = ComplianceControl::new("AC-1", "AC1", "D", ComplianceFramework::NIST);
        control1.add_test_result(TestResult::new(true, 90.0));

        let mut control2 = ComplianceControl::new("AC-2", "AC2", "D", ComplianceFramework::NIST);
        control2.add_test_result(TestResult::new(false, 50.0));

        manager.add_control(control1);
        manager.add_control(control2);

        let compliant = manager.compliant_controls();
        assert_eq!(compliant.len(), 1);
    }

    #[test]
    fn test_manager_implementation_rate() {
        let mut manager = StandardsManager::new();

        let mut control1 = ComplianceControl::new("AC-1", "AC1", "D", ComplianceFramework::NIST);
        control1.mark_implemented();

        let mut control2 = ComplianceControl::new("AC-2", "AC2", "D", ComplianceFramework::NIST);
        control2.mark_implemented();

        let control3 = ComplianceControl::new("AC-3", "AC3", "D", ComplianceFramework::NIST);
        let control4 = ComplianceControl::new("AC-4", "AC4", "D", ComplianceFramework::NIST);

        manager.add_control(control1);
        manager.add_control(control2);
        manager.add_control(control3);
        manager.add_control(control4);

        assert_eq!(manager.implementation_rate(), 50.0);
    }

    #[test]
    fn test_manager_testing_rate() {
        let mut manager = StandardsManager::new();

        let mut control1 = ComplianceControl::new("AC-1", "AC1", "D", ComplianceFramework::NIST);
        control1.add_test_result(TestResult::new(true, 90.0));

        let mut control2 = ComplianceControl::new("AC-2", "AC2", "D", ComplianceFramework::NIST);
        control2.add_test_result(TestResult::new(true, 95.0));

        let mut control3 = ComplianceControl::new("AC-3", "AC3", "D", ComplianceFramework::NIST);
        control3.add_test_result(TestResult::new(false, 60.0));

        let control4 = ComplianceControl::new("AC-4", "AC4", "D", ComplianceFramework::NIST);

        manager.add_control(control1);
        manager.add_control(control2);
        manager.add_control(control3);
        manager.add_control(control4);

        assert_eq!(manager.testing_rate(), 75.0);
    }

    #[test]
    fn test_manager_empty_rates() {
        let manager = StandardsManager::new();

        assert_eq!(manager.implementation_rate(), 100.0);
        assert_eq!(manager.testing_rate(), 100.0);
    }

    #[test]
    fn test_manager_get_control_mut() {
        let mut manager = StandardsManager::new();

        let control = ComplianceControl::new("AC-1", "AC1", "D", ComplianceFramework::NIST);
        let id = manager.add_control(control);

        if let Some(control_mut) = manager.get_control_mut(&id) {
            control_mut.mark_implemented();
        }

        let control = manager.get_control(&id).unwrap();
        assert!(control.implemented);
    }
}
