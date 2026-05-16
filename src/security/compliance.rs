// Compliance Checking - Verify VM compliance with security standards

use crate::kube::VirtualMachine;
use chrono::{DateTime, Utc};
use kube::ResourceExt;
use serde::{Deserialize, Serialize};

/// Compliance check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub check_id: String,
    pub control_id: String,
    pub title: String,
    pub description: String,
    pub framework: ComplianceFramework,
    pub severity: CheckSeverity,
    pub automated: bool,
}

impl ComplianceCheck {
    pub fn new(
        check_id: impl Into<String>,
        control_id: impl Into<String>,
        title: impl Into<String>,
        framework: ComplianceFramework,
    ) -> Self {
        Self {
            check_id: check_id.into(),
            control_id: control_id.into(),
            title: title.into(),
            description: String::new(),
            framework,
            severity: CheckSeverity::Medium,
            automated: false,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_severity(mut self, severity: CheckSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn automated(mut self) -> Self {
        self.automated = true;
        self
    }
}

/// Compliance framework
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceFramework {
    PCIDSS,   // Payment Card Industry Data Security Standard
    HIPAA,    // Health Insurance Portability and Accountability Act
    SOC2,     // Service Organization Control 2
    ISO27001, // ISO/IEC 27001
    GDPR,     // General Data Protection Regulation
    NIST,     // NIST Cybersecurity Framework
    CIS,      // CIS Controls
    Custom(String),
}

impl std::fmt::Display for ComplianceFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplianceFramework::PCIDSS => write!(f, "PCI-DSS"),
            ComplianceFramework::HIPAA => write!(f, "HIPAA"),
            ComplianceFramework::SOC2 => write!(f, "SOC 2"),
            ComplianceFramework::ISO27001 => write!(f, "ISO 27001"),
            ComplianceFramework::GDPR => write!(f, "GDPR"),
            ComplianceFramework::NIST => write!(f, "NIST CSF"),
            ComplianceFramework::CIS => write!(f, "CIS Controls"),
            ComplianceFramework::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Check severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub report_id: String,
    pub vm_name: String,
    pub framework: ComplianceFramework,
    pub generated_at: DateTime<Utc>,
    pub check_results: Vec<CheckResult>,
    pub summary: ComplianceSummary,
    pub compliant: bool,
}

impl ComplianceReport {
    pub fn new(vm_name: impl Into<String>, framework: ComplianceFramework) -> Self {
        let report_id = format!("report-{}", Utc::now().format("%Y%m%d-%H%M%S"));
        Self {
            report_id,
            vm_name: vm_name.into(),
            framework,
            generated_at: Utc::now(),
            check_results: Vec::new(),
            summary: ComplianceSummary::default(),
            compliant: false,
        }
    }

    pub fn add_result(&mut self, result: CheckResult) {
        self.summary.total_checks += 1;

        match result.status {
            CheckStatus::Passed => self.summary.passed += 1,
            CheckStatus::Failed => {
                self.summary.failed += 1;
                if result.severity == CheckSeverity::Critical {
                    self.summary.critical_failures += 1;
                }
            }
            CheckStatus::NotApplicable => self.summary.not_applicable += 1,
            CheckStatus::ManualReview => self.summary.manual_review += 1,
        }

        self.check_results.push(result);
    }

    pub fn finalize(&mut self) {
        self.summary.compliance_score = self.calculate_compliance_score();
        self.compliant = self.summary.failed == 0 && self.summary.manual_review == 0;
    }

    fn calculate_compliance_score(&self) -> f64 {
        let applicable = self
            .summary
            .total_checks
            .saturating_sub(self.summary.not_applicable);
        if applicable == 0 {
            return 100.0;
        }
        (self.summary.passed as f64 / applicable as f64) * 100.0
    }

    pub fn critical_failures(&self) -> Vec<&CheckResult> {
        self.check_results
            .iter()
            .filter(|r| r.status == CheckStatus::Failed && r.severity == CheckSeverity::Critical)
            .collect()
    }
}

/// Compliance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceSummary {
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub not_applicable: usize,
    pub manual_review: usize,
    pub critical_failures: usize,
    pub compliance_score: f64,
}

impl Default for ComplianceSummary {
    fn default() -> Self {
        Self {
            total_checks: 0,
            passed: 0,
            failed: 0,
            not_applicable: 0,
            manual_review: 0,
            critical_failures: 0,
            compliance_score: 0.0,
        }
    }
}

/// Check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub check_id: String,
    pub control_id: String,
    pub title: String,
    pub status: CheckStatus,
    pub severity: CheckSeverity,
    pub message: String,
    pub evidence: Option<String>,
    pub checked_at: DateTime<Utc>,
}

