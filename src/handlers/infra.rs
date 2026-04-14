use crate::output::{OutputFormat, format_output};
use crate::tui::colors::cli as color;
use anyhow::{Result, anyhow};

/// Classification of disk usage level based on percentage thresholds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DiskUsageLevel {
    Normal,
    Warning,
    Critical,
}

/// Classify disk usage percentage into Normal, Warning, or Critical.
/// Critical threshold: >= 90%, Warning threshold: >= 75%.
pub(crate) fn classify_disk_usage(usage_percent: f64) -> DiskUsageLevel {
    if usage_percent >= 90.0 {
        DiskUsageLevel::Critical
    } else if usage_percent >= 75.0 {
        DiskUsageLevel::Warning
    } else {
        DiskUsageLevel::Normal
    }
}

/// Derive a VM name from a snapshot name by stripping a trailing "-snapshot" or "-snapshot-TIMESTAMP" suffix.
pub(crate) fn derive_vm_name_from_snapshot(snapshot: &str) -> String {
    // Strip trailing -snapshot-TIMESTAMP or -snapshot suffix
    if let Some(pos) = snapshot.rfind("-snapshot") {
        snapshot[..pos].to_string()
    } else {
        snapshot.to_string()
    }
}

/// Generate a default restore target name from a snapshot name.
pub(crate) fn default_restore_target_name(snapshot: &str) -> String {
    format!("{}-restored", snapshot)
}

// ========== SNAPSHOT HANDLERS ==========

pub async fn handle_snapshot_create(
    vm: String,
    name: Option<String>,
    description: Option<String>,
    namespace: &str,
) -> Result<()> {
    use crate::snapshots::{SnapshotConfig, SnapshotManager};
    use chrono::Utc;

    let manager = SnapshotManager::new(namespace).await?;

    // Auto-generate snapshot name if not provided
    let snapshot_name =
        name.unwrap_or_else(|| format!("{}-snapshot-{}", vm, Utc::now().format("%Y%m%d-%H%M%S")));

    let mut config = SnapshotConfig::new(&vm, &snapshot_name);
    if let Some(desc) = description {
        config = config.with_description(desc);
    }

    println!(
        "{}",
        color::header(&format!("Creating snapshot for VM: {}", vm))
    );
    println!("  Snapshot name: {}", color::value(&snapshot_name));
    if let Some(desc) = &config.description {
        println!("  Description:   {}", color::muted(desc));
    }
    println!();

    match manager.create_snapshot(&config).await {
        Ok(_snapshot) => {
            println!("{} Snapshot creation started", color::success("✓"));
            println!("  Status:    {}", color::vm_status("InProgress"));
            println!();
            println!("{}", color::info("Check snapshot status with:"));
            println!(
                "  {}",
                color::command(&format!("vmrogue snapshot-get {}", snapshot_name))
            );
        }
        Err(e) => {
            println!("{} Failed to create snapshot: {}", color::error("✗"), e);
            return Err(e);
        }
    }
    Ok(())
}

pub async fn handle_snapshot_list(
    vm: Option<String>,
    _all_namespaces: bool,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::snapshots::{self, SnapshotManager};

    let manager = SnapshotManager::new(namespace).await?;

    let snapshots = if let Some(vm_name) = vm {
        println!(
            "{}",
            color::header(&format!("Snapshots for VM: {}", vm_name))
        );
        manager.list_snapshots_for_vm(&vm_name).await?
    } else {
        println!(
            "{}",
            color::header(&format!(
                "All Snapshots in namespace: {}",
                color::namespace(namespace)
            ))
        );
        manager.list_all_snapshots().await?
    };

    if snapshots.is_empty() {
        println!("{}", color::muted("No snapshots found"));
        return Ok(());
    }

    if output == "table" {
        println!();
        println!(
            "{:<30} {:<20} {:<12} {:<10} {:<10}",
            color::label("NAME"),
            color::label("VM"),
            color::label("STATUS"),
            color::label("SIZE"),
            color::label("AGE")
        );
        println!("{}", "-".repeat(82));

        for snapshot in &snapshots {
            let status_str = match snapshot.status {
                snapshots::SnapshotStatus::Succeeded => color::vm_status("Succeeded"),
                snapshots::SnapshotStatus::InProgress => color::vm_status("InProgress"),
                snapshots::SnapshotStatus::Failed => color::vm_status("Failed"),
                snapshots::SnapshotStatus::Unknown => color::vm_status("Unknown"),
            };

            println!(
                "{:<30} {:<20} {:<12} {:<10} {:<10}",
                snapshot.name,
                snapshot.vm_name,
                status_str,
                snapshot.size.as_deref().unwrap_or("-"),
                snapshot.age()
            );
        }
    } else {
        let output_format = if output == "json" {
            OutputFormat::Json
        } else {
            OutputFormat::Yaml
        };
        let formatted = format_output(&snapshots, output_format)?;
        println!("{}", formatted);
    }
    Ok(())
}

pub async fn handle_snapshot_get(name: String, output: String, namespace: &str) -> Result<()> {
    use crate::snapshots::{self, SnapshotManager};

    let manager = SnapshotManager::new(namespace).await?;
    let snapshot = manager.get_snapshot(&name).await?;

    if output == "yaml" || output == "json" {
        let output_format = if output == "json" {
            OutputFormat::Json
        } else {
            OutputFormat::Yaml
        };
        let formatted = format_output(&snapshot, output_format)?;
        println!("{}", formatted);
    } else {
        println!("{}", color::header(&format!("Snapshot: {}", name)));
        println!();
        println!("  VM:          {}", color::value(&snapshot.vm_name));
        println!("  Namespace:   {}", color::namespace(&snapshot.namespace));

        let status_str = match snapshot.status {
            snapshots::SnapshotStatus::Succeeded => color::success("✓ READY"),
            snapshots::SnapshotStatus::InProgress => color::warning("◐ IN PROGRESS"),
            snapshots::SnapshotStatus::Failed => color::error("✗ FAILED"),
            snapshots::SnapshotStatus::Unknown => color::muted("? UNKNOWN"),
        };
        println!("  Status:      {}", status_str);

        if let Some(desc) = &snapshot.description {
            println!("  Description: {}", color::muted(desc));
        }
        if let Some(size) = &snapshot.size {
            println!("  Size:        {}", color::resource(size, "storage"));
        }
        println!("  Age:         {}", snapshot.age());

        if let Some(duration) = snapshot.duration() {
            println!("  Duration:    {}", duration);
        }

        println!(
            "  Ready:       {}",
            if snapshot.ready_to_use {
                color::success("Yes")
            } else {
                color::muted("No")
            }
        );

        if let Some(error) = &snapshot.error {
            println!("  Error:       {}", color::error(error));
        }
    }
    Ok(())
}

