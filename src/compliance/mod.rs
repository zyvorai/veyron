use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod audit;
pub mod policies;
pub mod reporting;
pub mod scanning;
pub mod standards;

/// Compliance framework
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceFramework {
    SOC2,
    HIPAA,
    PciDss,
    GDPR,
    ISO27001,
    NIST,
    CIS,
    FedRAMP,
    Custom(String),
}

impl std::fmt::Display for ComplianceFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplianceFramework::SOC2 => write!(f, "SOC 2"),
            ComplianceFramework::HIPAA => write!(f, "HIPAA"),
            ComplianceFramework::PciDss => write!(f, "PCI DSS"),
            ComplianceFramework::GDPR => write!(f, "GDPR"),
            ComplianceFramework::ISO27001 => write!(f, "ISO 27001"),
            ComplianceFramework::NIST => write!(f, "NIST"),
            ComplianceFramework::CIS => write!(f, "CIS"),
            ComplianceFramework::FedRAMP => write!(f, "FedRAMP"),
            ComplianceFramework::Custom(name) => write!(f, "Custom: {}", name),
        }
    }
}

/// Compliance status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Compliant,
    NonCompliant,
    PartiallyCompliant,
    NotApplicable,
    UnderReview,
}

impl std::fmt::Display for ComplianceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplianceStatus::Compliant => write!(f, "Compliant"),
            ComplianceStatus::NonCompliant => write!(f, "Non-Compliant"),
            ComplianceStatus::PartiallyCompliant => write!(f, "Partially Compliant"),
            ComplianceStatus::NotApplicable => write!(f, "Not Applicable"),
            ComplianceStatus::UnderReview => write!(f, "Under Review"),
        }
    }
}

pub use crate::security::Severity;

/// Compliance requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRequirement {
    pub id: String,
    pub name: String,
    pub framework: ComplianceFramework,
    pub requirement_id: String,
    pub description: String,
    pub severity: Severity,
    pub status: ComplianceStatus,
    pub control_objectives: Vec<String>,
    pub evidence_required: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_assessed: Option<DateTime<Utc>>,
}

impl ComplianceRequirement {
    pub fn new(
        name: impl Into<String>,
        framework: ComplianceFramework,
        requirement_id: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "req-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            framework,
            requirement_id: requirement_id.into(),
            description: description.into(),
            severity: Severity::Medium,
            status: ComplianceStatus::UnderReview,
            control_objectives: Vec::new(),
            evidence_required: Vec::new(),
            created_at: Utc::now(),
            last_assessed: None,
        }
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_status(mut self, status: ComplianceStatus) -> Self {
        self.status = status;
        self
    }

    pub fn add_control_objective(&mut self, objective: impl Into<String>) {
        self.control_objectives.push(objective.into());
    }

    pub fn add_evidence(&mut self, evidence: impl Into<String>) {
        self.evidence_required.push(evidence.into());
    }

    pub fn update_status(&mut self, status: ComplianceStatus) {
        self.status = status;
        self.last_assessed = Some(Utc::now());
    }

    pub fn is_compliant(&self) -> bool {
        self.status == ComplianceStatus::Compliant
    }

    pub fn is_critical(&self) -> bool {
        self.severity == Severity::Critical
    }
}

/// Compliance violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub id: String,
    pub requirement_id: String,
    pub resource_id: String,
    pub resource_type: String,
    pub severity: Severity,
    pub description: String,
    pub remediation: String,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
}

impl ComplianceViolation {
    pub fn new(
        requirement_id: impl Into<String>,
        resource_id: impl Into<String>,
        resource_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let req_id = requirement_id.into();
        let id = format!("violation-{}-{}", req_id, Utc::now().timestamp());

        Self {
            id,
            requirement_id: req_id,
            resource_id: resource_id.into(),
            resource_type: resource_type.into(),
            severity: Severity::Medium,
            description: description.into(),
            remediation: String::new(),
            detected_at: Utc::now(),
            resolved_at: None,
            resolved_by: None,
        }
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = remediation.into();
        self
    }

    pub fn resolve(&mut self, resolved_by: impl Into<String>) {
        self.resolved_at = Some(Utc::now());
        self.resolved_by = Some(resolved_by.into());
    }

    pub fn is_resolved(&self) -> bool {
        self.resolved_at.is_some()
    }