impl CheckResult {
    pub fn new(
        check_id: impl Into<String>,
        control_id: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        Self {
            check_id: check_id.into(),
            control_id: control_id.into(),
            title: title.into(),
            status: CheckStatus::Passed,
            severity: CheckSeverity::Medium,
            message: String::new(),
            evidence: None,
            checked_at: Utc::now(),
        }
    }

    pub fn passed(mut self) -> Self {
        self.status = CheckStatus::Passed;
        self.message = "Check passed".to_string();
        self
    }

    pub fn failed(mut self, message: impl Into<String>) -> Self {
        self.status = CheckStatus::Failed;
        self.message = message.into();
        self
    }

    pub fn not_applicable(mut self) -> Self {
        self.status = CheckStatus::NotApplicable;
        self.message = "Not applicable to this VM".to_string();
        self
    }

    pub fn manual_review(mut self, message: impl Into<String>) -> Self {
        self.status = CheckStatus::ManualReview;
        self.message = message.into();
        self
    }

    pub fn with_severity(mut self, severity: CheckSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = Some(evidence.into());
        self
    }
}

/// Check status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckStatus {
    Passed,
    Failed,
    NotApplicable,
    ManualReview,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Passed => write!(f, "Passed"),
            CheckStatus::Failed => write!(f, "Failed"),
            CheckStatus::NotApplicable => write!(f, "N/A"),
            CheckStatus::ManualReview => write!(f, "Manual Review"),
        }
    }
}

/// Compliance checker that inspects actual VM specs from Kubernetes.
pub struct ComplianceChecker;

impl ComplianceChecker {
    /// Run PCI-DSS compliance check against the actual VM spec.
    ///
    /// Checks:
    /// 1. Resource limits set (CPU/memory limits in addition to requests)
    /// 2. Eviction strategy configured (for HA)
    /// 3. No default namespace (should be in a dedicated namespace)
    /// 4. Network interfaces configured (not exposed on default pod network without policies)
    pub fn check_pci_dss(vm_name: &str, vm: &VirtualMachine) -> ComplianceReport {
        let mut report = ComplianceReport::new(vm_name, ComplianceFramework::PCIDSS);
        let vmi_spec = &vm.spec.template.spec;
        let domain = &vmi_spec.domain;

        // PCI-DSS Requirement 1: Resource limits set (CPU and memory)
        {
            let has_cpu_limit = domain
                .resources
                .limits
                .as_ref()
                .map(|l| l.contains_key("cpu"))
                .unwrap_or(false);
            let has_mem_limit = domain
                .resources
                .limits
                .as_ref()
                .map(|l| l.contains_key("memory"))
                .unwrap_or(false);

            let result =
                CheckResult::new("PCI-1.1", "REQ-1", "Resource limits set (CPU and memory)")
                    .with_severity(CheckSeverity::Critical);

            if has_cpu_limit && has_mem_limit {
                report.add_result(
                    result
                        .passed()
                        .with_evidence("Both CPU and memory limits are configured"),
                );
            } else {
                let mut missing = Vec::new();
                if !has_cpu_limit {
                    missing.push("cpu");
                }
                if !has_mem_limit {
                    missing.push("memory");
                }
                report.add_result(result.failed(format!(
                    "Resource limits missing for: {}",
                    missing.join(", ")
                )));
            }
        }

        // PCI-DSS Requirement 2: Eviction strategy configured (for HA)
        {
            let has_eviction = vmi_spec.eviction_strategy.is_some();
            let result =
                CheckResult::new("PCI-2.1", "REQ-2", "Eviction strategy configured for HA")
                    .with_severity(CheckSeverity::High);

            if has_eviction {
                report.add_result(result.passed().with_evidence(format!(
                    "Eviction strategy: {}",
                    vmi_spec.eviction_strategy.as_deref().unwrap_or("set")
                )));
            } else {
                report.add_result(
                    result.failed("No eviction strategy configured; VM may not be HA-resilient"),
                );
            }
        }

        // PCI-DSS Requirement 3: Not in default namespace
        {
            let ns = vm.namespace().unwrap_or_default();
            let result = CheckResult::new("PCI-3.1", "REQ-3", "VM not in default namespace")
                .with_severity(CheckSeverity::Critical);

            if ns != "default" && !ns.is_empty() {
                report.add_result(
                    result
                        .passed()
                        .with_evidence(format!("VM is in dedicated namespace: {}", ns)),
                );
            } else {
                report.add_result(result.failed(
                    "VM is in the 'default' namespace; should use a dedicated namespace for isolation",
                ));
            }
        }

        // PCI-DSS Requirement 4: Network interfaces configured
        {
            let has_interfaces = vmi_spec
                .networks
                .as_ref()
                .map(|n| !n.is_empty())
                .unwrap_or(false);
            let result = CheckResult::new(
                "PCI-4.1",
                "REQ-4",
                "Network interfaces explicitly configured",
            )
            .with_severity(CheckSeverity::High);

            if has_interfaces {
                report.add_result(
                    result
                        .passed()
                        .with_evidence("Network interfaces are explicitly configured"),
                );
            } else {
                report.add_result(result.failed(
                    "No network interfaces configured; VM may be exposed on default pod network",
                ));
            }
        }

        report.finalize();
        report
    }

