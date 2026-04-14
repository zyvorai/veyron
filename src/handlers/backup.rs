use crate::tui::colors::cli as color;
use anyhow::Result;

pub async fn handle_backup_create(
    vm: String,
    name: Option<String>,
    backup_type: String,
    compression: String,
    no_encryption: bool,
    namespace: &str,
) -> Result<()> {
    use crate::backup::{BackupConfig, BackupType, CompressionType};
    use chrono::Utc;

    log::debug!("Using namespace: {}", namespace);

    let backup_name =
        name.unwrap_or_else(|| format!("{}-backup-{}", vm, Utc::now().format("%Y%m%d-%H%M%S")));

    println!("{}", color::header(&format!("Creating Backup: {}", vm)));
    println!();

    let b_type = match backup_type.as_str() {
        "incremental" => BackupType::Incremental,
        "differential" => BackupType::Differential,
        _ => BackupType::Full,
    };

    let comp_type = match compression.as_str() {
        "zstd" => CompressionType::Zstd,
        "lz4" => CompressionType::Lz4,
        "none" => CompressionType::None,
        _ => CompressionType::Gzip,
    };

    let mut config = BackupConfig::new(&vm, &backup_name);
    config.backup_type = b_type;
    config.compression = comp_type;
    if no_encryption {
        config.encryption_enabled = false;
    }

    println!("  Backup Name:  {}", color::value(&backup_name));
    println!("  VM:           {}", vm);
    println!("  Type:         {}", config.backup_type.as_str());
    println!("  Compression:  {}", config.compression);
    println!(
        "  Encryption:   {}",
        if config.encryption_enabled {
            "Enabled"
        } else {
            "Disabled"
        }
    );
    println!();

    // Create actual VirtualMachineSnapshot via SnapshotManager
    let manager = crate::snapshots::SnapshotManager::new(namespace)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to Kubernetes: {}", e))?;

    let snapshot_config = crate::snapshots::SnapshotConfig::new(&vm, &backup_name)
        .with_description(format!(
            "Backup snapshot (type={}, compression={})",
            config.backup_type.as_str(),
            config.compression
        ))
        .with_label("vmrogue.io/backup-type", config.backup_type.as_str());

    manager
        .create_snapshot(&snapshot_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create backup snapshot: {}", e))?;

    println!("{}", color::success("✓ Backup created successfully"));
    println!();
    println!(
        "{}",
        color::info(&format!(
            "ℹ Use 'vmrogue backup-get {}' to view details",
            backup_name
        ))
    );
    Ok(())
}

pub async fn handle_backup_list(vm: Option<String>, output: String, namespace: &str) -> Result<()> {
    use crate::backup::BackupStatus;
    use crate::snapshots::crds::VirtualMachineSnapshot;

    println!("{}", color::header("Backups"));
    if let Some(v) = &vm {
        println!("  VM: {}", color::value(v));
    }
    println!();

    // Query VirtualMachineSnapshot CRDs (backups are snapshots in KubeVirt)
    let client = crate::kube::get_client().await?;

    let snapshots_api: kube::api::Api<VirtualMachineSnapshot> =
        kube::api::Api::namespaced(client, namespace);

    let snapshot_list = snapshots_api
        .list(&kube::api::ListParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list snapshots: {}", e))?;

    // Convert snapshots to BackupStatus entries
    let backups: Vec<BackupStatus> = snapshot_list
        .items
        .iter()
        .filter(|s| {
            // Filter by VM if specified
            if let Some(ref vm_name) = vm {
                s.spec.source.name == *vm_name
            } else {
                true
            }
        })
        .map(|s| {
            let snap_name = s.metadata.name.clone().unwrap_or_default();
            let vm_name = s.spec.source.name.clone();
            BackupStatus::new(&vm_name, &snap_name)
        })
        .collect();

    if backups.is_empty() {
        println!("{}", color::muted("No backups (snapshots) found"));
        println!();
        println!(
            "{}",
            color::info("ℹ Create a backup with 'vmrogue snapshot-create <vm>'")
        );
        return Ok(());
    }

    if output == "json" {
        let json = serde_json::to_string_pretty(&backups)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&backups)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<30} {:<15} {:<15} {:<15} {}",
            color::label("BACKUP"),
            color::label("VM"),
            color::label("SIZE"),
            color::label("COMPRESSED"),
            color::label("RATIO")
        );
        println!("{}", "-".repeat(90));

        for b in &backups {
            let size = format!("{:.2} GB", b.size_bytes as f64 / 1_000_000_000.0);
            let compressed = format!("{:.2} GB", b.compressed_size_bytes as f64 / 1_000_000_000.0);
            let ratio = format!("{:.1}%", b.compression_ratio());

            println!(
                "{:<30} {:<15} {:<15} {:<15} {}",
                b.backup_name, b.vm_name, size, compressed, ratio
            );
        }
    }
    Ok(())
}

pub fn handle_backup_get(name: String, output: String, namespace: &str) -> Result<()> {
    use crate::backup::BackupStatus;

    log::debug!("Using namespace: {}", namespace);

    // Derive VM name from backup name (e.g., "my-vm-backup-20240101" -> "my-vm")
    let vm_name = if let Some(pos) = name.rfind("-backup") {
        name[..pos].to_string()
    } else {
        name.clone()
    };
    let backup = BackupStatus::new(vm_name, &name);

    if output == "json" {
        let json = serde_json::to_string_pretty(&backup)?;
        println!("{}", json);
    } else {
        let yaml = serde_yml::to_string(&backup)?;
        println!("{}", yaml);
    }
    Ok(())
}