pub async fn handle_snapshot_delete(name: String, yes: bool, namespace: &str) -> Result<()> {
    use crate::snapshots::SnapshotManager;

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!("Delete snapshot '{}'?", name))
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            println!("{}", color::muted("Cancelled"));
            return Ok(());
        }
    }

    let manager = SnapshotManager::new(namespace).await?;

    println!("{}", color::header(&format!("Deleting snapshot: {}", name)));
    match manager.delete_snapshot(&name).await {
        Ok(_) => {
            println!("{} Snapshot deleted successfully", color::success("✓"));
        }
        Err(e) => {
            println!("{} Failed to delete snapshot: {}", color::error("✗"), e);
            return Err(e);
        }
    }
    Ok(())
}

pub async fn handle_snapshot_restore(
    snapshot: String,
    target: Option<String>,
    in_place: bool,
    start: bool,
    namespace: &str,
) -> Result<()> {
    use crate::snapshots::RestoreManager;

    let manager = RestoreManager::new(namespace).await?;

    if in_place {
        // Restore in-place (overwrite existing VM)
        let default_vm = derive_vm_name_from_snapshot(&snapshot);
        let vm_name = target.as_deref().unwrap_or(&default_vm);

        println!(
            "{}",
            color::header(&format!("Restoring VM in-place: {}", vm_name))
        );
        println!(
            "{}",
            color::warning("⚠ This will overwrite the current VM state")
        );
        println!();

        match manager.restore_in_place(vm_name, &snapshot).await {
            Ok(restore) => {
                println!("{} Restore started", color::success("✓"));
                println!("  Restore name: {}", restore.name);
                println!("  Status:       {}", color::vm_status("InProgress"));
            }
            Err(e) => {
                println!("{} Failed to restore: {}", color::error("✗"), e);
                return Err(e);
            }
        }
    } else {
        // Restore to new VM
        let default_target = default_restore_target_name(&snapshot);
        let target_vm = target.as_deref().unwrap_or(&default_target);

        println!(
            "{}",
            color::header(&format!("Restoring snapshot to new VM: {}", target_vm))
        );
        println!("  Snapshot:  {}", color::value(&snapshot));
        println!("  Target VM: {}", color::value(target_vm));
        if start {
            println!("  Start:     {}", color::success("Yes"));
        }
        println!();

        match manager.restore_to_new_vm(&snapshot, target_vm, start).await {
            Ok(restore) => {
                println!("{} Restore started", color::success("✓"));
                println!("  Restore name: {}", restore.name);
                println!("  Status:       {}", color::vm_status("InProgress"));
                println!();
                if start {
                    println!(
                        "{}",
                        color::info("VM will be started after restore completes")
                    );
                }
            }
            Err(e) => {
                println!("{} Failed to restore: {}", color::error("✗"), e);
                return Err(e);
            }
        }
    }
    Ok(())
}

// ========== MONITORING HANDLERS ==========

pub async fn handle_monitor_live(vm: String, interval: u64, namespace: &str) -> Result<()> {
    use crate::monitoring::{MetricsCollector, MonitoringReporter};

    let collector = MetricsCollector::new(namespace);
    let reporter = MonitoringReporter::new();

    println!(
        "{}",
        color::header(&format!("Live Monitoring: {} (Press Ctrl+C to stop)", vm))
    );
    println!(
        "{}",
        color::muted(&format!("Update interval: {} seconds", interval))
    );
    println!();

    // Monitor until interrupted (Ctrl+C) or connection fails
    let mut first = true;
    loop {
        if !first {
            println!("\n{}", "═".repeat(80));
        }
        first = false;

        match collector.collect(&vm).await {
            Ok(metrics) => {
                println!("{}", reporter.format_live_metrics(&vm, &metrics));
            }
            Err(e) => {
                println!("{} Failed to collect metrics: {}", color::error("✗"), e);
                break;
            }
        }

        // Wait for interval or Ctrl+C
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval)) => {}
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }

    println!();
    println!("{}", color::info("ℹ Live monitoring stopped"));
    Ok(())
}

pub async fn handle_monitor_stats(
    vm: String,
    period: String,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::monitoring::{
        MetricsCollector, MonitoringReporter, PerformanceAnalyzer, ReportFormat,
    };

    let collector = MetricsCollector::new(namespace);
    let analyzer = PerformanceAnalyzer::with_default_thresholds();
    let reporter = MonitoringReporter::new();

    println!(
        "{}",
        color::header(&format!("Performance Statistics: {}", vm))
    );
    println!("  Period: {}", color::value(&period));
    println!();

    match collector.collect(&vm).await {
        Ok(metrics) => {
            let report = analyzer.analyze(&vm, &metrics);

            let format = match output.as_str() {
                "json" => ReportFormat::Json,
                "yaml" => ReportFormat::Yaml,
                "summary" => ReportFormat::Summary,
                _ => ReportFormat::Table,
            };

            match reporter.format_performance_report(&report, &format) {
                Ok(formatted) => println!("{}", formatted),
                Err(e) => {
                    println!("{} Failed to format report: {}", color::error("✗"), e);
                    return Err(e);
                }
            }
        }
        Err(e) => {
            println!("{} Failed to collect metrics: {}", color::error("✗"), e);
            return Err(e);
        }
    }
    Ok(())
}

