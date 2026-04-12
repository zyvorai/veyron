// Vulnerability Scanning - Scan VMs for security vulnerabilities

use super::{Severity, Vulnerability};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Vulnerability scan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub vm_name: String,
    pub scan_type: ScanType,
    pub enabled_scanners: Vec<Scanner>,
    pub scan_depth: ScanDepth,
    pub include_os_packages: bool,
    pub include_containers: bool,
    pub include_config_files: bool,
}

impl ScanConfig {
    pub fn new(vm_name: impl Into<String>, scan_type: ScanType) -> Self {
        Self {
            vm_name: vm_name.into(),
            scan_type,
            enabled_scanners: vec![Scanner::OSPackages, Scanner::ConfigFiles],
            scan_depth: ScanDepth::Standard,
            include_os_packages: true,
            include_containers: false,
            include_config_files: true,
        }
    }

    pub fn with_depth(mut self, depth: ScanDepth) -> Self {
        self.scan_depth = depth;
        self
    }

    pub fn enable_containers(mut self) -> Self {
        self.include_containers = true;
        if !self.enabled_scanners.contains(&Scanner::Containers) {
            self.enabled_scanners.push(Scanner::Containers);
        }
        self
    }

    pub fn add_scanner(mut self, scanner: Scanner) -> Self {
        if !self.enabled_scanners.contains(&scanner) {
            self.enabled_scanners.push(scanner);
        }
        self
    }
}

/// Scan type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScanType {
    Quick,      // Fast scan, essential vulnerabilities only
    Standard,   // Standard comprehensive scan
    Deep,       // Deep scan including all checks
    Compliance, // Compliance-focused scan
}

/// Scanner type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Scanner {
    OSPackages,  // OS package vulnerabilities (CVE scanning)
    Containers,  // Container image scanning
    ConfigFiles, // Configuration security issues
    Secrets,     // Exposed secrets detection
    Malware,     // Malware scanning
    Network,     // Network vulnerability scanning
}

/// Scan depth
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScanDepth {
    Quick,    // Surface-level scan
    Standard, // Standard depth
    Deep,     // Deep analysis
}

/// Scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub scan_id: String,
    pub vm_name: String,
    pub scan_type: ScanType,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ScanStatus,
    pub vulnerabilities: Vec<Vulnerability>,
    pub statistics: ScanStatistics,
    pub error_message: Option<String>,
}

impl ScanResult {
    pub fn new(
        scan_id: impl Into<String>,
        vm_name: impl Into<String>,
        scan_type: ScanType,
    ) -> Self {
        Self {
            scan_id: scan_id.into(),
            vm_name: vm_name.into(),
            scan_type,
            started_at: Utc::now(),
            completed_at: None,
            status: ScanStatus::Running,
            vulnerabilities: Vec::new(),
            statistics: ScanStatistics::default(),
            error_message: None,
        }
    }

    pub fn add_vulnerability(&mut self, vuln: Vulnerability) {
        match vuln.severity {
            Severity::Critical => self.statistics.critical += 1,
            Severity::High => self.statistics.high += 1,
            Severity::Medium => self.statistics.medium += 1,
            Severity::Low => self.statistics.low += 1,
            Severity::Info => self.statistics.info += 1,
        }
        self.statistics.total += 1;
        self.vulnerabilities.push(vuln);
    }

    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
        self.status = ScanStatus::Completed;
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.completed_at = Some(Utc::now());
        self.status = ScanStatus::Failed;
        self.error_message = Some(error.into());
    }

    pub fn duration_secs(&self) -> i64 {
        match self.completed_at {
            Some(completed) => completed
                .signed_duration_since(self.started_at)
                .num_seconds(),
            None => Utc::now()
                .signed_duration_since(self.started_at)
                .num_seconds(),
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.status, ScanStatus::Completed | ScanStatus::Failed)
    }
}

/// Scan status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScanStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl std::fmt::Display for ScanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanStatus::Pending => write!(f, "Pending"),
            ScanStatus::Running => write!(f, "Running"),
            ScanStatus::Completed => write!(f, "Completed"),
            ScanStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Scan statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanStatistics {
    pub total: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub packages_scanned: usize,
    pub files_scanned: usize,
}

impl ScanStatistics {
    pub fn risk_score(&self) -> f64 {
        // Weighted risk score
        (self.critical as f64 * 10.0)
            + (self.high as f64 * 5.0)
            + (self.medium as f64 * 2.0)
            + (self.low as f64 * 0.5)
    }
}

/// Vulnerability scanner
pub struct VulnerabilityScanner;

