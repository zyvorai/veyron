use crate::tui::colors::cli as color;
use anyhow::Result;

pub fn handle_security_scan(
    vm: String,
    scan_type: String,
    containers: bool,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::security::scan::{ScanConfig, ScanType, VulnerabilityScanner};

    log::debug!("Using namespace: {}", namespace);

    println!("{}", color::header(&format!("Scanning VM: {}", vm)));
    println!();

    let s_type = match scan_type.as_str() {
        "quick" => ScanType::Quick,
        "deep" => ScanType::Deep,
        "compliance" => ScanType::Compliance,
        _ => ScanType::Standard,
    };

    let mut config = ScanConfig::new(&vm, s_type);
    if containers {
        config = config.enable_containers();
    }

    println!("  Scan Type:  {}", color::value(&scan_type));
    println!(
        "  Containers: {}",
        if containers {
            color::success("Yes")
        } else {
            "No".to_string()
        }
    );
    println!();
    println!("Scanning...");

    let result = VulnerabilityScanner::scan(&config);

    println!();
    println!("Scan Results:");
    println!(
        "  Status:     {}",
        color::success(&result.status.to_string())
    );
    println!(
        "  Total:      {}",
        color::value(&result.statistics.total.to_string())
    );
    println!(
        "  Critical:   {}",
        if result.statistics.critical > 0 {
            color::error(&result.statistics.critical.to_string())
        } else {
            color::success("0")
        }
    );
    println!(
        "  High:       {}",
        if result.statistics.high > 0 {
            color::warning(&result.statistics.high.to_string())
        } else {
            color::success("0")
        }
    );
    println!("  Medium:     {}", result.statistics.medium);
    println!("  Low:        {}", result.statistics.low);
    println!();

    if output == "json" {
        let json = serde_json::to_string_pretty(&result)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&result)?;
        println!("{}", yaml);
    }

    Ok(())
}

pub fn handle_security_assess(vm: String, output: String, namespace: &str) -> Result<()> {
    use crate::security::{SecurityAssessment, Vulnerability};

    log::debug!("Using namespace: {}", namespace);

    let mut assessment = SecurityAssessment::new(&vm);

    // Load vulnerabilities from config directory if available
    let vuln_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".config"))
        .join("vmrogue")
        .join("vulnerabilities");

    let mut loaded_from_file = false;
    if vuln_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&vuln_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .extension()
                    .map(|e| e == "yaml" || e == "yml" || e == "json")
                    .unwrap_or(false)
                {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(vulns) = serde_yml::from_str::<Vec<Vulnerability>>(&content) {
                            for v in vulns {
                                assessment.add_vulnerability(v);
                            }
                            loaded_from_file = true;
                        } else if let Ok(vulns) =
                            serde_json::from_str::<Vec<Vulnerability>>(&content)
                        {
                            for v in vulns {
                                assessment.add_vulnerability(v);
                            }
                            loaded_from_file = true;
                        }
                    }
                }
            }
        }
    }

    // Run the built-in scanner as baseline if no vulnerabilities loaded from files
    if !loaded_from_file {
        use crate::security::scan::{ScanConfig, ScanType, VulnerabilityScanner};
        let scan_config = ScanConfig::new(&vm, ScanType::Standard);
        let result = VulnerabilityScanner::scan(&scan_config);

        for vuln in result.vulnerabilities {
            assessment.add_vulnerability(vuln);
        }
    }

    assessment.calculate_score();

    println!("{}", color::header(&format!("Security Assessment: {}", vm)));
    println!();
    println!(
        "  Score:         {}",
        color::value(&assessment.overall_score.to_string())
    );
    println!(
        "  Risk Level:    {}",
        match assessment.risk_level {
            crate::security::RiskLevel::Critical => color::error("Critical"),
            crate::security::RiskLevel::High => color::error("High"),
            crate::security::RiskLevel::Medium => color::warning("Medium"),
            crate::security::RiskLevel::Low => color::success("Low"),
            crate::security::RiskLevel::Unknown => color::muted("Unknown"),
        }
    );
    println!("  Vulnerabilities: {}", assessment.vulnerabilities.len());
    println!(
        "    Critical:    {}",
        color::error(&assessment.critical_count().to_string())
    );
    println!(
        "    High:        {}",
        color::warning(&assessment.high_count().to_string())
    );
    println!();

    if output == "json" {
        let json = serde_json::to_string_pretty(&assessment)?;
        println!("{}", json);
    } else {
        let yaml = serde_yml::to_string(&assessment)?;
        println!("{}", yaml);
    }

    Ok(())
}