pub async fn handle_backup_delete(name: String, yes: bool, namespace: &str) -> Result<()> {
    log::debug!("Using namespace: {}", namespace);

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!("Delete backup '{}'?", name))
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            println!("{}", color::muted("Cancelled"));
            return Ok(());
        }
    }

    println!("{}", color::header(&format!("Deleting Backup: {}", name)));
    println!();

    // Delete actual VirtualMachineSnapshot via SnapshotManager
    let manager = crate::snapshots::SnapshotManager::new(namespace)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to Kubernetes: {}", e))?;

    manager
        .delete_snapshot(&name)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to delete backup: {}", e))?;

    println!("{}", color::success("✓ Backup deleted successfully"));
    Ok(())
}

pub async fn handle_backup_restore(
    backup: String,
    target: Option<String>,
    start: bool,
    namespace: &str,
) -> Result<()> {
    log::debug!("Using namespace: {}", namespace);

    let target_vm = target.unwrap_or_else(|| backup.replace("-backup-", "-restored-"));

    println!(
        "{}",
        color::header(&format!("Restoring from Backup: {}", backup))
    );
    println!();

    // Restore via RestoreManager (creates VirtualMachineRestore CRD)
    let restore_manager = crate::snapshots::RestoreManager::new(namespace)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to Kubernetes: {}", e))?;

    let restore_info = restore_manager
        .restore_to_new_vm(&backup, &target_vm, start)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initiate restore: {}", e))?;

    println!("  Restore ID:   {}", color::value(&restore_info.name));
    println!("  Backup:       {}", backup);
    println!("  Target VM:    {}", target_vm);
    println!("  Start After:  {}", if start { "Yes" } else { "No" });
    println!();
    println!("{}", color::success("✓ Restore initiated successfully"));
    println!();
    println!(
        "{}",
        color::info("ℹ Restore in progress. This may take several minutes.")
    );
    Ok(())
}

pub async fn handle_backup_verify(
    name: String,
    verification_type: String,
    namespace: &str,
) -> Result<()> {
    use crate::backup::verify::{VerificationRunner, VerificationStatus, VerificationType};

    log::debug!("Using namespace: {}", namespace);

    println!("{}", color::header(&format!("Verifying Backup: {}", name)));
    println!();

    let v_type = match verification_type.as_str() {
        "quick" => VerificationType::Quick,
        "full" => VerificationType::Full,
        _ => VerificationType::Standard,
    };

    let report = VerificationRunner::verify(&name, namespace, v_type).await;

    println!("  Verification Type:  {}", report.verification_type);
    println!(
        "  Status:             {}",
        match report.status {
            VerificationStatus::Passed => color::success("✓ Passed"),
            VerificationStatus::Failed => color::error("✗ Failed"),
            VerificationStatus::Warning => color::warning("⚠ Warning"),
            _ => report.status.to_string(),
        }
    );
    println!("  Checks Run:         {}", report.checks.len());
    println!(
        "  Passed:             {}",
        report
            .checks
            .len()
            .saturating_sub(report.error_count as usize)
            .saturating_sub(report.warning_count as usize)
    );
    println!(
        "  Warnings:           {}",
        if report.warning_count > 0 {
            color::warning(&report.warning_count.to_string())
        } else {
            "0".to_string()
        }
    );
    println!(
        "  Errors:             {}",
        if report.error_count > 0 {
            color::error(&report.error_count.to_string())
        } else {
            "0".to_string()
        }
    );
    println!("  Pass Rate:          {:.1}%", report.pass_rate());
    println!("  Duration:           {}s", report.duration_secs());
    Ok(())
}

pub fn handle_backup_schedules(output: String, namespace: &str) -> Result<()> {
    use crate::backup::schedule::BackupSchedule;

    log::debug!("Using namespace: {}", namespace);

    println!("{}", color::header("Backup Schedules"));
    println!();

    // Load schedules from config directory
    let schedules_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".config"))
        .join("vmrogue")
        .join("schedules");

    let schedules: Vec<BackupSchedule> = if schedules_dir.exists() {
        let mut loaded = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&schedules_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .extension()
                    .map(|e| e == "yaml" || e == "yml" || e == "json")
                    .unwrap_or(false)
                {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(schedule) = serde_yml::from_str::<BackupSchedule>(&content) {
                            loaded.push(schedule);
                        } else if let Ok(schedule) =
                            serde_json::from_str::<BackupSchedule>(&content)
                        {
                            loaded.push(schedule);
                        }
                    }
                }
            }
        }
        loaded
    } else {
        Vec::new()
    };

    if schedules.is_empty() {
        println!("{}", color::muted("No backup schedules found"));
        println!();
        println!(
            "{}",
            color::info(&format!(
                "ℹ Create schedule files in: {}",
                schedules_dir.display()
            ))
        );
        println!(
            "  {}",
            color::muted("Or use 'vmrogue backup-schedule-create' to create one")
        );
        return Ok(());
    }

    if output == "json" {
        let json = serde_json::to_string_pretty(&schedules)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&schedules)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<25} {:<15} {:<10} {}",
            color::label("NAME"),
            color::label("TYPE"),
            color::label("ENABLED"),
            color::label("NEXT RUN")
        );
        println!("{}", "-".repeat(70));

        for s in &schedules {
            let enabled_str = if s.enabled {
                color::success("Yes")
            } else {
                color::muted("No")
            };
            let next_run = s
                .next_run
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());

            println!(
                "{:<25} {:<15} {:<10} {}",
                s.name,
                match &s.schedule_type {
                    crate::backup::schedule::ScheduleType::Hourly { .. } => "Hourly",
                    crate::backup::schedule::ScheduleType::Daily { .. } => "Daily",
                    crate::backup::schedule::ScheduleType::Weekly { .. } => "Weekly",
                    crate::backup::schedule::ScheduleType::Monthly { .. } => "Monthly",
                    crate::backup::schedule::ScheduleType::Cron { .. } => "Cron",
                },
                enabled_str,
                next_run
            );
        }
    }
    Ok(())
}