pub async fn handle_monitor_compare(
    vms: Vec<String>,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::monitoring::{MetricsCollector, MonitoringReporter, PerformanceAnalyzer};

    if vms.len() < 2 {
        println!(
            "{} At least 2 VMs are required for comparison",
            color::error("✗")
        );
        return Err(anyhow!("Need at least 2 VMs"));
    }

    let collector = MetricsCollector::new(namespace);
    let analyzer = PerformanceAnalyzer::with_default_thresholds();
    let reporter = MonitoringReporter::new();

    println!("{}", color::header(&format!("Comparing {} VMs", vms.len())));
    println!();

    let mut reports = Vec::new();

    for vm_name in &vms {
        match collector.collect(vm_name).await {
            Ok(metrics) => {
                let report = analyzer.analyze(vm_name, &metrics);
                reports.push(report);
            }
            Err(e) => {
                println!(
                    "{} Failed to collect metrics for {}: {}",
                    color::error("✗"),
                    vm_name,
                    e
                );
            }
        }
    }

    if reports.is_empty() {
        println!("{} No metrics collected", color::error("✗"));
        return Ok(());
    }

    if output == "json" {
        let json = serde_json::to_string_pretty(&reports)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&reports)?;
        println!("{}", yaml);
    } else {
        // Table format
        let comparison = analyzer.compare(reports);
        println!("{}", reporter.format_comparison(comparison));
    }
    Ok(())
}

pub async fn handle_monitor_top(
    all_namespaces: bool,
    sort_by: String,
    limit: usize,
    namespace: &str,
) -> Result<()> {
    use crate::kube::KubeClient;
    use crate::monitoring::{MetricsCollector, MonitoringReporter, PerformanceAnalyzer};

    // Query real VMs from Kubernetes
    let vm_names: Vec<String> = match KubeClient::new().await {
        Ok(client) => {
            let vms = if all_namespaces {
                client
                    .list_all_vms()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to list VMs: {}", e))?
            } else {
                client
                    .list_vms(namespace)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to list VMs: {}", e))?
            };
            vms.iter()
                .filter_map(|vm| vm.metadata.name.clone())
                .collect()
        }
        Err(_) => {
            println!(
                "{}",
                color::warning("⚠ Could not connect to Kubernetes cluster")
            );
            println!(
                "{}",
                color::muted("  Showing no VMs. Ensure kubeconfig is configured.")
            );
            return Ok(());
        }
    };

    if vm_names.is_empty() {
        println!("{}", color::muted("No VMs found in namespace"));
        return Ok(());
    }

    let collector = MetricsCollector::new(namespace);
    let analyzer = PerformanceAnalyzer::with_default_thresholds();
    let reporter = MonitoringReporter::new();

    println!(
        "{}",
        color::header(&format!(
            "Top {} VMs by {}",
            limit.min(vm_names.len()),
            sort_by
        ))
    );
    println!();

    let mut reports = Vec::new();

    for vm_name in vm_names.iter() {
        if let Ok(metrics) = collector.collect(vm_name).await {
            let report = analyzer.analyze(vm_name, &metrics);
            reports.push(report);
        }
    }

    // Sort based on sort_by parameter
    reports.sort_by(|a, b| {
        match sort_by.as_str() {
            "cpu" => b
                .current_metrics
                .cpu
                .usage_percent
                .partial_cmp(&a.current_metrics.cpu.usage_percent)
                .unwrap_or(std::cmp::Ordering::Equal),
            "memory" => b
                .current_metrics
                .memory
                .usage_percent
                .partial_cmp(&a.current_metrics.memory.usage_percent)
                .unwrap_or(std::cmp::Ordering::Equal),
            "disk" => b
                .current_metrics
                .disk
                .usage_percent
                .partial_cmp(&a.current_metrics.disk.usage_percent)
                .unwrap_or(std::cmp::Ordering::Equal),
            _ => b.performance_score.cmp(&a.performance_score), // default: score
        }
    });

    // Truncate to limit after sorting
    reports.truncate(limit);

    let comparison: Vec<_> = reports
        .into_iter()
        .map(|r| {
            (
                r.vm_name,
                r.performance_score,
                r.status.as_str().to_string(),
            )
        })
        .collect();

    println!("{}", reporter.format_comparison(comparison));

    println!();
    println!("{}", color::info(&format!("ℹ Sorted by: {}", sort_by)));
    Ok(())
}

// ========== DISK HANDLERS ==========

pub async fn handle_disk_expand(
    vm: String,
    disk: String,
    size: String,
    pvc: Option<String>,
    plan: bool,
    namespace: &str,
) -> Result<()> {
    use crate::disk::{DiskConfig, DiskExpansion, DiskInfo};

    let pvc_name = pvc.as_deref().unwrap_or(&disk);
    let config = DiskConfig::new(&disk, pvc_name).with_sizes("unknown", &size);

    let expansion = DiskExpansion::new(namespace);
    let expansion_plan = expansion.create_plan(&vm, &config)?;

    if plan {
        // Show plan without executing
        println!("{}", color::header(&format!("Disk Expansion Plan: {}", vm)));
        println!("  Disk:        {}", color::value(&disk));
        println!("  PVC:         {}", color::value(pvc_name));
        println!("  Target Size: {}", color::value(&size));
        println!();

        println!("{}", color::header("Expansion Steps:"));
        for step in &expansion_plan.steps {
            let status = if step.completed {
                color::success("✓")
            } else {
                color::muted("○")
            };
            println!(
                "  {} Step {}: {}",
                status, step.step_number, step.description
            );
            println!("     {}", color::muted(&format!("$ {}", step.command)));
        }

        println!();
        let increase_gi = DiskInfo::parse_size(&size) / (1024 * 1024 * 1024);
        println!(
            "{}",
            color::info(&format!(
                "ℹ Estimated time: {}",
                expansion.estimate_duration(increase_gi)
            ))
        );
    } else {
        // Execute expansion
        println!("{}", color::header(&format!("Expanding disk: {}", disk)));
        println!("  VM:          {}", vm);
        println!("  Target size: {}", color::value(&size));
        println!();

        println!("{}", color::info("Starting PVC resize..."));
        match expansion.resize_pvc(pvc_name, &size).await {
            Ok(_) => {
                println!("{} PVC resize initiated", color::success("✓"));
                println!();
                println!("{}", color::warning("⚠ Next steps (run inside VM):"));
                println!("  1. Rescan disk:");
                println!(
                    "     {}",
                    color::command("echo 1 | sudo tee /sys/class/block/vda/device/rescan")
                );
                println!("  2. Expand filesystem (see: vmrogue disk-script)");
            }
            Err(e) => {
                println!("{} Failed to resize PVC: {}", color::error("✗"), e);
                return Err(e);
            }
        }
    }
    Ok(())
}