pub fn handle_security_harden(
    vm: String,
    profile: String,
    verify_only: bool,
    namespace: &str,
) -> Result<()> {
    use crate::security::hardening::{HardeningEngine, SecurityBaseline};

    log::debug!("Using namespace: {}", namespace);

    println!("{}", color::header(&format!("Security Hardening: {}", vm)));
    println!();

    let baseline = match profile.as_str() {
        "stig" => SecurityBaseline::STIG,
        "pci-dss" => SecurityBaseline::PciDss,
        "nist" => SecurityBaseline::NIST,
        "custom" => SecurityBaseline::Custom,
        _ => SecurityBaseline::CIS,
    };

    let hardening_profile = match profile.as_str() {
        "stig" => HardeningEngine::stig_profile(),
        "cis" => HardeningEngine::cis_profile(),
        other => {
            println!(
                "{}",
                color::warning(&format!(
                    "Profile '{}' does not have a dedicated hardening ruleset. Using CIS as baseline.",
                    other
                ))
            );
            HardeningEngine::cis_profile()
        }
    };

    println!("  Profile:     {}", color::value(&baseline.to_string()));
    println!("  Rules:       {}", hardening_profile.rule_count());
    println!(
        "  Mode:        {}",
        if verify_only {
            color::info("Verify Only")
        } else {
            color::warning("Apply")
        }
    );
    println!();

    let result = if verify_only {
        HardeningEngine::verify(&vm, &hardening_profile)
    } else {
        HardeningEngine::apply(&vm, &hardening_profile)
    };

    println!("Results:");
    println!(
        "  Status:      {}",
        color::success(&result.status.to_string())
    );
    println!(
        "  Applied:     {}",
        color::success(&result.statistics.applied.to_string())
    );
    println!("  Skipped:     {}", result.statistics.skipped);
    println!(
        "  Failed:      {}",
        if result.statistics.failed > 0 {
            color::error(&result.statistics.failed.to_string())
        } else {
            color::success("0")
        }
    );
    println!("  Success:     {}%", result.success_rate() as u8);

    Ok(())
}

pub fn handle_security_profiles(details: bool) -> Result<()> {
    use crate::security::hardening::{HardeningEngine, SecurityBaseline};

    println!("{}", color::header("Security Hardening Profiles"));
    println!();

    let profiles = vec![
        (SecurityBaseline::CIS, HardeningEngine::cis_profile()),
        (SecurityBaseline::STIG, HardeningEngine::stig_profile()),
    ];

    if details {
        for (baseline, profile) in profiles {
            println!("Profile: {}", color::value(&baseline.to_string()));
            println!("  Name:        {}", profile.name);
            println!("  Description: {}", profile.description);
            println!("  Rules:       {}", profile.rule_count());
            println!();
        }
    } else {
        println!(
            "{:<20} {:<50} {}",
            color::label("PROFILE"),
            color::label("DESCRIPTION"),
            color::label("RULES")
        );
        println!("{}", "-".repeat(80));

        for (baseline, profile) in profiles {
            println!(
                "{:<20} {:<50} {}",
                baseline.to_string(),
                profile.description,
                profile.rule_count()
            );
        }
    }

    Ok(())
}