    /// Run HIPAA compliance check against the actual VM spec.
    ///
    /// Checks:
    /// 1. Encryption - TPM device enabled
    /// 2. Resource isolation - dedicated CPU placement
    /// 3. Audit logging - labels present for tracking
    pub fn check_hipaa(vm_name: &str, vm: &VirtualMachine) -> ComplianceReport {
        let mut report = ComplianceReport::new(vm_name, ComplianceFramework::HIPAA);
        let vmi_spec = &vm.spec.template.spec;
        let domain = &vmi_spec.domain;

        // HIPAA 1: Encryption - TPM device enabled
        {
            let has_tpm = domain
                .devices
                .as_ref()
                .and_then(|d| d.tpm.as_ref())
                .is_some();
            let result = CheckResult::new("HIPAA-EN-1", "164.312(e)(1)", "TPM device enabled")
                .with_severity(CheckSeverity::Critical);

            if has_tpm {
                report.add_result(
                    result
                        .passed()
                        .with_evidence("TPM device is configured on the VM"),
                );
            } else {
                report.add_result(result.failed(
                    "No TPM device configured; encryption capabilities are not hardware-backed",
                ));
            }
        }

        // HIPAA 2: Resource isolation - dedicated CPU placement
        {
            let has_dedicated_cpu = domain
                .cpu
                .as_ref()
                .and_then(|c| c.dedicated_cpu_placement)
                .unwrap_or(false);
            let result = CheckResult::new(
                "HIPAA-ISO-1",
                "164.312(a)(1)",
                "Dedicated CPU placement for resource isolation",
            )
            .with_severity(CheckSeverity::High);

            if has_dedicated_cpu {
                report.add_result(
                    result
                        .passed()
                        .with_evidence("Dedicated CPU placement is enabled"),
                );
            } else {
                report.add_result(result.failed(
                    "Dedicated CPU placement not enabled; VM shares CPU resources with other workloads",
                ));
            }
        }

        // HIPAA 3: Audit logging - labels present for tracking
        {
            let has_labels = vm
                .metadata
                .labels
                .as_ref()
                .map(|l| !l.is_empty())
                .unwrap_or(false);
            let result = CheckResult::new(
                "HIPAA-AU-1",
                "164.312(b)",
                "Labels present for audit tracking",
            )
            .with_severity(CheckSeverity::High);

            if has_labels {
                let label_count = vm.metadata.labels.as_ref().map(|l| l.len()).unwrap_or(0);
                report.add_result(result.passed().with_evidence(format!(
                    "{} label(s) configured for audit tracking",
                    label_count
                )));
            } else {
                report.add_result(result.failed(
                    "No labels configured on VM; labels are needed for audit trail and tracking",
                ));
            }
        }

        report.finalize();
        report
    }