pub async fn handle_disk_health(vm: String, detailed: bool, namespace: &str) -> Result<()> {
    use crate::disk::health::DiskHealthStatus;
    use crate::disk::{DiskHealthCheck, DiskInfo};
    use crate::kube::KubeClient;

    println!("{}", color::header(&format!("Disk Health: {}", vm)));
    println!();

    // Query real VM and PVC data from Kubernetes
    let disks = match KubeClient::new().await {
        Ok(client) => match client.get_vm(namespace, &vm).await {
            Ok(vm_obj) => {
                let pvc_api: kube::api::Api<k8s_openapi::api::core::v1::PersistentVolumeClaim> =
                    kube::api::Api::namespaced(client.client(), namespace);
                let mut disk_list = Vec::new();
                if let Some(volumes) = &vm_obj.spec.template.spec.volumes {
                    for vol in volumes {
                        let mut d = DiskInfo::new(&vol.name);
                        if let Some(ref pvc) = vol.persistent_volume_claim {
                            // Try to get PVC size
                            let pvcs = &pvc_api;
                            if let Ok(pvc_obj) = pvcs.get(&pvc.claim_name).await {
                                let size = pvc_obj
                                    .spec
                                    .as_ref()
                                    .and_then(|s| s.resources.as_ref())
                                    .and_then(|r| r.requests.as_ref())
                                    .and_then(|req| req.get("storage"))
                                    .map(|q| q.0.clone())
                                    .unwrap_or_else(|| "unknown".to_string());
                                d.size = size.clone();
                                // Actual disk usage not available via KubeVirt API
                                let total = DiskInfo::parse_size(&size);
                                d.used = DiskInfo::format_size(0);
                                d.available = DiskInfo::format_size(total);
                                d.usage_percent = 0.0; // Note: actual disk usage not available via KubeVirt API

                                // Get PVC status phase and actual capacity
                                if let Some(ref status) = pvc_obj.status {
                                    if let Some(ref phase) = status.phase {
                                        d.device = format!("pvc:{} ({})", pvc.claim_name, phase);
                                    } else {
                                        d.device = format!("pvc:{}", pvc.claim_name);
                                    }
                                    // Get actual capacity if different from requested
                                    if let Some(ref cap) = status.capacity {
                                        if let Some(actual) = cap.get("storage") {
                                            if actual.0 != size {
                                                d.size = format!("{} (actual: {})", size, actual.0);
                                            }
                                        }
                                    }
                                } else {
                                    d.device = format!("pvc:{}", pvc.claim_name);
                                }
                            } else {
                                d.device = format!("pvc:{}", pvc.claim_name);
                            }
                        } else if let Some(ref empty) = vol.empty_disk {
                            d.size = empty.capacity.clone();
                            let total = DiskInfo::parse_size(&empty.capacity);
                            d.used = DiskInfo::format_size(0);
                            d.available = DiskInfo::format_size(total);
                            d.usage_percent = 0.0; // Note: actual disk usage not available via KubeVirt API
                        } else if let Some(ref dv) = vol.data_volume {
                            d.device = format!("dv:{}", dv.name);
                            d.size = "unknown".to_string();
                        } else if vol.container_disk.is_some() {
                            d.device = "container-disk".to_string();
                            continue; // Skip container disks for health
                        } else if vol.cloud_init_no_cloud.is_some() {
                            continue; // Skip cloud-init volumes
                        }
                        d.mount_point = format!("/{}", vol.name);
                        d.filesystem = "ext4".to_string();
                        disk_list.push(d);
                    }
                }
                if disk_list.is_empty() {
                    println!("{}", color::muted("No persistent disks found for this VM"));
                    return Ok(());
                }
                disk_list
            }
            Err(e) => {
                return Err(anyhow!("Failed to get VM '{}': {}", vm, e));
            }
        },
        Err(e) => {
            return Err(anyhow!("Failed to connect to Kubernetes: {}", e));
        }
    };

    let checker = DiskHealthCheck::with_defaults();
    let health = checker.check_vm(&vm, disks.clone());

    let status_str = match health.overall_status {
        DiskHealthStatus::Healthy => color::success("✓ HEALTHY"),
        DiskHealthStatus::Warning => color::warning("⚠ WARNING"),
        DiskHealthStatus::Critical => color::error("✗ CRITICAL"),
        DiskHealthStatus::Full => color::error("✗ FULL"),
    };

    println!("Overall Status: {}", status_str);
    println!();

    if detailed {
        println!("{}", color::header("Disk Details:"));
        for disk in &disks {
            let status = checker.check_disk(disk);
            let status_icon = match status {
                DiskHealthStatus::Healthy => color::success("✓"),
                DiskHealthStatus::Warning => color::warning("⚠"),
                DiskHealthStatus::Critical => color::error("✗"),
                DiskHealthStatus::Full => color::error("✗"),
            };

            println!();
            println!("  {} {}", status_icon, color::value(&disk.mount_point));
            println!("    Size:       {}", disk.size);
            println!("    Used:       {} ({:.1}%)", disk.used, disk.usage_percent);
            println!("    Available:  {}", disk.available);
            println!("    Filesystem: {}", disk.filesystem);
            println!("    Device:     {}", color::muted(&disk.device));

            if status != DiskHealthStatus::Healthy {
                let recommended = checker.recommend_expansion_size(disk, 60.0);
                println!(
                    "    {}",
                    color::warning(&format!("→ Recommended size: {}", recommended))
                );
            }
        }
    }

    if !health.alerts.is_empty() {
        println!();
        println!("{}", color::header("Alerts:"));
        for alert in &health.alerts {
            println!("  {} {}", color::warning("⚠"), alert.message);
            println!("    {}", color::muted(&alert.recommendation));
        }
    }

    if health.needs_expansion {
        println!();
        println!(
            "{}",
            color::info("ℹ Use 'vmrogue disk-expand' to expand disks")
        );
    }
    Ok(())
}