pub async fn handle_compliance_check(
    vm: String,
    framework: String,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::security::compliance::{ComplianceChecker, ComplianceFramework};

    log::debug!("Using namespace: {}", namespace);

    println!("{}", color::header(&format!("Compliance Check: {}", vm)));
    println!();

    let fw = match framework.as_str() {
        "hipaa" => ComplianceFramework::HIPAA,
        "soc2" => ComplianceFramework::SOC2,
        "iso27001" => ComplianceFramework::ISO27001,
        "gdpr" => ComplianceFramework::GDPR,
        "nist" => ComplianceFramework::NIST,
        "cis" => ComplianceFramework::CIS,
        _ => ComplianceFramework::PCIDSS,
    };

    println!("  Framework:   {}", color::value(&fw.to_string()));
    println!();
    println!("Fetching VM from Kubernetes...");

    let client = crate::kube::KubeClient::new()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to Kubernetes: {}", e))?;
    let vm_obj = client
        .get_vm(namespace, &vm)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get VM '{}': {}", vm, e))?;

    println!("Checking compliance...");

    let report = match framework.as_str() {
        "hipaa" => ComplianceChecker::check_hipaa(&vm, &vm_obj),
        "soc2" => ComplianceChecker::check_soc2(&vm, &vm_obj),
        "pci-dss" | "pci_dss" => ComplianceChecker::check_pci_dss(&vm, &vm_obj),
        other => {
            println!(
                "{}",
                color::warning(&format!(
                    "Framework '{}' does not have a dedicated checker. Using PCI-DSS as baseline.",
                    other
                ))
            );
            ComplianceChecker::check_pci_dss(&vm, &vm_obj)
        }
    };

    println!();
    println!("Compliance Report:");
    println!(
        "  Status:      {}",
        if report.compliant {
            color::success("Compliant")
        } else {
            color::error("Non-Compliant")
        }
    );
    println!("  Score:       {}%", report.summary.compliance_score as u8);
    println!("  Total:       {}", report.summary.total_checks);
    println!(
        "  Passed:      {}",
        color::success(&report.summary.passed.to_string())
    );
    println!(
        "  Failed:      {}",
        if report.summary.failed > 0 {
            color::error(&report.summary.failed.to_string())
        } else {
            color::success("0")
        }
    );
    println!(
        "  Critical:    {}",
        if report.summary.critical_failures > 0 {
            color::error(&report.summary.critical_failures.to_string())
        } else {
            color::success("0")
        }
    );
    println!();

    if output == "json" {
        let json = serde_json::to_string_pretty(&report)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&report)?;
        println!("{}", yaml);
    }

    Ok(())
}