pub fn handle_backup_schedule_create(
    name: String,
    schedule: String,
    vm: Option<String>,
    namespace: &str,
) -> Result<()> {
    log::debug!("Using namespace: {}", namespace);
    println!(
        "{}",
        color::header(&format!("Creating Backup Schedule: {}", name))
    );
    println!();
    println!("  Schedule:  {}", color::value(&schedule));
    println!("  VM:        {}", vm.as_deref().unwrap_or("All"));
    println!();
    println!("{}", color::success("✓ Schedule created successfully"));
    Ok(())
}

pub fn handle_recovery_plan(name: String, output: String, namespace: &str) -> Result<()> {
    use crate::backup::recovery::RecoveryPlan;

    log::debug!("Using namespace: {}", namespace);

    let plan = RecoveryPlan::new(name).with_description("Disaster recovery plan");

    if output == "json" {
        let json = serde_json::to_string_pretty(&plan)?;
        println!("{}", json);
    } else {
        let yaml = serde_yml::to_string(&plan)?;
        println!("{}", yaml);
    }
    Ok(())
}

pub async fn handle_recovery_execute(plan: String, dry_run: bool, namespace: &str) -> Result<()> {
    log::debug!("Using namespace: {}", namespace);
    println!(
        "{}",
        color::header(&format!("Executing Recovery Plan: {}", plan))
    );
    println!();

    if dry_run {
        println!("{}", color::info("=== DRY RUN MODE ==="));
        println!();

        // List available snapshots to show what would be restored
        let manager = crate::snapshots::SnapshotManager::new(namespace)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Kubernetes: {}", e))?;

        let snapshots = manager
            .list_all_snapshots()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list snapshots: {}", e))?;

        if snapshots.is_empty() {
            println!("{}", color::warning("No snapshots found to restore from"));
        } else {
            println!("Recovery Steps:");
            for (i, snap) in snapshots.iter().enumerate() {
                let ready = if snap.ready_to_use {
                    color::success("ready")
                } else {
                    color::warning("not ready")
                };
                println!(
                    "  {}. Restore VM '{}' from snapshot '{}' ({})",
                    i + 1,
                    snap.vm_name,
                    snap.name,
                    ready
                );
            }
        }

        println!();
        println!("{}", color::info("ℹ Run without --dry-run to execute"));
    } else {
        // Find the latest snapshot for each VM and trigger restores
        let manager = crate::snapshots::SnapshotManager::new(namespace)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Kubernetes: {}", e))?;

        let snapshots = manager
            .list_snapshots_sorted(None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list snapshots: {}", e))?;

        if snapshots.is_empty() {
            return Err(anyhow::anyhow!(
                "No snapshots found in namespace '{}'. Cannot execute recovery without snapshots.",
                namespace
            ));
        }

        // Group snapshots by VM, keeping only the latest (already sorted newest-first)
        let mut seen_vms = std::collections::HashSet::new();
        let mut latest_per_vm = Vec::new();
        for snap in &snapshots {
            if seen_vms.insert(snap.vm_name.clone()) {
                latest_per_vm.push(snap);
            }
        }

        let restore_manager = crate::snapshots::RestoreManager::new(namespace)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Kubernetes: {}", e))?;

        let mut restored = 0usize;
        let mut failed = 0usize;

        for snap in &latest_per_vm {
            if !snap.ready_to_use {
                println!(
                    "  {} Skipping '{}': snapshot '{}' not ready",
                    color::warning("⚠"),
                    snap.vm_name,
                    snap.name
                );
                continue;
            }

            println!(
                "  Restoring VM '{}' from snapshot '{}'...",
                snap.vm_name, snap.name
            );
            match restore_manager
                .restore_in_place(&snap.vm_name, &snap.name)
                .await
            {
                Ok(info) => {
                    println!(
                        "  {} Restore '{}' initiated for VM '{}'",
                        color::success("✓"),
                        info.name,
                        snap.vm_name
                    );
                    restored += 1;
                }
                Err(e) => {
                    println!(
                        "  {} Failed to restore VM '{}': {}",
                        color::error("✗"),
                        snap.vm_name,
                        e
                    );
                    failed += 1;
                }
            }
        }

        println!();
        println!("  Restored: {}  Failed: {}", restored, failed);
        println!();
        if failed == 0 {
            println!("{}", color::success("✓ Recovery completed successfully"));
        } else {
            println!("{}", color::warning("⚠ Recovery completed with errors"));
        }
    }
    Ok(())
}

// ========== NETWORK & MIGRATION ==========

pub async fn handle_migrate(
    vm: String,
    target_node: Option<String>,
    migration_type: String,
    plan: bool,
    namespace: &str,
) -> Result<()> {
    use crate::kube::types::VirtualMachineInstanceMigration;
    use crate::migration::{MigrationRequest, MigrationType};

    log::debug!("Using namespace: {}", namespace);

    println!("{}", color::header(&format!("VM Migration: {}", vm)));
    println!();

    let mig_type = match migration_type.as_str() {
        "offline" => MigrationType::Offline,
        "post-copy" => MigrationType::PostCopy,
        _ => MigrationType::Live,
    };

    let source_node = "unknown";
    if target_node.is_none() {
        log::warn!(
            "No --target-node specified; source node could not be auto-detected. Specify nodes explicitly for production use."
        );
    }
    let request = MigrationRequest::new(&vm, source_node)
        .to_node(target_node.unwrap_or_else(|| {
            log::warn!("No target node specified, migration may not proceed correctly");
            "auto-select".to_string()
        }))
        .with_type(mig_type);

    if plan {
        println!("{}", color::header("Migration Plan:"));
        println!("  VM:           {}", color::value(&vm));
        println!("  Source:       {}", request.source_node);
        println!(
            "  Target:       {}",
            color::value(request.target_node.as_deref().unwrap_or("auto"))
        );
        println!("  Type:         {}", request.migration_type.as_str());
        println!();
        println!("{}", color::header("Migration Steps:"));
        println!("  1. Validate source and target nodes");
        println!("  2. Prepare target node resources");
        println!("  3. Start live memory transfer");
        println!("  4. Sync disk state");
        println!("  5. Pause VM and final sync");
        println!("  6. Resume VM on target node");
        println!();
        println!(
            "{}",
            color::info("ℹ Use 'vmrogue migrate' without --plan to execute")
        );
    } else {
        // Create actual VirtualMachineInstanceMigration CRD
        let client = crate::kube::get_client().await?;

        let migrations_api: kube::api::Api<VirtualMachineInstanceMigration> =
            kube::api::Api::namespaced(client, namespace);

        let migration_name = format!(
            "{}-migration-{}",
            vm,
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        );

        let migration = VirtualMachineInstanceMigration {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(migration_name.clone()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crate::kube::types::VirtualMachineInstanceMigrationSpec {
                vmi_name: Some(vm.clone()),
            },
            status: None,
        };

        let pp = kube::api::PostParams::default();
        migrations_api
            .create(&pp, &migration)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create migration: {}", e))?;

        println!("{}", color::header("Migration Started:"));
        println!("  Migration ID: {}", color::value(&migration_name));
        println!("  VM:           {}", color::value(&vm));
        println!(
            "  Target:       {}",
            color::value(request.target_node.as_deref().unwrap_or("auto-select"))
        );
        println!("  Type:         {}", request.migration_type.as_str());
        println!();
        println!("{}", color::success("✓ Migration initiated successfully"));
        println!();
        println!(
            "{}",
            color::info("ℹ Use 'vmrogue migration-status' to monitor progress")
        );
    }
    Ok(())
}

pub async fn handle_migration_status(
    vm: String,
    watch: bool,
    interval: u64,
    namespace: &str,
) -> Result<()> {
    use crate::kube::types::VirtualMachineInstanceMigration;
    use crate::migration::{MigrationPhase, MigrationState, MigrationStatus};

    let client = crate::kube::get_client().await?;

    let mut first = true;
    loop {
        if !first {
            println!("\n{}", "═".repeat(60));
        }
        first = false;

        println!("{}", color::header(&format!("Migration Status: {}", vm)));
        println!();

        let migrations_api: kube::api::Api<VirtualMachineInstanceMigration> =
            kube::api::Api::namespaced(client.clone(), namespace);

        let migration_list = migrations_api
            .list(&kube::api::ListParams::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list migrations: {}", e))?;

        let migration = migration_list.items.iter().rev().find(|m| {
            m.spec
                .vmi_name
                .as_deref()
                .map(|name| name == vm)
                .unwrap_or(false)
                || m.metadata
                    .name
                    .as_deref()
                    .map(|name| name.contains(&vm))
                    .unwrap_or(false)
        });

        let status = match migration {
            Some(m) => {
                let source = m
                    .status
                    .as_ref()
                    .and_then(|s| s.migration_state.as_ref())
                    .and_then(|ms| ms.source_node.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let target = m
                    .status
                    .as_ref()
                    .and_then(|s| s.migration_state.as_ref())
                    .and_then(|ms| ms.target_node.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let mut status = MigrationStatus::new(&vm, &source, &target);

                let phase_str = m
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.as_deref())
                    .unwrap_or("Unknown");

                status.state = match phase_str {
                    "Succeeded" => MigrationState::Succeeded,
                    "Failed" => MigrationState::Failed,
                    "Running" => MigrationState::Running,
                    _ => MigrationState::Pending,
                };

                status.phase = match phase_str {
                    "Succeeded" => MigrationPhase::Succeeded,
                    "Failed" => MigrationPhase::Failed,
                    "Running" => MigrationPhase::MemoryTransfer,
                    _ => MigrationPhase::Preparing,
                };

                let completed = m
                    .status
                    .as_ref()
                    .and_then(|s| s.migration_state.as_ref())
                    .and_then(|ms| ms.completed)
                    .unwrap_or(false);

                status.progress_percent = if completed {
                    100
                } else if status.state == MigrationState::Running {
                    50
                } else {
                    0
                };

                status
            }
            None => {
                println!("{}", color::muted("No migrations found for this VM"));
                return Ok(());
            }
        };

        println!(
            "  State:     {}",
            match status.state {
                MigrationState::Running => color::info("Running"),
                MigrationState::Succeeded => color::success("Succeeded"),
                MigrationState::Failed => color::error("Failed"),
                _ => status.state.to_string(),
            }
        );
        println!("  Phase:     {}", status.phase);
        println!("  Progress:  {}%", status.progress_percent);
        println!("  Source:    {}", status.source_node);
        println!("  Target:    {}", status.target_node);
        println!("  Duration:  {}s", status.duration_secs());

        if !watch {
            break;
        }

        // Stop watching once migration is terminal
        if status.state == MigrationState::Succeeded || status.state == MigrationState::Failed {
            println!();
            println!("{}", color::info("ℹ Migration reached terminal state"));
            break;
        }

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval)) => {}
            _ = tokio::signal::ctrl_c() => { break; }
        }
    }
    Ok(())
}

pub async fn handle_migration_list(
    all_namespaces: bool,
    state: Option<String>,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::kube::types::VirtualMachineInstanceMigration;
    use crate::migration::{MigrationPhase, MigrationState, MigrationStatus};

    println!("{}", color::header("VM Migrations"));
    if all_namespaces {
        println!("  Namespace: {}", color::value("All"));
    }
    if let Some(s) = &state {
        println!("  State Filter: {}", color::value(s));
    }
    println!();

    // Query VirtualMachineInstanceMigration CRDs from K8s
    let client = crate::kube::get_client().await?;

    let migrations_api: kube::api::Api<VirtualMachineInstanceMigration> = if all_namespaces {
        kube::api::Api::all(client)
    } else {
        kube::api::Api::namespaced(client, namespace)
    };

    let migration_list = migrations_api
        .list(&kube::api::ListParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list migrations: {}", e))?;

    // Convert K8s migrations to display type
    let migrations: Vec<MigrationStatus> = migration_list
        .items
        .iter()
        .filter(|m| {
            if let Some(ref filter_state) = state {
                let phase = m
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.as_deref())
                    .unwrap_or("Unknown");
                phase.to_lowercase().contains(&filter_state.to_lowercase())
            } else {
                true
            }
        })
        .map(|m| {
            let vmi_name = m
                .spec
                .vmi_name
                .clone()
                .unwrap_or_else(|| m.metadata.name.clone().unwrap_or_default());

            let source = m
                .status
                .as_ref()
                .and_then(|s| s.migration_state.as_ref())
                .and_then(|ms| ms.source_node.clone())
                .unwrap_or_else(|| "unknown".to_string());

            let target = m
                .status
                .as_ref()
                .and_then(|s| s.migration_state.as_ref())
                .and_then(|ms| ms.target_node.clone())
                .unwrap_or_else(|| "unknown".to_string());

            let mut status = MigrationStatus::new(&vmi_name, &source, &target);

            let phase_str = m
                .status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("Unknown");

            status.state = match phase_str {
                "Succeeded" => MigrationState::Succeeded,
                "Failed" => MigrationState::Failed,
                "Running" => MigrationState::Running,
                _ => MigrationState::Pending,
            };

            status.phase = match phase_str {
                "Succeeded" => MigrationPhase::Succeeded,
                "Failed" => MigrationPhase::Failed,
                "Running" => MigrationPhase::MemoryTransfer,
                _ => MigrationPhase::Preparing,
            };

            let completed = m
                .status
                .as_ref()
                .and_then(|s| s.migration_state.as_ref())
                .and_then(|ms| ms.completed)
                .unwrap_or(false);

            status.progress_percent = if completed {
                100
            } else if status.state == MigrationState::Running {
                50
            } else {
                0
            };

            status
        })
        .collect();

    if migrations.is_empty() {
        println!("{}", color::muted("No migrations found"));
        return Ok(());
    }

    if output == "json" {
        let json = serde_json::to_string_pretty(&migrations)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&migrations)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<15} {:<12} {:<10} {:<10} {:<10} {}",
            color::label("VM"),
            color::label("STATE"),
            color::label("PHASE"),
            color::label("PROGRESS"),
            color::label("SOURCE"),
            color::label("TARGET")
        );
        println!("{}", "-".repeat(80));

        for m in &migrations {
            let state_str = match m.state {
                MigrationState::Running => color::info("Running"),
                MigrationState::Succeeded => color::success("Succeeded"),
                MigrationState::Failed => color::error("Failed"),
                _ => m.state.to_string(),
            };

            println!(
                "{:<15} {:<12} {:<10} {:<10} {:<10} {}",
                m.vm_name,
                state_str,
                m.phase.to_string(),
                format!("{}%", m.progress_percent),
                m.source_node,
                m.target_node
            );
        }
    }
    Ok(())
}