    pub fn is_open(&self) -> bool {
        !self.is_resolved()
    }

    pub fn age_days(&self) -> i64 {
        (Utc::now() - self.detected_at).num_days()
    }
}

/// Compliance manager
pub struct ComplianceManager {
    requirements: HashMap<String, ComplianceRequirement>,
    violations: HashMap<String, ComplianceViolation>,
}

impl ComplianceManager {
    pub fn new() -> Self {
        Self {
            requirements: HashMap::new(),
            violations: HashMap::new(),
        }
    }

    pub fn add_requirement(&mut self, requirement: ComplianceRequirement) -> String {
        let id = requirement.id.clone();
        self.requirements.insert(id.clone(), requirement);
        id
    }

    pub fn get_requirement(&self, id: &str) -> Option<&ComplianceRequirement> {
        self.requirements.get(id)
    }

    pub fn get_requirement_mut(&mut self, id: &str) -> Option<&mut ComplianceRequirement> {
        self.requirements.get_mut(id)
    }

    pub fn remove_requirement(&mut self, id: &str) -> bool {
        self.requirements.remove(id).is_some()
    }

    pub fn requirement_count(&self) -> usize {
        self.requirements.len()
    }

    pub fn add_violation(&mut self, violation: ComplianceViolation) -> String {
        let id = violation.id.clone();
        self.violations.insert(id.clone(), violation);
        id
    }

    pub fn get_violation(&self, id: &str) -> Option<&ComplianceViolation> {
        self.violations.get(id)
    }

    pub fn get_violation_mut(&mut self, id: &str) -> Option<&mut ComplianceViolation> {
        self.violations.get_mut(id)
    }

    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    pub fn by_framework(&self, framework: &ComplianceFramework) -> Vec<&ComplianceRequirement> {
        self.requirements
            .values()
            .filter(|r| &r.framework == framework)
            .collect()
    }

    pub fn by_status(&self, status: &ComplianceStatus) -> Vec<&ComplianceRequirement> {
        self.requirements
            .values()
            .filter(|r| &r.status == status)
            .collect()
    }

    pub fn compliant_requirements(&self) -> Vec<&ComplianceRequirement> {
        self.requirements
            .values()
            .filter(|r| r.is_compliant())
            .collect()
    }

    pub fn non_compliant_requirements(&self) -> Vec<&ComplianceRequirement> {
        self.requirements
            .values()
            .filter(|r| r.status == ComplianceStatus::NonCompliant)
            .collect()
    }

    pub fn critical_requirements(&self) -> Vec<&ComplianceRequirement> {
        self.requirements
            .values()
            .filter(|r| r.is_critical())
            .collect()
    }

    pub fn open_violations(&self) -> Vec<&ComplianceViolation> {
        self.violations.values().filter(|v| v.is_open()).collect()
    }

    pub fn resolved_violations(&self) -> Vec<&ComplianceViolation> {
        self.violations
            .values()
            .filter(|v| v.is_resolved())
            .collect()
    }

    pub fn critical_violations(&self) -> Vec<&ComplianceViolation> {
        self.violations
            .values()
            .filter(|v| v.severity == Severity::Critical)
            .collect()
    }

    pub fn violations_by_resource(&self, resource_id: &str) -> Vec<&ComplianceViolation> {
        self.violations
            .values()
            .filter(|v| v.resource_id == resource_id)
            .collect()
    }

    pub fn compliance_score(&self) -> f64 {
        if self.requirements.is_empty() {
            return 100.0;
        }

        let compliant = self.compliant_requirements().len() as f64;
        let total = self.requirements.len() as f64;
        (compliant / total) * 100.0
    }
}