pub fn handle_disk_script(
    filesystem: String,
    device: String,
    output: Option<String>,
    dry_run: bool,
) -> Result<()> {
    use crate::disk::{ExpansionScript, FilesystemType, ScriptGenerator};

    let fs_type = FilesystemType::from_string(&filesystem);
    let script_config = ExpansionScript::new(fs_type.clone(), &device)
        .with_lvm("ubuntu-vg", "ubuntu-lv")
        .with_partition(3)
        .dry_run(dry_run);

    let script = ScriptGenerator::generate(&script_config)
        .map_err(|e| anyhow::anyhow!("Invalid device path: {}", e))?;

    if let Some(output_file) = output {
        std::fs::write(&output_file, &script)?;
        println!(
            "{} Script written to: {}",
            color::success("✓"),
            color::path(&output_file)
        );
        println!();
        println!("{}", color::info("To execute:"));
        println!("  {}", color::command(&format!("chmod +x {}", output_file)));
        println!("  {}", color::command(&format!("sudo ./{}", output_file)));
    } else {
        println!("{}", script);
    }

    println!();
    println!("{}", color::header("Quick One-Liner:"));
    let oneliner = ScriptGenerator::generate_oneliner(&fs_type, &device)
        .map_err(|e| anyhow::anyhow!("Invalid device path: {}", e))?;
    println!("{}", color::command(&oneliner));
    Ok(())
}