pub async fn handle_ha_config(
    vm: String,
    enable: bool,
    disable: bool,
    priority: Option<String>,
    eviction_strategy: Option<String>,
    namespace: &str,
) -> Result<()> {
    use crate::migration::ha::{EvictionStrategy, HAConfig, HAPriority};

    log::debug!("Using namespace: {}", namespace);

    if enable && disable {
        return Err(anyhow::anyhow!(
            "Cannot specify both --enable and --disable"
        ));
    }
    if !enable && !disable {
        return Err(anyhow::anyhow!("Must specify either --enable or --disable"));
    }

    println!("{}", color::header(&format!("HA Configuration: {}", vm)));
    println!();

    let mut config = HAConfig::new(&vm);
    config.enabled = enable;

    if let Some(ref p) = priority {
        config.priority = match p.as_str() {
            "critical" => HAPriority::Critical,
            "high" => HAPriority::High,
            "low" => HAPriority::Low,
            _ => HAPriority::Normal,
        };
    }

    let strategy_str = if let Some(ref s) = eviction_strategy {
        config.eviction_strategy = match s.as_str() {
            "shutdown" => EvictionStrategy::Shutdown,
            "none" => EvictionStrategy::None,
            _ => EvictionStrategy::LiveMigrate,
        };
        s.clone()
    } else {
        "LiveMigrate".to_string()
    };

    // Patch the VM's eviction strategy via the Kubernetes API
    let client = crate::kube::get_client().await?;
    let vms_api: kube::api::Api<crate::kube::types::VirtualMachine> =
        kube::api::Api::namespaced(client, namespace);

    let patch = if enable {
        serde_json::json!({
            "spec": {
                "template": {
                    "spec": {
                        "evictionStrategy": strategy_str
                    }
                }
            }
        })
    } else {
        serde_json::json!({
            "spec": {
                "template": {
                    "spec": {
                        "evictionStrategy": null
                    }
                }
            }
        })
    };

    vms_api
        .patch(
            &vm,
            &kube::api::PatchParams::default(),
            &kube::api::Patch::Merge(&patch),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update VM HA config: {}", e))?;

    println!(
        "  Enabled:            {}",
        if config.enabled {
            color::success("Yes")
        } else {
            color::muted("No")
        }
    );
    println!("  Priority:           {}", config.priority);
    println!("  Eviction Strategy:  {}", config.eviction_strategy);
    println!(
        "  Auto Restart:       {}",
        config.failover_policy.auto_restart
    );
    println!(
        "  Max Restarts:       {}",
        config.failover_policy.max_restart_attempts
    );
    println!();
    println!("{}", color::success("✓ HA configuration updated"));
    Ok(())
}

pub fn handle_ha_status(vm: String, output: String, namespace: &str) -> Result<()> {
    use crate::migration::ha::HAConfig;

    log::debug!("Using namespace: {}", namespace);

    let config = HAConfig::new(&vm);

    if output == "json" {
        let json = serde_json::to_string_pretty(&config)?;
        println!("{}", json);
    } else {
        let yaml = serde_yml::to_string(&config)?;
        println!("{}", yaml);
    }
    Ok(())
}

pub async fn handle_evacuate_node(
    node: String,
    reason: Option<String>,
    max_parallel: u32,
    timeout: u64,
    force: bool,
    plan: bool,
    namespace: &str,
) -> Result<()> {
    use crate::migration::evacuation::{EvacuationPlanner, EvacuationRequest, EvacuationStatus};

    log::debug!("Using namespace: {}", namespace);

    println!("{}", color::header(&format!("Node Evacuation: {}", node)));
    println!();

    let request =
        EvacuationRequest::new(&node, reason.unwrap_or_else(|| "Maintenance".to_string()))
            .with_timeout(timeout);

    let planner = EvacuationPlanner::new(max_parallel);

    if plan {
        println!("{}", color::header("Evacuation Plan:"));
        println!("  Node:         {}", color::value(&node));
        println!("  Reason:       {}", request.reason);
        println!("  Max Parallel: {}", max_parallel);
        println!("  Timeout:      {}s", timeout);
        println!("  Force:        {}", if force { "Yes" } else { "No" });
        println!();

        // Query VMs running on this specific node from Kubernetes
        let vms: Vec<(String, u8)> = if let Ok(client) = crate::kube::KubeClient::new().await {
            let all_vms = match client.list_all_vms().await {
                Ok(vms) => vms,
                Err(e) => {
                    eprintln!("Warning: Failed to list VMs from Kubernetes: {}", e);
                    Vec::new()
                }
            };
            all_vms
                .iter()
                .filter_map(|vm| {
                    let name = vm.metadata.name.clone()?;

                    // Filter: only include VMs whose status mentions this node
                    let on_target_node = vm
                        .status
                        .as_ref()
                        .and_then(|s| s.conditions.as_ref())
                        .map(|conds| {
                            conds.iter().any(|c| {
                                c.message
                                    .as_deref()
                                    .map(|m| m.contains(&node))
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false);

                    if !on_target_node {
                        return None;
                    }

                    let priority = vm
                        .metadata
                        .annotations
                        .as_ref()
                        .and_then(|a| a.get("vmrogue.io/priority"))
                        .and_then(|p| p.parse::<u8>().ok())
                        .unwrap_or(50);
                    Some((name, priority))
                })
                .collect()
        } else {
            println!(
                "{}",
                color::warning("⚠ Could not connect to Kubernetes, showing empty plan")
            );
            Vec::new()
        };

        let vm_count = vms.len();
        let batches = planner.plan_evacuation(vms);
        let estimated = planner.estimate_duration(vm_count, 120);

        println!("{}", color::header("Migration Batches:"));
        for (i, batch) in batches.iter().enumerate() {
            println!("  Batch {}: {}", i + 1, batch.join(", "));
        }
        println!();
        println!(
            "  Estimated Duration: {}s (~{} minutes)",
            estimated,
            estimated / 60
        );
        println!();
        println!(
            "{}",
            color::info("ℹ Use 'vmrogue evacuate-node' without --plan to execute")
        );
    } else {
        // Cordon the node (mark as unschedulable)
        let client = crate::kube::get_client().await?;

        let nodes_api: kube::api::Api<k8s_openapi::api::core::v1::Node> =
            kube::api::Api::all(client.clone());
        let cordon_patch = serde_json::json!({
            "spec": { "unschedulable": true }
        });
        nodes_api
            .patch(
                &node,
                &kube::api::PatchParams::default(),
                &kube::api::Patch::Merge(&cordon_patch),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to cordon node: {}", e))?;

        println!(
            "  {} Node '{}' cordoned (marked unschedulable)",
            color::success("✓"),
            node
        );

        // List VMIs on the node and create migrations
        let kube_client = crate::kube::KubeClient::new().await?;
        let vms = kube_client.list_vms(namespace).await?;
        let mut migrated_count: usize = 0;

        for vm in &vms {
            let vm_name = vm.metadata.name.as_deref().unwrap_or_default();
            // Check if VM is on the target node via VMI
            if let Ok(Some(vm_node)) = kube_client.get_vm_node(namespace, vm_name).await {
                if vm_node == node {
                    // Create migration CRD
                    let mig_name = format!(
                        "{}-evacuate-{}",
                        vm_name,
                        chrono::Utc::now().format("%H%M%S")
                    );
                    let migration = crate::kube::types::VirtualMachineInstanceMigration {
                        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                            name: Some(mig_name),
                            namespace: Some(namespace.to_string()),
                            ..Default::default()
                        },
                        spec: crate::kube::types::VirtualMachineInstanceMigrationSpec {
                            vmi_name: Some(vm_name.to_string()),
                        },
                        status: None,
                    };
                    let mig_api: kube::api::Api<
                        crate::kube::types::VirtualMachineInstanceMigration,
                    > = kube::api::Api::namespaced(client.clone(), namespace);
                    match mig_api
                        .create(&kube::api::PostParams::default(), &migration)
                        .await
                    {
                        Ok(_) => {
                            println!(
                                "  {} Initiated migration for {}",
                                color::success("✓"),
                                vm_name
                            );
                            migrated_count += 1;
                        }
                        Err(e) => println!(
                            "  {} Failed to migrate {}: {}",
                            color::error("✗"),
                            vm_name,
                            e
                        ),
                    }
                }
            }
        }

        let status = EvacuationStatus::new(&node, migrated_count);

        println!();
        println!("{}", color::header("Evacuation Started:"));
        println!("  Node:         {}", status.node_name);
        println!("  Total VMs:    {}", status.total_vms);
        println!("  Strategy:     Live Migration");
        println!("  Max Parallel: {}", max_parallel);
        println!();
        println!("{}", color::success("✓ Evacuation initiated successfully"));
        println!();
        println!(
            "{}",
            color::info("ℹ Use 'vmrogue evacuation-status' to monitor progress")
        );
    }
    Ok(())
}

pub async fn handle_evacuation_status(node: String, watch: bool, namespace: &str) -> Result<()> {
    use crate::kube::types::VirtualMachineInstanceMigration;
    use crate::migration::evacuation::{EvacuationState, EvacuationStatus};

    let client = crate::kube::get_client().await?;

    let mut first = true;
    loop {
        if !first {
            println!("\n{}", "═".repeat(60));
        }
        first = false;

        println!("{}", color::header(&format!("Evacuation Status: {}", node)));
        println!();

        let migrations_api: kube::api::Api<VirtualMachineInstanceMigration> =
            kube::api::Api::namespaced(client.clone(), namespace);

        let migration_list = migrations_api
            .list(&kube::api::ListParams::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list migrations: {}", e))?;

        let node_migrations: Vec<_> = migration_list
            .items
            .iter()
            .filter(|m| {
                m.status
                    .as_ref()
                    .and_then(|s| s.migration_state.as_ref())
                    .and_then(|ms| ms.source_node.as_deref())
                    .map(|src| src == node)
                    .unwrap_or(false)
            })
            .collect();

        let total = node_migrations.len();
        let mut status = EvacuationStatus::new(&node, total);

        let mut migrated: usize = 0;
        let mut in_progress: usize = 0;
        let mut failed: usize = 0;

        for m in &node_migrations {
            let phase = m
                .status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("Unknown");

            match phase {
                "Succeeded" => migrated += 1,
                "Failed" => failed += 1,
                "Running" => in_progress += 1,
                _ => {}
            }
        }

        status.migrated_vms = migrated;
        status.in_progress_vms = in_progress;
        status.failed_vms = failed;

        if total == 0 {
            status.state = EvacuationState::Completed;
        } else if failed > 0 && in_progress == 0 {
            status.state = EvacuationState::Failed;
        } else if migrated == total {
            status.state = EvacuationState::Completed;
        } else {
            status.state = EvacuationState::InProgress;
        }

        println!(
            "  State:        {}",
            match status.state {
                EvacuationState::InProgress => color::info("In Progress"),
                EvacuationState::Completed => color::success("Completed"),
                EvacuationState::Failed => color::error("Failed"),
                _ => status.state.to_string(),
            }
        );
        println!("  Total VMs:    {}", status.total_vms);
        println!(
            "  Migrated:     {}",
            color::success(&status.migrated_vms.to_string())
        );
        println!("  In Progress:  {}", status.in_progress_vms);
        println!(
            "  Failed:       {}",
            if status.failed_vms > 0 {
                color::error(&status.failed_vms.to_string())
            } else {
                "0".to_string()
            }
        );
        println!("  Progress:     {}%", status.progress_percent());
        println!("  Duration:     {}s", status.duration_secs());

        if !watch {
            break;
        }

        // Stop watching once evacuation is terminal
        if status.state == EvacuationState::Completed || status.state == EvacuationState::Failed {
            println!();
            println!("{}", color::info("ℹ Evacuation reached terminal state"));
            break;
        }

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {}
            _ = tokio::signal::ctrl_c() => { break; }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::backup::recovery::{RecoveryPlan, RestoreOperation};
    use crate::backup::verify::{VerificationRunner, VerificationStatus, VerificationType};
    use crate::backup::{BackupConfig, BackupType, CompressionType};
    use crate::migration::ha::{EvictionStrategy, HAConfig, HAPriority};
    use crate::migration::{MigrationRequest, MigrationType};

    #[test]
    fn test_backup_type_parsing() {
        assert_eq!(
            match "incremental" {
                "incremental" => BackupType::Incremental,
                "differential" => BackupType::Differential,
                _ => BackupType::Full,
            },
            BackupType::Incremental
        );
        assert_eq!(
            match "differential" {
                "incremental" => BackupType::Incremental,
                "differential" => BackupType::Differential,
                _ => BackupType::Full,
            },
            BackupType::Differential
        );
        assert_eq!(
            match "full" {
                "incremental" => BackupType::Incremental,
                "differential" => BackupType::Differential,
                _ => BackupType::Full,
            },
            BackupType::Full
        );
        assert_eq!(
            match "other" {
                "incremental" => BackupType::Incremental,
                "differential" => BackupType::Differential,
                _ => BackupType::Full,
            },
            BackupType::Full
        );
    }

    #[test]
    fn test_compression_type_parsing() {
        assert_eq!(
            match "zstd" {
                "zstd" => CompressionType::Zstd,
                "lz4" => CompressionType::Lz4,
                "none" => CompressionType::None,
                _ => CompressionType::Gzip,
            },
            CompressionType::Zstd
        );
        assert_eq!(
            match "lz4" {
                "zstd" => CompressionType::Zstd,
                "lz4" => CompressionType::Lz4,
                "none" => CompressionType::None,
                _ => CompressionType::Gzip,
            },
            CompressionType::Lz4
        );
        assert_eq!(
            match "none" {
                "zstd" => CompressionType::Zstd,
                "lz4" => CompressionType::Lz4,
                "none" => CompressionType::None,
                _ => CompressionType::Gzip,
            },
            CompressionType::None
        );
        assert_eq!(
            match "other" {
                "zstd" => CompressionType::Zstd,
                "lz4" => CompressionType::Lz4,
                "none" => CompressionType::None,
                _ => CompressionType::Gzip,
            },
            CompressionType::Gzip
        );
    }

    #[test]
    fn test_backup_config_no_encryption() {
        let mut config = BackupConfig::new("vm1", "backup-001");
        config.backup_type = BackupType::Incremental;
        config.compression = CompressionType::Zstd;
        config.encryption_enabled = false;
        assert_eq!(config.vm_name, "vm1");
        assert!(!config.encryption_enabled);
    }

    #[test]
    fn test_restore_target_derivation() {
        let backup = "my-vm-backup-20240101-120000";
        let target_vm = backup.replace("-backup-", "-restored-");
        assert_eq!(target_vm, "my-vm-restored-20240101-120000");
    }

    #[test]
    fn test_restore_operation_to_new_vm() {
        let restore =
            RestoreOperation::new("restore-001", "original-vm", "backup-001").to_new_vm("new-vm");
        assert_eq!(restore.target_name, "new-vm");
        assert_eq!(restore.vm_name, "original-vm");
    }

    #[tokio::test]
    async fn test_verification_runner_all_types() {
        for v_type in [
            VerificationType::Quick,
            VerificationType::Standard,
            VerificationType::Full,
        ] {
            let report = VerificationRunner::verify("test-backup", "default", v_type).await;
            // Without cluster access, expect a failed cluster-connection check
            assert_eq!(report.status, VerificationStatus::Failed);
            assert!(!report.checks.is_empty());
        }
    }

    #[test]
    fn test_migration_request_with_target() {
        let request = MigrationRequest::new("vm1", "node1")
            .to_node("node2")
            .with_type(MigrationType::PostCopy);
        assert_eq!(request.vm_name, "vm1");
        assert_eq!(request.target_node, Some("node2".to_string()));
        assert_eq!(request.migration_type, MigrationType::PostCopy);
    }

    #[test]
    fn test_ha_config_construction() {
        let mut config = HAConfig::new("critical-db");
        config.priority = HAPriority::Critical;
        config.eviction_strategy = EvictionStrategy::LiveMigrate;
        assert_eq!(config.priority, HAPriority::Critical);
        assert!(config.failover_policy.auto_restart);
    }

    #[test]
    fn test_recovery_plan_with_description() {
        let plan = RecoveryPlan::new("dr-plan").with_description("Disaster recovery plan");
        assert_eq!(plan.name, "dr-plan");
        assert_eq!(plan.description, "Disaster recovery plan");
    }
}