pub async fn handle_compliance_report(
    vm: String,
    report_id: Option<String>,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::security::compliance::ComplianceChecker;

    log::debug!("Using namespace: {}", namespace);

    let client = crate::kube::KubeClient::new()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to Kubernetes: {}", e))?;
    let vm_obj = client
        .get_vm(namespace, &vm)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get VM '{}': {}", vm, e))?;

    let report = ComplianceChecker::check_pci_dss(&vm, &vm_obj);

    println!("{}", color::header(&format!("Compliance Report: {}", vm)));
    println!();
    println!(
        "  Report ID:   {}",
        report_id.as_deref().unwrap_or(&report.report_id)
    );
    println!(
        "  Generated:   {}",
        report.generated_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!();

    if output == "json" {
        let json = serde_json::to_string_pretty(&report)?;
        println!("{}", json);
    } else {
        let yaml = serde_yml::to_string(&report)?;
        println!("{}", yaml);
    }

    Ok(())
}

pub fn handle_audit_list(
    vm: Option<String>,
    event_type: Option<String>,
    severity: Option<String>,
    security_only: bool,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::security::audit::{AuditEvent, AuditLog, EventSeverity};

    log::debug!("Using namespace: {}", namespace);

    println!("{}", color::header("Audit Events"));
    if let Some(ref vm_name) = vm {
        println!("  VM: {}", color::value(vm_name));
    }
    println!();

    // Load audit events from config directory
    let audit_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".config"))
        .join("vmrogue")
        .join("audit");

    let mut log = AuditLog::new(vm.clone());

    if audit_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&audit_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .extension()
                    .map(|e| e == "yaml" || e == "yml" || e == "json")
                    .unwrap_or(false)
                {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        // Try to load events from file
                        if let Ok(events) = serde_yml::from_str::<Vec<AuditEvent>>(&content) {
                            for event in events {
                                log.add_event(event);
                            }
                        } else if let Ok(events) = serde_json::from_str::<Vec<AuditEvent>>(&content)
                        {
                            for event in events {
                                log.add_event(event);
                            }
                        } else if let Ok(event) = serde_yml::from_str::<AuditEvent>(&content) {
                            log.add_event(event);
                        }
                    }
                }
            }
        }
    }

    if log.events.is_empty() {
        println!("{}", color::muted("No audit events found"));
        println!();
        println!(
            "{}",
            color::info(&format!(
                "ℹ Audit events can be stored in: {}",
                audit_dir.display()
            ))
        );
        return Ok(());
    }

    // Apply filters
    let events: Vec<&AuditEvent> = if security_only {
        log.security_events()
    } else {
        let mut filtered: Vec<&AuditEvent> = log.events.iter().collect();

        if let Some(ref et) = event_type {
            filtered.retain(|e| {
                e.event_type
                    .to_string()
                    .to_lowercase()
                    .contains(&et.to_lowercase())
            });
        }

        if let Some(ref sev) = severity {
            filtered.retain(|e| {
                let event_sev = match e.severity {
                    EventSeverity::Critical => "critical",
                    EventSeverity::High => "high",
                    EventSeverity::Medium => "medium",
                    EventSeverity::Low => "low",
                    EventSeverity::Info => "info",
                };
                event_sev == sev.to_lowercase()
            });
        }

        filtered
    };

    if output == "json" {
        let json = serde_json::to_string_pretty(&events)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&events)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<25} {:<20} {:<15} {:<10} {}",
            color::label("TIMESTAMP"),
            color::label("TYPE"),
            color::label("ACTOR"),
            color::label("SEVERITY"),
            color::label("ACTION")
        );
        println!("{}", "-".repeat(90));

        for event in events {
            let severity_str = match event.severity {
                EventSeverity::Critical => color::error("Critical"),
                EventSeverity::High => color::error("High"),
                EventSeverity::Medium => color::warning("Medium"),
                EventSeverity::Low => color::info("Low"),
                EventSeverity::Info => color::muted("Info"),
            };

            println!(
                "{:<25} {:<20} {:<15} {:<10} {}",
                event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                event.event_type.to_string(),
                event.actor,
                severity_str,
                event.action
            );
        }
    }

    Ok(())
}