pub async fn handle_disk_usage(
    vm: Option<String>,
    sort_by: String,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::disk::DiskInfo;
    use crate::kube::KubeClient;

    println!("{}", color::header("Disk Usage"));
    if let Some(vm_name) = &vm {
        println!("  VM: {}", color::value(vm_name));
    }
    println!();

    // Query real VM and PVC data from Kubernetes
    let client = KubeClient::new()
        .await
        .map_err(|e| anyhow!("Failed to connect to Kubernetes: {}", e))?;

    let vms = if let Some(ref vm_name) = vm {
        match client.get_vm(namespace, vm_name).await {
            Ok(v) => vec![v],
            Err(e) => return Err(anyhow!("Failed to get VM '{}': {}", vm_name, e)),
        }
    } else {
        client
            .list_vms(namespace)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list VMs: {}", e))?
    };

    let pvcs: kube::api::Api<k8s_openapi::api::core::v1::PersistentVolumeClaim> =
        kube::api::Api::namespaced(client.client(), namespace);
    let pvc_list = pvcs
        .list(&kube::api::ListParams::default())
        .await
        .map_err(|e| anyhow!("Failed to list PVCs: {}", e))?;

    let mut disks: Vec<DiskInfo> = Vec::new();

    for vm_obj in &vms {
        let vm_name = vm_obj.metadata.name.clone().unwrap_or_default();
        if let Some(volumes) = &vm_obj.spec.template.spec.volumes {
            for vol in volumes {
                if let Some(ref pvc_ref) = vol.persistent_volume_claim {
                    let mut d = DiskInfo::new(&vm_name);
                    d.mount_point = format!("/{}", vol.name);

                    // Find the PVC in our list
                    if let Some(pvc_obj) = pvc_list
                        .items
                        .iter()
                        .find(|p| p.metadata.name.as_deref() == Some(&pvc_ref.claim_name))
                    {
                        let size = pvc_obj
                            .spec
                            .as_ref()
                            .and_then(|s| s.resources.as_ref())
                            .and_then(|r| r.requests.as_ref())
                            .and_then(|req| req.get("storage"))
                            .map(|q| q.0.clone())
                            .unwrap_or_else(|| "unknown".to_string());
                        d.size = size.clone();
                        let total = DiskInfo::parse_size(&size);
                        d.used = DiskInfo::format_size(0);
                        d.available = DiskInfo::format_size(total);
                        d.usage_percent = 0.0; // Note: actual disk usage not available via KubeVirt API
                    }
                    disks.push(d);
                } else if let Some(ref empty) = vol.empty_disk {
                    let mut d = DiskInfo::new(&vm_name);
                    d.mount_point = format!("/{}", vol.name);
                    d.size = empty.capacity.clone();
                    let total = DiskInfo::parse_size(&empty.capacity);
                    d.used = DiskInfo::format_size(0);
                    d.available = DiskInfo::format_size(total);
                    d.usage_percent = 0.0; // Note: actual disk usage not available via KubeVirt API
                    disks.push(d);
                }
            }
        }
    }

    if disks.is_empty() {
        println!("{}", color::muted("No disk data found"));
        return Ok(());
    }

    // Sort disks
    match sort_by.as_str() {
        "usage" => disks.sort_by(|a, b| {
            b.usage_percent
                .partial_cmp(&a.usage_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "size" => disks.sort_by(|a, b| {
            let a_size = DiskInfo::parse_size(&a.size);
            let b_size = DiskInfo::parse_size(&b.size);
            b_size.cmp(&a_size)
        }),
        "available" => disks.sort_by(|a, b| {
            let a_avail = DiskInfo::parse_size(&a.available);
            let b_avail = DiskInfo::parse_size(&b.available);
            a_avail.cmp(&b_avail)
        }),
        _ => disks.sort_by(|a, b| a.name.cmp(&b.name)),
    }

    if output == "json" {
        let json = serde_json::to_string_pretty(&disks)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&disks)?;
        println!("{}", yaml);
    } else {
        // Table format
        println!(
            "{:<20} {:<12} {:<12} {:<12} {:<10}",
            color::label("VM"),
            color::label("SIZE"),
            color::label("USED"),
            color::label("AVAILABLE"),
            color::label("USAGE%")
        );
        println!("{}", "-".repeat(70));

        for disk in &disks {
            let usage_str = match classify_disk_usage(disk.usage_percent) {
                DiskUsageLevel::Critical => color::error(&format!("{:.1}%", disk.usage_percent)),
                DiskUsageLevel::Warning => color::warning(&format!("{:.1}%", disk.usage_percent)),
                DiskUsageLevel::Normal => format!("{:.1}%", disk.usage_percent),
            };

            println!(
                "{:<20} {:<12} {:<12} {:<12} {}",
                disk.name, disk.size, disk.used, disk.available, usage_str
            );
        }
    }

    println!();
    println!("{}", color::info(&format!("ℹ Sorted by: {}", sort_by)));
    Ok(())
}

// ========== NETWORK HANDLERS ==========

pub async fn handle_network_list(vm: String, output: String, namespace: &str) -> Result<()> {
    use crate::kube::KubeClient;
    use crate::network::{self, NetworkInterface};

    println!("{}", color::header(&format!("Network Interfaces: {}", vm)));
    println!();

    // Query real VM spec for network interfaces
    let client = KubeClient::new()
        .await
        .map_err(|e| anyhow!("Failed to connect to Kubernetes: {}", e))?;

    let vm_obj = client
        .get_vm(namespace, &vm)
        .await
        .map_err(|e| anyhow!("Failed to get VM '{}': {}", vm, e))?;

    let mut interfaces: Vec<NetworkInterface> = Vec::new();

    // Extract interfaces from VM spec
    if let Some(ref devices) = vm_obj.spec.template.spec.domain.devices {
        if let Some(ref ifaces) = devices.interfaces {
            for iface in ifaces {
                let mut net_iface = NetworkInterface::new(&iface.name);

                // Determine interface type from spec
                if iface.masquerade.is_some() {
                    net_iface.interface_type = network::InterfaceType::Masquerade;
                } else if iface.bridge.is_some() {
                    net_iface.interface_type = network::InterfaceType::Bridge;
                }

                if let Some(ref model) = iface.model {
                    net_iface.model = model.clone();
                }

                // Match with network definitions
                if let Some(ref networks) = vm_obj.spec.template.spec.networks {
                    if let Some(net) = networks.iter().find(|n| n.name == iface.name) {
                        if net.pod.is_some() {
                            net_iface.network = "pod-network".to_string();
                        } else if let Some(ref multus) = net.multus {
                            net_iface.network = multus.network_name.clone();
                            net_iface.interface_type = network::InterfaceType::Multus;
                        }
                    }
                }

                // VM is presumably running if we can query it
                let is_running = vm_obj.spec.running.unwrap_or(false);
                net_iface.state = if is_running {
                    network::InterfaceState::Up
                } else {
                    network::InterfaceState::Down
                };

                interfaces.push(net_iface);
            }
        }
    }

    if interfaces.is_empty() {
        println!(
            "{}",
            color::muted("No network interfaces found for this VM")
        );
        return Ok(());
    }

    if output == "json" {
        let json = serde_json::to_string_pretty(&interfaces)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&interfaces)?;
        println!("{}", yaml);
    } else {
        // Table format
        println!(
            "{:<12} {:<15} {:<20} {:<18} {:<12} {}",
            color::label("NAME"),
            color::label("NETWORK"),
            color::label("MAC ADDRESS"),
            color::label("IP ADDRESS"),
            color::label("TYPE"),
            color::label("STATE")
        );
        println!("{}", "-".repeat(95));

        for iface in &interfaces {
            let state_str = match iface.state {
                network::InterfaceState::Up => color::success("UP"),
                network::InterfaceState::Down => color::error("DOWN"),
                network::InterfaceState::Unknown => color::muted("UNKNOWN"),
            };

            println!(
                "{:<12} {:<15} {:<20} {:<18} {:<12} {}",
                iface.name,
                iface.network,
                iface.mac_address,
                iface.ip_address.as_deref().unwrap_or("-"),
                iface.interface_type.as_str(),
                state_str
            );
        }
    }
    Ok(())
}

pub async fn handle_network_get(
    vm: String,
    interface: String,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::kube::KubeClient;
    use crate::network::{self, NetworkInterface};

    let client = KubeClient::new()
        .await
        .map_err(|e| anyhow!("Failed to connect to Kubernetes: {}", e))?;

    let vm_obj = client
        .get_vm(namespace, &vm)
        .await
        .map_err(|e| anyhow!("Failed to get VM '{}': {}", vm, e))?;

    let mut iface = NetworkInterface::new(&interface);

    // Find the interface in VM spec
    if let Some(ref devices) = vm_obj.spec.template.spec.domain.devices {
        if let Some(ref ifaces) = devices.interfaces {
            if let Some(spec_iface) = ifaces.iter().find(|i| i.name == interface) {
                if spec_iface.masquerade.is_some() {
                    iface.interface_type = network::InterfaceType::Masquerade;
                } else if spec_iface.bridge.is_some() {
                    iface.interface_type = network::InterfaceType::Bridge;
                }
                if let Some(ref model) = spec_iface.model {
                    iface.model = model.clone();
                }
            }
        }
    }

    // Match with network definitions
    if let Some(ref networks) = vm_obj.spec.template.spec.networks {
        if let Some(net) = networks.iter().find(|n| n.name == interface) {
            if net.pod.is_some() {
                iface.network = "pod-network".to_string();
            } else if let Some(ref multus) = net.multus {
                iface.network = multus.network_name.clone();
                iface.interface_type = network::InterfaceType::Multus;
            }
        }
    }

    let is_running = vm_obj.spec.running.unwrap_or(false);
    iface.state = if is_running {
        network::InterfaceState::Up
    } else {
        network::InterfaceState::Down
    };

    if output == "json" {
        let json = serde_json::to_string_pretty(&iface)?;
        println!("{}", json);
    } else {
        let yaml = serde_yml::to_string(&iface)?;
        println!("{}", yaml);
    }
    Ok(())
}

pub async fn handle_network_bandwidth(
    vm: String,
    interface: Option<String>,
    watch: bool,
    interval: u64,
    namespace: &str,
) -> Result<()> {
    use crate::monitoring::MetricsCollector;
    use crate::network::bandwidth::BandwidthMetrics;

    let iface_name = interface.unwrap_or_else(|| "eth0".to_string());
    let collector = MetricsCollector::new(namespace);

    let mut first = true;
    loop {
        if !first {
            println!("\n{}", "═".repeat(75));
        }
        first = false;

        println!("{}", color::header(&format!("Network Bandwidth: {}", vm)));
        println!("  Interface: {}", color::value(&iface_name));
        println!();

        // Fetch real interface info from VMI
        let client = crate::kube::KubeClient::new().await.ok();
        if let Some(ref client) = client {
            if let Ok(vmi) = client.get_vmi(namespace, &vm).await {
                if let Some(status) = &vmi.status {
                    if !status.interfaces.is_empty() {
                        println!("{}", color::header("VMI Interfaces:"));
                        for iface in &status.interfaces {
                            if let Some(ref name) = iface.name {
                                println!(
                                    "  {} - IP: {}, MAC: {}",
                                    color::value(name),
                                    iface.ip_address.as_deref().unwrap_or("n/a"),
                                    iface.mac.as_deref().unwrap_or("n/a")
                                );
                            }
                        }
                        println!();
                    }
                }
            }
        }

        // Collect metrics from the VM
        let metrics = collector.collect(&vm).await?;

        println!(
            "{:<15} {:<15} {:<15} {:<12} {:<12}",
            color::label("INTERFACE"),
            color::label("RX"),
            color::label("TX"),
            color::label("RX RATE"),
            color::label("TX RATE")
        );
        println!("{}", "-".repeat(75));

        let rx_rate = format!(
            "{}/s",
            BandwidthMetrics::format_bytes(metrics.network.rx_bytes_per_sec)
        );
        let tx_rate = format!(
            "{}/s",
            BandwidthMetrics::format_bytes(metrics.network.tx_bytes_per_sec)
        );

        println!(
            "{:<15} {:<15} {:<15} {:<12} {:<12}",
            iface_name,
            format!("{} pkt/s", metrics.network.rx_packets_per_sec),
            format!("{} pkt/s", metrics.network.tx_packets_per_sec),
            rx_rate,
            tx_rate
        );

        println!();
        println!("{}", color::header("Statistics:"));
        println!("  RX Packets/s:  {}", metrics.network.rx_packets_per_sec);
        println!("  TX Packets/s:  {}", metrics.network.tx_packets_per_sec);
        println!(
            "  RX Errors:     {}",
            if metrics.network.rx_errors > 0 {
                color::warning(&metrics.network.rx_errors.to_string())
            } else {
                "0".to_string()
            }
        );
        println!(
            "  TX Errors:     {}",
            if metrics.network.tx_errors > 0 {
                color::warning(&metrics.network.tx_errors.to_string())
            } else {
                "0".to_string()
            }
        );
        println!(
            "  Bandwidth:     {:.2} MB/s",
            metrics.network.total_bandwidth_mb_per_sec()
        );

        if !watch {
            break;
        }

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval)) => {}
            _ = tokio::signal::ctrl_c() => { break; }
        }
    }
    Ok(())
}

