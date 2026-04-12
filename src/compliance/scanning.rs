use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Severity;

/// Scan status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ScanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanStatus::Pending => write!(f, "Pending"),
            ScanStatus::Running => write!(f, "Running"),
            ScanStatus::Completed => write!(f, "Completed"),
            ScanStatus::Failed => write!(f, "Failed"),
            ScanStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Scan type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanType {
    Vulnerability,
    Compliance,
    Configuration,
    Security,
    FullScan,
}

/// Compliance scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceScan {
    pub id: String,
    pub name: String,
    pub scan_type: ScanType,
    pub status: ScanStatus,
    pub target_resources: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub findings: Vec<ScanFinding>,
    pub total_checks: u32,
    pub passed_checks: u32,
    pub failed_checks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanFinding {
    pub check_id: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub resource_id: String,
    pub remediation: String,
}

impl ComplianceScan {
    pub fn new(name: impl Into<String>, scan_type: ScanType) -> Self {
        let name_str = name.into();
        let id = format!(
            "scan-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            scan_type,
            status: ScanStatus::Pending,
            target_resources: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
            findings: Vec::new(),
            total_checks: 0,
            passed_checks: 0,
            failed_checks: 0,
        }
    }

    pub fn add_target(&mut self, resource_id: impl Into<String>) {
        self.target_resources.push(resource_id.into());
    }

    pub fn start(&mut self) {
        self.status = ScanStatus::Running;
        self.started_at = Utc::now();
    }

    pub fn complete(&mut self) {
        self.status = ScanStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self) {
        self.status = ScanStatus::Failed;
        self.completed_at = Some(Utc::now());
    }

    pub fn cancel(&mut self) {
        self.status = ScanStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    pub fn add_finding(&mut self, finding: ScanFinding) {
        self.findings.push(finding);
    }

    pub fn update_checks(&mut self, total: u32, passed: u32, failed: u32) {
        self.total_checks = total;
        self.passed_checks = passed;
        self.failed_checks = failed;
    }

    pub fn compliance_score(&self) -> f64 {
        if self.total_checks == 0 {
            return 100.0;
        }

        (self.passed_checks as f64 / self.total_checks as f64) * 100.0
    }

    pub fn critical_findings(&self) -> Vec<&ScanFinding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .collect()
    }

    pub fn is_completed(&self) -> bool {
        self.status == ScanStatus::Completed
    }

    pub fn duration_seconds(&self) -> Option<i64> {
        self.completed_at
            .map(|end| (end - self.started_at).num_seconds())
    }
}

impl ScanFinding {
    pub fn new(
        check_id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        severity: Severity,
        resource_id: impl Into<String>,
    ) -> Self {
        Self {
            check_id: check_id.into(),
            title: title.into(),
            description: description.into(),
            severity,
            resource_id: resource_id.into(),
            remediation: String::new(),
        }
    }

    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = remediation.into();
        self
    }

    pub fn is_critical(&self) -> bool {
        self.severity == Severity::Critical
    }
}

/// Scan manager
pub struct ScanManager {
    scans: HashMap<String, ComplianceScan>,
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            scans: HashMap::new(),
        }
    }

    pub fn add_scan(&mut self, scan: ComplianceScan) -> String {
        let id = scan.id.clone();
        self.scans.insert(id.clone(), scan);
        id
    }

    pub fn get_scan(&self, id: &str) -> Option<&ComplianceScan> {
        self.scans.get(id)
    }

    pub fn get_scan_mut(&mut self, id: &str) -> Option<&mut ComplianceScan> {
        self.scans.get_mut(id)
    }

    pub fn scan_count(&self) -> usize {
        self.scans.len()
    }

    pub fn active_scans(&self) -> Vec<&ComplianceScan> {
        self.scans
            .values()
            .filter(|s| s.status == ScanStatus::Running)
            .collect()
    }

    pub fn completed_scans(&self) -> Vec<&ComplianceScan> {
        self.scans.values().filter(|s| s.is_completed()).collect()
    }

    pub fn by_type(&self, scan_type: &ScanType) -> Vec<&ComplianceScan> {
        self.scans
            .values()
            .filter(|s| &s.scan_type == scan_type)
            .collect()
    }

    pub fn scans_with_critical_findings(&self) -> Vec<&ComplianceScan> {
        self.scans
            .values()
            .filter(|s| !s.critical_findings().is_empty())
            .collect()
    }
}