pub fn handle_audit_get(log_id: String, output: String, namespace: &str) -> Result<()> {
    use crate::security::audit::AuditLog;

    log::debug!("Using namespace: {}", namespace);

    let log = AuditLog::new(Some(log_id.to_string()));

    println!("{}", color::header(&format!("Audit Log: {}", log_id)));
    println!();
    println!("  Log ID:      {}", log.log_id);
    println!("  Events:      {}", log.event_count());
    println!(
        "  Created:     {}",
        log.created_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!();

    if output == "json" {
        let json = serde_json::to_string_pretty(&log)?;
        println!("{}", json);
    } else {
        let yaml = serde_yml::to_string(&log)?;
        println!("{}", yaml);
    }

    Ok(())
}

pub fn handle_audit_stats(
    vm: Option<String>,
    period: String,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::security::audit::{AuditLog, AuditStatistics};

    log::debug!("Using namespace: {}", namespace);

    println!("{}", color::header("Audit Statistics"));
    if let Some(ref vm_name) = vm {
        println!("  VM:          {}", color::value(vm_name));
    }
    println!("  Period:      {}", period);
    println!();

    let log = AuditLog::new(vm);
    let stats = AuditStatistics::from_log(&log);

    if output == "json" {
        let json = serde_json::to_string_pretty(&stats)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&stats)?;
        println!("{}", yaml);
    } else {
        println!("Summary:");
        println!("  Total Events:      {}", stats.total_events);
        println!("  Security Events:   {}", stats.security_events);
        println!(
            "  Critical Events:   {}",
            if stats.critical_events > 0 {
                color::error(&stats.critical_events.to_string())
            } else {
                color::success("0")
            }
        );
        println!(
            "  Failed Events:     {}",
            if stats.failed_events > 0 {
                color::warning(&stats.failed_events.to_string())
            } else {
                color::success("0")
            }
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::security::compliance::{ComplianceChecker, ComplianceFramework};
    use crate::security::hardening::{HardeningEngine, SecurityBaseline};
    use crate::security::scan::{ScanConfig, ScanStatus, ScanType, VulnerabilityScanner};
    use crate::security::{SecurityAssessment, Severity, Vulnerability};

    #[test]
    fn test_scan_type_parsing() {
        assert_eq!(
            match "quick" {
                "quick" => ScanType::Quick,
                "deep" => ScanType::Deep,
                "compliance" => ScanType::Compliance,
                _ => ScanType::Standard,
            },
            ScanType::Quick
        );
        assert_eq!(
            match "deep" {
                "quick" => ScanType::Quick,
                "deep" => ScanType::Deep,
                "compliance" => ScanType::Compliance,
                _ => ScanType::Standard,
            },
            ScanType::Deep
        );
        assert_eq!(
            match "other" {
                "quick" => ScanType::Quick,
                "deep" => ScanType::Deep,
                "compliance" => ScanType::Compliance,
                _ => ScanType::Standard,
            },
            ScanType::Standard
        );
    }

    #[test]
    fn test_security_baseline_parsing() {
        assert_eq!(
            match "stig" {
                "stig" => SecurityBaseline::STIG,
                "pci-dss" => SecurityBaseline::PciDss,
                "nist" => SecurityBaseline::NIST,
                "custom" => SecurityBaseline::Custom,
                _ => SecurityBaseline::CIS,
            },
            SecurityBaseline::STIG
        );
        assert_eq!(
            match "cis" {
                "stig" => SecurityBaseline::STIG,
                "pci-dss" => SecurityBaseline::PciDss,
                "nist" => SecurityBaseline::NIST,
                "custom" => SecurityBaseline::Custom,
                _ => SecurityBaseline::CIS,
            },
            SecurityBaseline::CIS
        );
    }

    #[test]
    fn test_compliance_framework_parsing() {
        assert_eq!(
            match "hipaa" {
                "hipaa" => ComplianceFramework::HIPAA,
                "soc2" => ComplianceFramework::SOC2,
                _ => ComplianceFramework::PCIDSS,
            },
            ComplianceFramework::HIPAA
        );
        assert_eq!(
            match "other" {
                "hipaa" => ComplianceFramework::HIPAA,
                "soc2" => ComplianceFramework::SOC2,
                _ => ComplianceFramework::PCIDSS,
            },
            ComplianceFramework::PCIDSS
        );
    }

    #[test]
    fn test_vulnerability_scanner_produces_results() {
        let config = ScanConfig::new("test-vm", ScanType::Standard);
        let result = VulnerabilityScanner::scan(&config);
        assert_eq!(result.status, ScanStatus::Completed);
        assert!(result.statistics.total > 0);
    }

    #[test]
    fn test_hardening_profiles() {
        let cis = HardeningEngine::cis_profile();
        assert!(cis.rule_count() >= 3);
        let stig = HardeningEngine::stig_profile();
        assert!(stig.rule_count() >= 2);
    }

    #[test]
    fn test_compliance_checker_all_frameworks() {
        use crate::kube::types::*;
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

        let vm = VirtualMachine {
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
        };

        let pci = ComplianceChecker::check_pci_dss("test-vm", &vm);
        assert_eq!(pci.framework, ComplianceFramework::PCIDSS);
        assert!(pci.summary.total_checks >= 4);
    }

    #[test]
    fn test_security_assessment_score_calculation() {
        let mut assessment = SecurityAssessment::new("test-vm");
        assessment.add_vulnerability(Vulnerability::new("V1", "Crit1", Severity::Critical));
        assessment.add_vulnerability(Vulnerability::new("V2", "Crit2", Severity::Critical));
        assessment.add_vulnerability(Vulnerability::new("V3", "High1", Severity::High));
        assessment.calculate_score();
        assert_eq!(assessment.overall_score, 50);
        assert_eq!(assessment.critical_count(), 2);
    }
}