pub fn handle_network_traffic(
    vm: String,
    interface: Option<String>,
    period: String,
    _top: usize,
    _output: String,
) -> Result<()> {
    println!(
        "{}",
        color::header(&format!("Network Traffic Analysis: {}", vm))
    );
    if let Some(iface) = &interface {
        println!("  Interface: {}", color::value(iface));
    }
    println!("  Period: {}", color::value(&period));
    println!();

    // Real traffic flow analysis requires Prometheus or guest agent integration.
    // No live network metrics are available without a monitoring backend.
    println!(
        "{}",
        color::info("Network metrics require Prometheus or guest agent integration")
    );
    println!("  Configure with: vmrogue config set monitoring.prometheus-url <url>");
    println!();
    println!("  {}", color::muted("No network traffic data available"));
    Ok(())
}

pub async fn handle_network_policies(
    all_namespaces: bool,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::network::policies::{NetworkPolicy, VMSelector};

    println!("{}", color::header("Network Policies"));
    if all_namespaces {
        println!("  Namespace: {}", color::value("All"));
    }
    println!();

    // Query real K8s NetworkPolicy resources
    let client = crate::kube::get_client().await?;

    let k8s_policies_api: kube::api::Api<k8s_openapi::api::networking::v1::NetworkPolicy> =
        if all_namespaces {
            kube::api::Api::all(client)
        } else {
            kube::api::Api::namespaced(client, namespace)
        };

    let k8s_policies = k8s_policies_api
        .list(&kube::api::ListParams::default())
        .await
        .map_err(|e| anyhow!("Failed to list network policies: {}", e))?;

    // Convert K8s NetworkPolicy to our NetworkPolicy type
    let policies: Vec<NetworkPolicy> = k8s_policies
        .items
        .iter()
        .map(|p| {
            let name = p.metadata.name.clone().unwrap_or_default();
            let mut selector = VMSelector::default();

            // Extract pod selector labels (which select VMs in KubeVirt)
            if let Some(ref spec) = p.spec {
                if let Some(ref labels) = spec.pod_selector.match_labels {
                    for (k, v) in labels {
                        selector = selector.with_label(k.clone(), v.clone());
                    }
                }
            }

            let mut policy = NetworkPolicy::new(name).with_vm_selector(selector);

            // Count ingress/egress rules
            if let Some(ref spec) = p.spec {
                if let Some(ref ingress) = spec.ingress {
                    policy
                        .ingress_rules
                        .extend(ingress.iter().map(|_| Default::default()));
                }
                if let Some(ref egress) = spec.egress {
                    policy
                        .egress_rules
                        .extend(egress.iter().map(|_| Default::default()));
                }
            }

            policy
        })
        .collect();

    if policies.is_empty() {
        println!("{}", color::muted("No network policies found"));
        return Ok(());
    }

    if output == "json" {
        let json = serde_json::to_string_pretty(&policies)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&policies)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<25} {:<12} {:<12} {}",
            color::label("NAME"),
            color::label("INGRESS"),
            color::label("EGRESS"),
            color::label("SELECTOR")
        );
        println!("{}", "-".repeat(70));

        for policy in &policies {
            let selector_str = policy
                .vm_selector
                .labels
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(",");

            println!(
                "{:<25} {:<12} {:<12} {}",
                policy.name,
                policy.ingress_rules.len(),
                policy.egress_rules.len(),
                selector_str
            );
        }
    }
    Ok(())
}