    /// Run SOC 2 compliance check against the actual VM spec.
    ///
    /// Checks:
    /// 1. Resource requests defined
    /// 2. Boot order configured (secure boot chain)
    /// 3. RNG device enabled (for cryptographic operations)
    pub fn check_soc2(vm_name: &str, vm: &VirtualMachine) -> ComplianceReport {
        let mut report = ComplianceReport::new(vm_name, ComplianceFramework::SOC2);
        let vmi_spec = &vm.spec.template.spec;
        let domain = &vmi_spec.domain;

        // SOC2 1: Resource requests defined
        {
            let has_requests = domain
                .resources
                .requests
                .as_ref()
                .map(|r| !r.is_empty())
                .unwrap_or(false);
            let result = CheckResult::new("SOC2-SEC-1", "CC6.1", "Resource requests defined")
                .with_severity(CheckSeverity::High);

            if has_requests {
                report.add_result(
                    result
                        .passed()
                        .with_evidence("Resource requests are configured"),
                );
            } else {
                report.add_result(result.failed(
                    "No resource requests defined; VM has no guaranteed resource allocation",
                ));
            }
        }

        // SOC2 2: Boot order configured (secure boot chain)
        {
            let has_boot_order = domain
                .devices
                .as_ref()
                .and_then(|d| d.disks.as_ref())
                .map(|disks| disks.iter().any(|d| d.boot_order.is_some()))
                .unwrap_or(false);
            let result = CheckResult::new(
                "SOC2-SEC-2",
                "CC6.2",
                "Boot order configured for secure boot chain",
            )
            .with_severity(CheckSeverity::Medium);

            if has_boot_order {
                report.add_result(
                    result
                        .passed()
                        .with_evidence("Boot order is explicitly configured on disk(s)"),
                );
            } else {
                report.add_result(result.failed(
                    "No boot order configured on any disk; boot chain is not explicitly controlled",
                ));
            }
        }

        // SOC2 3: RNG device enabled (for cryptographic operations)
        {
            let has_rng = domain
                .devices
                .as_ref()
                .and_then(|d| d.rng.as_ref())
                .is_some();
            let result = CheckResult::new(
                "SOC2-AVL-1",
                "A1.1",
                "RNG device enabled for cryptographic operations",
            )
            .with_severity(CheckSeverity::Medium);

            if has_rng {
                report.add_result(
                    result
                        .passed()
                        .with_evidence("RNG device is configured on the VM"),
                );
            } else {
                report.add_result(result.failed(
                    "No RNG device configured; cryptographic operations may lack sufficient entropy",
                ));
            }
        }

        report.finalize();
        report
    }