impl VulnerabilityScanner {
    /// Run a vulnerability scan
    pub fn scan(config: &ScanConfig) -> ScanResult {
        let scan_id = format!("scan-{}", Utc::now().format("%Y%m%d-%H%M%S"));
        let mut result = ScanResult::new(scan_id, &config.vm_name, config.scan_type.clone());

        // Simulate scanning different components
        if config.include_os_packages {
            Self::scan_os_packages(&mut result);
        }

        if config.include_config_files {
            Self::scan_config_files(&mut result);
        }

        if config.include_containers {
            Self::scan_containers(&mut result);
        }

        result.complete();
        result
    }

    fn scan_os_packages(result: &mut ScanResult) {
        result.statistics.packages_scanned = 150;

        // Example vulnerabilities
        result.add_vulnerability(
            Vulnerability::new(
                "VULN-OS-001",
                "OpenSSL vulnerable to CVE-2024-0001",
                Severity::High,
            )
            .with_description("OpenSSL version contains known vulnerability")
            .with_cvss(7.5)
            .with_cve("CVE-2024-0001")
            .with_package("openssl", Some("3.0.8".to_string())),
        );
    }

    fn scan_config_files(result: &mut ScanResult) {
        result.statistics.files_scanned = 50;

        result.add_vulnerability(
            Vulnerability::new("VULN-CFG-001", "SSH permits root login", Severity::Medium)
                .with_description("SSH configuration allows direct root login")
                .with_cvss(5.0),
        );
    }

    fn scan_containers(result: &mut ScanResult) {
        result.add_vulnerability(
            Vulnerability::new("VULN-CTR-001", "Container running as root", Severity::High)
                .with_description("Container process running with root privileges")
                .with_cvss(7.0),
        );
    }

    /// Quick security check
    pub fn quick_check(vm_name: &str) -> bool {
        let config = ScanConfig::new(vm_name, ScanType::Quick);
        let result = Self::scan(&config);
        result.statistics.critical == 0 && result.statistics.high == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_config() {
        let config = ScanConfig::new("test-vm", ScanType::Standard)
            .with_depth(ScanDepth::Deep)
            .enable_containers();

        assert_eq!(config.scan_type, ScanType::Standard);
        assert_eq!(config.scan_depth, ScanDepth::Deep);
        assert!(config.include_containers);
    }

    #[test]
    fn test_scan_result() {
        let mut result = ScanResult::new("scan-001", "test-vm", ScanType::Standard);

        result.add_vulnerability(Vulnerability::new(
            "V1",
            "Critical vuln",
            Severity::Critical,
        ));
        result.add_vulnerability(Vulnerability::new("V2", "High vuln", Severity::High));

        assert_eq!(result.statistics.total, 2);
        assert_eq!(result.statistics.critical, 1);
        assert_eq!(result.statistics.high, 1);
    }

    #[test]
    fn test_scan_completion() {
        let mut result = ScanResult::new("scan-001", "test-vm", ScanType::Quick);
        assert!(!result.is_complete());

        result.complete();
        assert!(result.is_complete());
        assert_eq!(result.status, ScanStatus::Completed);
        assert!(result.completed_at.is_some());
    }

    #[test]
    fn test_scan_failure() {
        let mut result = ScanResult::new("scan-001", "test-vm", ScanType::Quick);

        result.fail("Network timeout");
        assert!(result.is_complete());
        assert_eq!(result.status, ScanStatus::Failed);
        assert_eq!(result.error_message, Some("Network timeout".to_string()));
    }

    #[test]
    fn test_risk_score() {
        let stats = ScanStatistics {
            critical: 1,
            high: 2,
            medium: 3,
            low: 4,
            ..Default::default()
        };

        let score = stats.risk_score();
        assert_eq!(score, 10.0 + 10.0 + 6.0 + 2.0); // 28.0
    }

    #[test]
    fn test_vulnerability_scanner() {
        let config = ScanConfig::new("test-vm", ScanType::Standard);
        let result = VulnerabilityScanner::scan(&config);

        assert!(result.is_complete());
        assert_eq!(result.status, ScanStatus::Completed);
        assert!(result.statistics.total > 0);
    }

    #[test]
    fn test_quick_check() {
        let passed = VulnerabilityScanner::quick_check("test-vm");
        // Should pass since example vulns are High/Medium, not Critical
        assert!(!passed);
    }

    #[test]
    fn test_scanner_addition() {
        let config = ScanConfig::new("test-vm", ScanType::Standard)
            .add_scanner(Scanner::Secrets)
            .add_scanner(Scanner::Malware);

        assert!(config.enabled_scanners.contains(&Scanner::Secrets));
        assert!(config.enabled_scanners.contains(&Scanner::Malware));
    }

    #[test]
    fn test_scan_status_display() {
        assert_eq!(ScanStatus::Pending.to_string(), "Pending");
        assert_eq!(ScanStatus::Running.to_string(), "Running");
        assert_eq!(ScanStatus::Completed.to_string(), "Completed");
    }
}