pub fn handle_network_policy(name: String, output: String) -> Result<()> {
    use crate::network::policies::{NetworkPolicy, VMSelector};

    let policy_name = name;
    let selector = VMSelector::default().with_label("app".to_string(), "web".to_string());
    let policy = NetworkPolicy::new(policy_name.clone()).with_vm_selector(selector);

    println!("Note: Showing simulated data for policy '{}'", policy_name);
    println!();

    if output == "json" {
        let json = serde_json::to_string_pretty(&policy)?;
        println!("{}", json);
    } else {
        let yaml = serde_yml::to_string(&policy)?;
        println!("{}", yaml);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== classify_disk_usage tests =====

    #[test]
    fn test_classify_disk_usage_normal_zero() {
        assert_eq!(classify_disk_usage(0.0), DiskUsageLevel::Normal);
    }

    #[test]
    fn test_classify_disk_usage_normal_low() {
        assert_eq!(classify_disk_usage(50.0), DiskUsageLevel::Normal);
    }

    #[test]
    fn test_classify_disk_usage_normal_just_below_warning() {
        assert_eq!(classify_disk_usage(74.9), DiskUsageLevel::Normal);
    }

    #[test]
    fn test_classify_disk_usage_warning_boundary() {
        assert_eq!(classify_disk_usage(75.0), DiskUsageLevel::Warning);
    }

    #[test]
    fn test_classify_disk_usage_warning_mid() {
        assert_eq!(classify_disk_usage(85.0), DiskUsageLevel::Warning);
    }

    #[test]
    fn test_classify_disk_usage_warning_just_below_critical() {
        assert_eq!(classify_disk_usage(89.9), DiskUsageLevel::Warning);
    }

    #[test]
    fn test_classify_disk_usage_critical_boundary() {
        assert_eq!(classify_disk_usage(90.0), DiskUsageLevel::Critical);
    }

    #[test]
    fn test_classify_disk_usage_critical_high() {
        assert_eq!(classify_disk_usage(99.5), DiskUsageLevel::Critical);
    }

    #[test]
    fn test_classify_disk_usage_critical_100() {
        assert_eq!(classify_disk_usage(100.0), DiskUsageLevel::Critical);
    }

    // ===== derive_vm_name_from_snapshot tests =====

    #[test]
    fn test_derive_vm_name_standard() {
        // Strips from the last "-snapshot" onward, leaving just the VM name
        assert_eq!(
            derive_vm_name_from_snapshot("myvm-snapshot-20240101"),
            "myvm"
        );
    }

    #[test]
    fn test_derive_vm_name_simple() {
        assert_eq!(derive_vm_name_from_snapshot("web-snapshot"), "web");
    }

    #[test]
    fn test_derive_vm_name_no_snapshot_suffix() {
        assert_eq!(
            derive_vm_name_from_snapshot("myvm-backup-123"),
            "myvm-backup-123"
        );
    }

    #[test]
    fn test_derive_vm_name_multiple_snapshot() {
        // Strips from the last "-snapshot" onward
        assert_eq!(
            derive_vm_name_from_snapshot("snapshot-vm-snapshot"),
            "snapshot-vm"
        );
    }

    #[test]
    fn test_derive_vm_name_empty() {
        assert_eq!(derive_vm_name_from_snapshot(""), "");
    }

    // ===== default_restore_target_name tests =====

    #[test]
    fn test_default_restore_target_name_standard() {
        assert_eq!(
            default_restore_target_name("myvm-snap-001"),
            "myvm-snap-001-restored"
        );
    }

    #[test]
    fn test_default_restore_target_name_simple() {
        assert_eq!(default_restore_target_name("backup"), "backup-restored");
    }

    #[test]
    fn test_default_restore_target_name_empty() {
        assert_eq!(default_restore_target_name(""), "-restored");
    }

    #[test]
    fn test_default_restore_target_name_with_hyphens() {
        assert_eq!(default_restore_target_name("a-b-c"), "a-b-c-restored");
    }

    #[test]
    fn test_default_restore_target_name_already_restored() {
        assert_eq!(
            default_restore_target_name("snap-restored"),
            "snap-restored-restored"
        );
    }
}