    /// Run comprehensive compliance check across all frameworks
    pub fn check_all(vm_name: &str, vm: &VirtualMachine) -> Vec<ComplianceReport> {
        vec![
            Self::check_pci_dss(vm_name, vm),
            Self::check_hipaa(vm_name, vm),
            Self::check_soc2(vm_name, vm),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::types::*;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use std::collections::BTreeMap;

    /// Build a minimal VM for testing (non-compliant baseline).
    fn make_minimal_vm() -> VirtualMachine {
        VirtualMachine {
            metadata: ObjectMeta {
                name: Some("test-vm".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: VirtualMachineSpec {
                running: Some(false),
                run_strategy: None,
                instancetype: None,
                preference: None,
                template: VirtualMachineInstanceTemplateSpec {
                    metadata: None,
                    spec: VirtualMachineInstanceSpec {
                        domain: DomainSpec {
                            resources: ResourceRequirements {
                                requests: None,
                                limits: None,
                            },
                            cpu: None,
                            memory: None,
                            devices: None,
                            filesystems: None,
                            features: None,
                            clock: None,
                            firmware: None,
                            machine: None,
                            io_threads_policy: None,
                        },
                        volumes: None,
                        networks: None,
                        termination_grace_period_seconds: None,
                        eviction_strategy: None,
                        node_selector: None,
                        priority_class_name: None,
                        affinity: None,
                        tolerations: None,
                        topology_spread_constraints: None,
                        scheduler_name: None,
                        access_credentials: None,
                    },
                },
            },
            status: None,
        }
    }

    /// Build a fully-compliant VM for testing (passes all checks).
    fn make_compliant_vm() -> VirtualMachine {
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "secure-vm".to_string());
        labels.insert("env".to_string(), "production".to_string());

        let mut requests = BTreeMap::new();
        requests.insert("cpu".to_string(), "2".to_string());
        requests.insert("memory".to_string(), "4Gi".to_string());
        let mut limits = BTreeMap::new();
        limits.insert("cpu".to_string(), "4".to_string());
        limits.insert("memory".to_string(), "8Gi".to_string());

        VirtualMachine {
            metadata: ObjectMeta {
                name: Some("test-vm".to_string()),
                namespace: Some("production".to_string()),
                labels: Some(labels),
                ..Default::default()
            },
            spec: VirtualMachineSpec {
                running: Some(true),
                run_strategy: None,
                instancetype: None,
                preference: None,
                template: VirtualMachineInstanceTemplateSpec {
                    metadata: None,
                    spec: VirtualMachineInstanceSpec {
                        domain: DomainSpec {
                            resources: ResourceRequirements {
                                requests: Some(requests),
                                limits: Some(limits),
                            },
                            cpu: Some(CPU {
                                cores: Some(2),
                                sockets: Some(1),
                                threads: Some(1),
                                model: None,
                                dedicated_cpu_placement: Some(true),
                                isolate_emulator_thread: None,
                                numa: None,
                                realtime: None,
                            }),
                            memory: None,
                            devices: Some(Devices {
                                disks: Some(vec![Disk {
                                    name: "rootdisk".to_string(),
                                    disk: Some(DiskTarget {
                                        bus: Some("virtio".to_string()),
                                        readonly: None,
                                    }),
                                    lun: None,
                                    cdrom: None,
                                    boot_order: Some(1),
                                    cache: None,
                                    io: None,
                                    dedicated_io_thread: None,
                                    serial: None,
                                }]),
                                interfaces: Some(vec![Interface {
                                    name: "default".to_string(),
                                    model: None,
                                    mac_address: None,
                                    masquerade: Some(BTreeMap::new()),
                                    bridge: None,
                                    sriov: None,
                                    ports: None,
                                    boot_order: None,
                                }]),
                                tpm: Some(TPMDevice {}),
                                rng: Some(RNGDevice {}),
                                inputs: None,
                                host_devices: None,
                                watchdog: None,
                                autoattach_graphics_device: None,
                                autoattach_mem_balloon: None,
                                network_interface_multiqueue: None,
                                video: None,
                            }),
                            filesystems: None,
                            features: None,
                            clock: None,
                            firmware: None,
                            machine: None,
                            io_threads_policy: None,
                        },
                        volumes: None,
                        networks: Some(vec![Network {
                            name: "default".to_string(),
                            pod: Some(BTreeMap::new()),
                            multus: None,
                        }]),
                        termination_grace_period_seconds: None,
                        eviction_strategy: Some("LiveMigrate".to_string()),
                        node_selector: None,
                        priority_class_name: None,
                        affinity: None,
                        tolerations: None,
                        topology_spread_constraints: None,
                        scheduler_name: None,
                        access_credentials: None,
                    },
                },
            },
            status: None,
        }
    }

    #[test]
    fn test_compliance_check() {
        let check = ComplianceCheck::new(
            "CHECK-001",
            "CTRL-001",
            "Encryption enabled",
            ComplianceFramework::PCIDSS,
        )
        .with_description("Verify encryption is enabled")
        .with_severity(CheckSeverity::Critical)
        .automated();

        assert_eq!(check.framework, ComplianceFramework::PCIDSS);
        assert_eq!(check.severity, CheckSeverity::Critical);
        assert!(check.automated);
    }

    #[test]
    fn test_compliance_report() {
        let mut report = ComplianceReport::new("test-vm", ComplianceFramework::PCIDSS);

        report.add_result(CheckResult::new("C1", "CTRL1", "Test 1").passed());
        report.add_result(
            CheckResult::new("C2", "CTRL2", "Test 2")
                .with_severity(CheckSeverity::Critical)
                .failed("Critical issue"),
        );
        report.add_result(CheckResult::new("C3", "CTRL3", "Test 3").not_applicable());

        assert_eq!(report.summary.total_checks, 3);
        assert_eq!(report.summary.passed, 1);
        assert_eq!(report.summary.failed, 1);
        assert_eq!(report.summary.not_applicable, 1);
        assert_eq!(report.summary.critical_failures, 1);
    }

    #[test]
    fn test_compliance_score() {
        let mut report = ComplianceReport::new("test-vm", ComplianceFramework::HIPAA);

        report.add_result(CheckResult::new("C1", "CTRL1", "Test 1").passed());
        report.add_result(CheckResult::new("C2", "CTRL2", "Test 2").passed());
        report.add_result(CheckResult::new("C3", "CTRL3", "Test 3").passed());
        report.add_result(CheckResult::new("C4", "CTRL4", "Test 4").failed("Error"));

        report.finalize();

        assert_eq!(report.summary.compliance_score, 75.0); // 3 out of 4
        assert!(!report.compliant);
    }

    #[test]
    fn test_check_result() {
        let passed = CheckResult::new("C1", "CTRL1", "Test")
            .with_severity(CheckSeverity::High)
            .with_evidence("Log file shows compliance")
            .passed();

        assert_eq!(passed.status, CheckStatus::Passed);
        assert_eq!(passed.severity, CheckSeverity::High);
        assert!(passed.evidence.is_some());

        let failed = CheckResult::new("C2", "CTRL2", "Test").failed("Firewall not configured");

        assert_eq!(failed.status, CheckStatus::Failed);
    }

    #[test]
    fn test_pci_dss_check_minimal_vm_fails() {
        let vm = make_minimal_vm();
        let report = ComplianceChecker::check_pci_dss("test-vm", &vm);

        assert_eq!(report.framework, ComplianceFramework::PCIDSS);
        assert_eq!(report.summary.total_checks, 4);
        // Minimal VM should fail all 4 checks
        assert_eq!(report.summary.failed, 4);
        assert_eq!(report.summary.passed, 0);
        assert!(!report.compliant);
    }

    #[test]
    fn test_pci_dss_check_compliant_vm_passes() {
        let vm = make_compliant_vm();
        let report = ComplianceChecker::check_pci_dss("test-vm", &vm);

        assert_eq!(report.framework, ComplianceFramework::PCIDSS);
        assert_eq!(report.summary.total_checks, 4);
        assert_eq!(report.summary.passed, 4);
        assert_eq!(report.summary.failed, 0);
        assert!(report.compliant);
    }

    #[test]
    fn test_hipaa_check_minimal_vm_fails() {
        let vm = make_minimal_vm();
        let report = ComplianceChecker::check_hipaa("test-vm", &vm);

        assert_eq!(report.framework, ComplianceFramework::HIPAA);
        assert_eq!(report.summary.total_checks, 3);
        assert_eq!(report.summary.failed, 3);
        assert_eq!(report.summary.passed, 0);
        assert!(!report.compliant);
    }

    #[test]
    fn test_hipaa_check_compliant_vm_passes() {
        let vm = make_compliant_vm();
        let report = ComplianceChecker::check_hipaa("test-vm", &vm);

        assert_eq!(report.framework, ComplianceFramework::HIPAA);
        assert_eq!(report.summary.total_checks, 3);
        assert_eq!(report.summary.passed, 3);
        assert_eq!(report.summary.failed, 0);
        assert!(report.compliant);
    }

    #[test]
    fn test_soc2_check_minimal_vm_fails() {
        let vm = make_minimal_vm();
        let report = ComplianceChecker::check_soc2("test-vm", &vm);

        assert_eq!(report.framework, ComplianceFramework::SOC2);
        assert_eq!(report.summary.total_checks, 3);
        assert_eq!(report.summary.failed, 3);
        assert_eq!(report.summary.passed, 0);
        assert!(!report.compliant);
    }

    #[test]
    fn test_soc2_check_compliant_vm_passes() {
        let vm = make_compliant_vm();
        let report = ComplianceChecker::check_soc2("test-vm", &vm);

        assert_eq!(report.framework, ComplianceFramework::SOC2);
        assert_eq!(report.summary.total_checks, 3);
        assert_eq!(report.summary.passed, 3);
        assert_eq!(report.summary.failed, 0);
        assert!(report.compliant);
    }

    #[test]
    fn test_check_all() {
        let vm = make_minimal_vm();
        let reports = ComplianceChecker::check_all("test-vm", &vm);

        assert_eq!(reports.len(), 3);
    }

    #[test]
    fn test_critical_failures() {
        let mut report = ComplianceReport::new("test-vm", ComplianceFramework::PCIDSS);

        report.add_result(
            CheckResult::new("C1", "CTRL1", "Test 1")
                .with_severity(CheckSeverity::Critical)
                .failed("Critical issue"),
        );
        report.add_result(
            CheckResult::new("C2", "CTRL2", "Test 2")
                .with_severity(CheckSeverity::High)
                .failed("High issue"),
        );

        let critical = report.critical_failures();
        assert_eq!(critical.len(), 1);
    }

    #[test]
    fn test_framework_display() {
        assert_eq!(ComplianceFramework::PCIDSS.to_string(), "PCI-DSS");
        assert_eq!(ComplianceFramework::HIPAA.to_string(), "HIPAA");
        assert_eq!(ComplianceFramework::SOC2.to_string(), "SOC 2");
    }

    #[test]
    fn test_check_status_display() {
        assert_eq!(CheckStatus::Passed.to_string(), "Passed");
        assert_eq!(CheckStatus::Failed.to_string(), "Failed");
        assert_eq!(CheckStatus::NotApplicable.to_string(), "N/A");
    }
}