impl Default for ScanManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_status_display() {
        assert_eq!(ScanStatus::Pending.to_string(), "Pending");
        assert_eq!(ScanStatus::Running.to_string(), "Running");
        assert_eq!(ScanStatus::Completed.to_string(), "Completed");
    }

    #[test]
    fn test_compliance_scan() {
        let scan = ComplianceScan::new("Security Scan", ScanType::Security);

        assert_eq!(scan.name, "Security Scan");
        assert_eq!(scan.scan_type, ScanType::Security);
        assert_eq!(scan.status, ScanStatus::Pending);
        assert_eq!(scan.findings.len(), 0);
        assert!(!scan.is_completed());
    }

    #[test]
    fn test_scan_lifecycle() {
        let mut scan = ComplianceScan::new("Test Scan", ScanType::Compliance);

        assert_eq!(scan.status, ScanStatus::Pending);

        scan.start();
        assert_eq!(scan.status, ScanStatus::Running);

        scan.complete();
        assert_eq!(scan.status, ScanStatus::Completed);
        assert!(scan.is_completed());
        assert!(scan.completed_at.is_some());
    }

    #[test]
    fn test_scan_fail() {
        let mut scan = ComplianceScan::new("Test", ScanType::Vulnerability);

        scan.fail();
        assert_eq!(scan.status, ScanStatus::Failed);
        assert!(scan.completed_at.is_some());
    }

    #[test]
    fn test_scan_cancel() {
        let mut scan = ComplianceScan::new("Test", ScanType::Configuration);

        scan.cancel();
        assert_eq!(scan.status, ScanStatus::Cancelled);
        assert!(scan.completed_at.is_some());
    }

    #[test]
    fn test_scan_add_target() {
        let mut scan = ComplianceScan::new("Test", ScanType::Security);

        scan.add_target("vm-123");
        scan.add_target("vm-456");

        assert_eq!(scan.target_resources.len(), 2);
    }

    #[test]
    fn test_scan_add_finding() {
        let mut scan = ComplianceScan::new("Test", ScanType::Compliance);

        scan.add_finding(ScanFinding::new(
            "CHECK-001",
            "Unencrypted volume",
            "Volume is not encrypted",
            Severity::High,
            "vol-123",
        ));

        assert_eq!(scan.findings.len(), 1);
    }

    #[test]
    fn test_scan_update_checks() {
        let mut scan = ComplianceScan::new("Test", ScanType::Compliance);

        scan.update_checks(100, 85, 15);

        assert_eq!(scan.total_checks, 100);
        assert_eq!(scan.passed_checks, 85);
        assert_eq!(scan.failed_checks, 15);
    }

    #[test]
    fn test_scan_compliance_score() {
        let mut scan = ComplianceScan::new("Test", ScanType::Compliance);

        scan.update_checks(100, 80, 20);

        assert_eq!(scan.compliance_score(), 80.0);
    }

    #[test]
    fn test_scan_empty_compliance_score() {
        let scan = ComplianceScan::new("Test", ScanType::Compliance);

        assert_eq!(scan.compliance_score(), 100.0);
    }

    #[test]
    fn test_scan_critical_findings() {
        let mut scan = ComplianceScan::new("Test", ScanType::Security);

        scan.add_finding(ScanFinding::new(
            "CHK-1",
            "Critical issue",
            "Desc",
            Severity::Critical,
            "vm-1",
        ));

        scan.add_finding(ScanFinding::new(
            "CHK-2",
            "High issue",
            "Desc",
            Severity::High,
            "vm-2",
        ));

        scan.add_finding(ScanFinding::new(
            "CHK-3",
            "Critical issue 2",
            "Desc",
            Severity::Critical,
            "vm-3",
        ));

        let critical = scan.critical_findings();
        assert_eq!(critical.len(), 2);
    }

    #[test]
    fn test_scan_duration() {
        let mut scan = ComplianceScan::new("Test", ScanType::Compliance);

        assert!(scan.duration_seconds().is_none());

        scan.complete();

        let duration = scan.duration_seconds();
        assert!(duration.is_some());
        assert!(duration.unwrap() >= 0);
    }

    #[test]
    fn test_scan_finding() {
        let finding = ScanFinding::new(
            "CHECK-001",
            "Missing encryption",
            "Data volume not encrypted",
            Severity::High,
            "vol-123",
        );

        assert_eq!(finding.check_id, "CHECK-001");
        assert_eq!(finding.title, "Missing encryption");
        assert_eq!(finding.severity, Severity::High);
        assert!(!finding.is_critical());
    }

    #[test]
    fn test_finding_with_remediation() {
        let finding = ScanFinding::new(
            "CHECK-002",
            "Weak password",
            "Password policy too weak",
            Severity::Medium,
            "config-1",
        )
        .with_remediation("Enable strong password policy");

        assert_eq!(finding.remediation, "Enable strong password policy");
    }

    #[test]
    fn test_finding_is_critical() {
        let critical = ScanFinding::new("CHK-1", "Title", "Desc", Severity::Critical, "res-1");
        let high = ScanFinding::new("CHK-2", "Title", "Desc", Severity::High, "res-2");

        assert!(critical.is_critical());
        assert!(!high.is_critical());
    }

    #[test]
    fn test_scan_manager() {
        let mut manager = ScanManager::new();

        let scan = ComplianceScan::new("Test", ScanType::Security);
        let id = manager.add_scan(scan);

        assert_eq!(manager.scan_count(), 1);
        assert!(manager.get_scan(&id).is_some());
    }

    #[test]
    fn test_manager_active_scans() {
        let mut manager = ScanManager::new();

        let mut scan1 = ComplianceScan::new("Scan 1", ScanType::Compliance);
        scan1.start();

        let mut scan2 = ComplianceScan::new("Scan 2", ScanType::Security);
        scan2.complete();

        manager.add_scan(scan1);
        manager.add_scan(scan2);

        let active = manager.active_scans();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_completed_scans() {
        let mut manager = ScanManager::new();

        let mut scan1 = ComplianceScan::new("Scan 1", ScanType::Compliance);
        scan1.complete();

        let mut scan2 = ComplianceScan::new("Scan 2", ScanType::Security);
        scan2.complete();

        let scan3 = ComplianceScan::new("Scan 3", ScanType::Vulnerability);

        manager.add_scan(scan1);
        manager.add_scan(scan2);
        manager.add_scan(scan3);

        let completed = manager.completed_scans();
        assert_eq!(completed.len(), 2);
    }

    #[test]
    fn test_manager_by_type() {
        let mut manager = ScanManager::new();

        manager.add_scan(ComplianceScan::new("S1", ScanType::Compliance));
        manager.add_scan(ComplianceScan::new("S2", ScanType::Security));
        manager.add_scan(ComplianceScan::new("S3", ScanType::Compliance));

        let compliance = manager.by_type(&ScanType::Compliance);
        assert_eq!(compliance.len(), 2);
    }

    #[test]
    fn test_manager_scans_with_critical_findings() {
        let mut manager = ScanManager::new();

        let mut scan1 = ComplianceScan::new("Scan 1", ScanType::Security);
        scan1.add_finding(ScanFinding::new(
            "C1",
            "Title",
            "Desc",
            Severity::Critical,
            "r1",
        ));

        let mut scan2 = ComplianceScan::new("Scan 2", ScanType::Compliance);
        scan2.add_finding(ScanFinding::new(
            "C2",
            "Title",
            "Desc",
            Severity::High,
            "r2",
        ));

        manager.add_scan(scan1);
        manager.add_scan(scan2);

        let critical = manager.scans_with_critical_findings();
        assert_eq!(critical.len(), 1);
    }

    #[test]
    fn test_manager_get_scan_mut() {
        let mut manager = ScanManager::new();

        let scan = ComplianceScan::new("Test", ScanType::Security);
        let id = manager.add_scan(scan);

        if let Some(scan_mut) = manager.get_scan_mut(&id) {
            scan_mut.start();
        }

        let scan = manager.get_scan(&id).unwrap();
        assert_eq!(scan.status, ScanStatus::Running);
    }

    #[test]
    fn test_scan_status_equality() {
        assert_eq!(ScanStatus::Running, ScanStatus::Running);
        assert_ne!(ScanStatus::Running, ScanStatus::Completed);
    }

    #[test]
    fn test_scan_type_equality() {
        assert_eq!(ScanType::Compliance, ScanType::Compliance);
        assert_ne!(ScanType::Compliance, ScanType::Security);
    }
}