impl Default for ComplianceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_framework_display() {
        assert_eq!(ComplianceFramework::SOC2.to_string(), "SOC 2");
        assert_eq!(ComplianceFramework::HIPAA.to_string(), "HIPAA");
        assert_eq!(ComplianceFramework::GDPR.to_string(), "GDPR");
        assert_eq!(
            ComplianceFramework::Custom("MyFramework".to_string()).to_string(),
            "Custom: MyFramework"
        );
    }

    #[test]
    fn test_compliance_status_display() {
        assert_eq!(ComplianceStatus::Compliant.to_string(), "Compliant");
        assert_eq!(ComplianceStatus::NonCompliant.to_string(), "Non-Compliant");
        assert_eq!(
            ComplianceStatus::PartiallyCompliant.to_string(),
            "Partially Compliant"
        );
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Critical.to_string(), "Critical");
        assert_eq!(Severity::High.to_string(), "High");
        assert_eq!(Severity::Low.to_string(), "Low");
    }

    #[test]
    fn test_compliance_requirement() {
        let req = ComplianceRequirement::new(
            "Access Control",
            ComplianceFramework::SOC2,
            "CC6.1",
            "Implement access controls",
        );

        assert_eq!(req.name, "Access Control");
        assert_eq!(req.framework, ComplianceFramework::SOC2);
        assert_eq!(req.requirement_id, "CC6.1");
        assert_eq!(req.status, ComplianceStatus::UnderReview);
        assert!(!req.is_compliant());
    }

    #[test]
    fn test_requirement_builder() {
        let req = ComplianceRequirement::new(
            "Test",
            ComplianceFramework::HIPAA,
            "164.312",
            "Security rule",
        )
        .with_severity(Severity::Critical)
        .with_status(ComplianceStatus::Compliant);

        assert_eq!(req.severity, Severity::Critical);
        assert_eq!(req.status, ComplianceStatus::Compliant);
        assert!(req.is_critical());
        assert!(req.is_compliant());
    }

    #[test]
    fn test_requirement_control_objectives() {
        let mut req =
            ComplianceRequirement::new("Test", ComplianceFramework::NIST, "AC-1", "Access Control");

        req.add_control_objective("Implement MFA");
        req.add_control_objective("Review access logs");

        assert_eq!(req.control_objectives.len(), 2);
    }

    #[test]
    fn test_requirement_evidence() {
        let mut req =
            ComplianceRequirement::new("Test", ComplianceFramework::PciDss, "8.2", "Auth");

        req.add_evidence("MFA configuration");
        req.add_evidence("Access logs");

        assert_eq!(req.evidence_required.len(), 2);
    }

    #[test]
    fn test_requirement_update_status() {
        let mut req = ComplianceRequirement::new("Test", ComplianceFramework::SOC2, "CC1", "Test");

        assert!(req.last_assessed.is_none());

        req.update_status(ComplianceStatus::Compliant);

        assert_eq!(req.status, ComplianceStatus::Compliant);
        assert!(req.last_assessed.is_some());
    }

    #[test]
    fn test_compliance_violation() {
        let violation = ComplianceViolation::new(
            "req-123",
            "vm-456",
            "VirtualMachine",
            "Unencrypted data volume",
        );

        assert_eq!(violation.requirement_id, "req-123");
        assert_eq!(violation.resource_id, "vm-456");
        assert!(!violation.is_resolved());
        assert!(violation.is_open());
    }

    #[test]
    fn test_violation_builder() {
        let violation = ComplianceViolation::new("req", "vm", "VM", "Issue")
            .with_severity(Severity::High)
            .with_remediation("Enable encryption");

        assert_eq!(violation.severity, Severity::High);
        assert_eq!(violation.remediation, "Enable encryption");
    }

    #[test]
    fn test_violation_resolve() {
        let mut violation = ComplianceViolation::new("req", "vm", "VM", "Issue");

        assert!(!violation.is_resolved());

        violation.resolve("admin@example.com");

        assert!(violation.is_resolved());
        assert_eq!(violation.resolved_by, Some("admin@example.com".to_string()));
        assert!(violation.resolved_at.is_some());
    }

    #[test]
    fn test_violation_age() {
        let violation = ComplianceViolation::new("req", "vm", "VM", "Issue");

        assert_eq!(violation.age_days(), 0);
    }

    #[test]
    fn test_compliance_manager() {
        let mut manager = ComplianceManager::new();

        let req = ComplianceRequirement::new("Test", ComplianceFramework::SOC2, "CC1", "Test");
        let id = manager.add_requirement(req);

        assert_eq!(manager.requirement_count(), 1);
        assert!(manager.get_requirement(&id).is_some());
    }

    #[test]
    fn test_manager_violations() {
        let mut manager = ComplianceManager::new();

        let violation = ComplianceViolation::new("req", "vm", "VM", "Issue");
        let id = manager.add_violation(violation);

        assert_eq!(manager.violation_count(), 1);
        assert!(manager.get_violation(&id).is_some());
    }

    #[test]
    fn test_manager_by_framework() {
        let mut manager = ComplianceManager::new();

        manager.add_requirement(ComplianceRequirement::new(
            "R1",
            ComplianceFramework::SOC2,
            "CC1",
            "Test",
        ));
        manager.add_requirement(ComplianceRequirement::new(
            "R2",
            ComplianceFramework::HIPAA,
            "164.1",
            "Test",
        ));
        manager.add_requirement(ComplianceRequirement::new(
            "R3",
            ComplianceFramework::SOC2,
            "CC2",
            "Test",
        ));

        let soc2 = manager.by_framework(&ComplianceFramework::SOC2);
        assert_eq!(soc2.len(), 2);
    }

    #[test]
    fn test_manager_by_status() {
        let mut manager = ComplianceManager::new();

        manager.add_requirement(
            ComplianceRequirement::new("R1", ComplianceFramework::SOC2, "CC1", "Test")
                .with_status(ComplianceStatus::Compliant),
        );
        manager.add_requirement(
            ComplianceRequirement::new("R2", ComplianceFramework::HIPAA, "164.1", "Test")
                .with_status(ComplianceStatus::NonCompliant),
        );

        let compliant = manager.by_status(&ComplianceStatus::Compliant);
        assert_eq!(compliant.len(), 1);
    }

    #[test]
    fn test_manager_compliant_requirements() {
        let mut manager = ComplianceManager::new();

        manager.add_requirement(
            ComplianceRequirement::new("R1", ComplianceFramework::SOC2, "CC1", "Test")
                .with_status(ComplianceStatus::Compliant),
        );
        manager.add_requirement(
            ComplianceRequirement::new("R2", ComplianceFramework::HIPAA, "164.1", "Test")
                .with_status(ComplianceStatus::Compliant),
        );
        manager.add_requirement(ComplianceRequirement::new(
            "R3",
            ComplianceFramework::GDPR,
            "Art5",
            "Test",
        ));

        let compliant = manager.compliant_requirements();
        assert_eq!(compliant.len(), 2);
    }

    #[test]
    fn test_manager_non_compliant_requirements() {
        let mut manager = ComplianceManager::new();

        manager.add_requirement(
            ComplianceRequirement::new("R1", ComplianceFramework::SOC2, "CC1", "Test")
                .with_status(ComplianceStatus::NonCompliant),
        );
        manager.add_requirement(
            ComplianceRequirement::new("R2", ComplianceFramework::HIPAA, "164.1", "Test")
                .with_status(ComplianceStatus::Compliant),
        );

        let non_compliant = manager.non_compliant_requirements();
        assert_eq!(non_compliant.len(), 1);
    }

    #[test]
    fn test_manager_critical_requirements() {
        let mut manager = ComplianceManager::new();

        manager.add_requirement(
            ComplianceRequirement::new("R1", ComplianceFramework::SOC2, "CC1", "Test")
                .with_severity(Severity::Critical),
        );
        manager.add_requirement(
            ComplianceRequirement::new("R2", ComplianceFramework::HIPAA, "164.1", "Test")
                .with_severity(Severity::High),
        );
        manager.add_requirement(
            ComplianceRequirement::new("R3", ComplianceFramework::GDPR, "Art5", "Test")
                .with_severity(Severity::Critical),
        );

        let critical = manager.critical_requirements();
        assert_eq!(critical.len(), 2);
    }

    #[test]
    fn test_manager_open_violations() {
        let mut manager = ComplianceManager::new();

        let mut v1 = ComplianceViolation::new("req1", "vm1", "VM", "Issue 1");
        v1.resolve("admin");

        let v2 = ComplianceViolation::new("req2", "vm2", "VM", "Issue 2");

        manager.add_violation(v1);
        manager.add_violation(v2);

        let open = manager.open_violations();
        assert_eq!(open.len(), 1);
    }

    #[test]
    fn test_manager_resolved_violations() {
        let mut manager = ComplianceManager::new();

        let mut v1 = ComplianceViolation::new("req1", "vm1", "VM", "Issue 1");
        v1.resolve("admin");

        let mut v2 = ComplianceViolation::new("req2", "vm2", "VM", "Issue 2");
        v2.resolve("admin");

        let v3 = ComplianceViolation::new("req3", "vm3", "VM", "Issue 3");

        manager.add_violation(v1);
        manager.add_violation(v2);
        manager.add_violation(v3);

        let resolved = manager.resolved_violations();
        assert_eq!(resolved.len(), 2);
    }

    #[test]
    fn test_manager_critical_violations() {
        let mut manager = ComplianceManager::new();

        manager.add_violation(
            ComplianceViolation::new("req1", "vm1", "VM", "Critical issue")
                .with_severity(Severity::Critical),
        );
        manager.add_violation(
            ComplianceViolation::new("req2", "vm2", "VM", "High issue")
                .with_severity(Severity::High),
        );

        let critical = manager.critical_violations();
        assert_eq!(critical.len(), 1);
    }

    #[test]
    fn test_manager_violations_by_resource() {
        let mut manager = ComplianceManager::new();

        manager.add_violation(ComplianceViolation::new("req1", "vm-123", "VM", "Issue 1"));
        manager.add_violation(ComplianceViolation::new("req2", "vm-123", "VM", "Issue 2"));
        manager.add_violation(ComplianceViolation::new("req3", "vm-456", "VM", "Issue 3"));

        let vm123_violations = manager.violations_by_resource("vm-123");
        assert_eq!(vm123_violations.len(), 2);
    }

    #[test]
    fn test_manager_compliance_score() {
        let mut manager = ComplianceManager::new();

        manager.add_requirement(
            ComplianceRequirement::new("R1", ComplianceFramework::SOC2, "CC1", "Test")
                .with_status(ComplianceStatus::Compliant),
        );
        manager.add_requirement(
            ComplianceRequirement::new("R2", ComplianceFramework::HIPAA, "164.1", "Test")
                .with_status(ComplianceStatus::Compliant),
        );
        manager.add_requirement(
            ComplianceRequirement::new("R3", ComplianceFramework::GDPR, "Art5", "Test")
                .with_status(ComplianceStatus::NonCompliant),
        );
        manager.add_requirement(
            ComplianceRequirement::new("R4", ComplianceFramework::PciDss, "8.1", "Test")
                .with_status(ComplianceStatus::Compliant),
        );

        let score = manager.compliance_score();
        assert_eq!(score, 75.0);
    }

    #[test]
    fn test_manager_empty_compliance_score() {
        let manager = ComplianceManager::new();

        assert_eq!(manager.compliance_score(), 100.0);
    }

    #[test]
    fn test_manager_remove_requirement() {
        let mut manager = ComplianceManager::new();

        let req = ComplianceRequirement::new("Test", ComplianceFramework::SOC2, "CC1", "Test");
        let id = manager.add_requirement(req);

        assert!(manager.remove_requirement(&id));
        assert_eq!(manager.requirement_count(), 0);
    }

    #[test]
    fn test_manager_get_requirement_mut() {
        let mut manager = ComplianceManager::new();

        let req = ComplianceRequirement::new("Test", ComplianceFramework::SOC2, "CC1", "Test");
        let id = manager.add_requirement(req);

        if let Some(req_mut) = manager.get_requirement_mut(&id) {
            req_mut.update_status(ComplianceStatus::Compliant);
        }

        let req = manager.get_requirement(&id).unwrap();
        assert!(req.is_compliant());
    }

    #[test]
    fn test_manager_get_violation_mut() {
        let mut manager = ComplianceManager::new();

        let violation = ComplianceViolation::new("req", "vm", "VM", "Issue");
        let id = manager.add_violation(violation);

        if let Some(violation_mut) = manager.get_violation_mut(&id) {
            violation_mut.resolve("admin");
        }

        let violation = manager.get_violation(&id).unwrap();
        assert!(violation.is_resolved());
    }

    #[test]
    fn test_compliance_framework_equality() {
        assert_eq!(ComplianceFramework::SOC2, ComplianceFramework::SOC2);
        assert_ne!(ComplianceFramework::SOC2, ComplianceFramework::HIPAA);
    }

    #[test]
    fn test_compliance_status_equality() {
        assert_eq!(ComplianceStatus::Compliant, ComplianceStatus::Compliant);
        assert_ne!(ComplianceStatus::Compliant, ComplianceStatus::NonCompliant);
    }

    #[test]
    fn test_severity_equality() {
        assert_eq!(Severity::Critical, Severity::Critical);
        assert_ne!(Severity::Critical, Severity::High);
    }
}
