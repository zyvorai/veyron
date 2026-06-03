// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! # VMRogue
//!
//! Craft VMs for KubeVirt with Rust power.
//!
//! VMRogue provides a CLI and library for managing KubeVirt virtual machines
//! on Kubernetes clusters. It handles VM configuration, template management,
//! snapshots, backups, monitoring, and more.
//!
//! ## Quick Start (Library Usage)
//!
//! ```
//! use vmrogue::{VMConfigBuilder, vm_config_to_kubevirt, to_yaml};
//!
//! // Build a VM configuration
//! let config = VMConfigBuilder::new("web-server")
//!     .namespace("production")
//!     .cpu(4, 1, 1)
//!     .memory("8Gi")
//!     .add_blank_disk("rootdisk", "40Gi", 1)
//!     .add_pod_network("eth0")
//!     .label("app", "nginx")
//!     .build();
//!
//! // Convert to KubeVirt manifest
//! let vm = vm_config_to_kubevirt(&config).unwrap();
//!
//! // Serialize to YAML
//! let yaml = to_yaml(&config).unwrap();
//! assert!(yaml.contains("web-server"));
//! ```

pub mod cli;
pub mod config;
pub mod copilot;
pub mod kube;
pub mod network;
pub mod output;
pub mod storage;
pub mod templates;
pub mod tui;
pub mod utils;

// Innovative features
pub mod aiml;
pub mod api;
pub mod automation;
pub mod backup;
pub mod blueprints;
pub mod capacity;
pub mod catalog;
pub mod compliance;
pub mod cost;
pub mod devexp;
pub mod disk;
pub mod dr;
pub mod edge;
pub mod finops;
pub mod gitops;
pub mod handlers;
pub mod health;
pub mod migration;
pub mod monitoring;
pub mod multicloud;
pub mod multitenancy;
pub mod networking;
pub mod observability;
pub mod profiles;
pub mod secrets;
pub mod security;
pub mod soc;
pub mod servicemesh;
pub mod snapshots;

// Operator CRD types (vmrogue.io/v1alpha1)
pub mod operator_crds;

// Features ported from v9s
pub mod advanced_filter;
pub mod ai_troubleshoot;
pub mod audit_trail;
pub mod autoscaler;
pub mod change_approval;
pub mod cluster_health;
pub mod console_panel;
pub mod dependency_graph;
pub mod disk_conversion;
pub mod hypervisor;
pub mod macros;
pub mod multi_cluster;
pub mod nlp_search;
pub mod notifications;
pub mod placement;
pub mod recommendation;
pub mod search_history;
pub mod session_sharing;
pub mod state_persistence;
pub mod topology;

// Convenience re-exports for public API
pub use config::{VMConfig, VMConfigBuilder};
pub use kube::converter::vm_config_to_kubevirt;
pub use output::{OutputFormat, format_output, to_json, to_yaml};
pub use utils::{
    VMRogueError, format_bytes, generate_id, parse_cpu_nanocores, parse_memory_bytes,
    parse_memory_gib, percent_to_u8,
};

use anyhow::Result;
use cli::{Cli, Commands};
use config::AppConfig;

/// Main entry point for the library
pub async fn run(mut cli: Cli) -> Result<()> {
    // Disable colored output if --no-color, NO_COLOR is set, or stdout is not a TTY
    use std::io::IsTerminal;
    if cli.no_color || std::env::var_os("NO_COLOR").is_some() || !std::io::stdout().is_terminal() {
        colored::control::set_override(false);
    }

    // Load application config file
    let app_config = if let Some(ref config_path) = cli.config {
        match AppConfig::load_from(std::path::PathBuf::from(config_path)) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to load config file '{}': {}",
                    config_path, e
                );
                AppConfig::default()
            }
        }
    } else {
        AppConfig::load().unwrap_or_default()
    };

    // Apply config file defaults where CLI didn't override.
    // We detect "not explicitly set" by checking against clap's default_value.
    if cli.kubeconfig.is_none() {
        if let Some(ref kc) = app_config.kubeconfig {
            cli.kubeconfig = Some(kc.clone());
        }
    }
    if let Some(ref kc) = cli.kubeconfig {
        crate::kube::set_kubeconfig_path(kc.clone());
    }
    // Only apply config namespace if CLI still has the clap default
    if cli.namespace == "default" && app_config.namespace != "default" {
        cli.namespace = app_config.namespace.clone();
    }

    // Initialize logging based on config + CLI
    let log_level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        match app_config.logging.level.as_str() {
            "error" => log::LevelFilter::Error,
            "warn" => log::LevelFilter::Warn,
            "debug" => log::LevelFilter::Debug,
            "trace" => log::LevelFilter::Trace,
            _ => log::LevelFilter::Info,
        }
    };

    env_logger::Builder::from_default_env()
        .filter_level(log_level)
        .try_init()
        .ok();

    match *cli.command {
        // ========== CORE VM MANAGEMENT ==========
        Commands::Create {
            name,
            template,
            from_file,
            cpus,
            memory,
            disk_size,
            storage_class,
            container_disk,
            cloud_init,
            no_internet,
            dry_run,
            output,
        } => {
            handlers::vm::handle_create(
                name,
                template,
                from_file,
                cpus,
                memory,
                disk_size,
                storage_class,
                container_disk,
                cloud_init,
                no_internet,
                dry_run,
                output,
                &cli.namespace,
            )
            .await?;
        }

        Commands::List {
            all_namespaces,
            output,
        } => {
            handlers::vm::handle_list(all_namespaces, output, &cli.namespace).await?;
        }

        Commands::Get { name, output } => {
            handlers::vm::handle_get(name, output, &cli.namespace).await?;
        }

        Commands::Delete { name, yes } => {
            handlers::vm::handle_delete(name, yes, &cli.namespace).await?;
        }

        Commands::Start { name } => {
            handlers::vm::handle_start(name, &cli.namespace).await?;
        }

        Commands::Stop { name } => {
            handlers::vm::handle_stop(name, &cli.namespace).await?;
        }

        Commands::Restart { name } => {
            handlers::vm::handle_restart(name, &cli.namespace).await?;
        }

        Commands::Pause { name } => {
            handlers::vm::handle_pause(name, &cli.namespace).await?;
        }

        Commands::Unpause { name } => {
            handlers::vm::handle_unpause(name, &cli.namespace).await?;
        }

        Commands::GuestFreeze { name } => {
            handlers::vm::handle_guest_freeze(name, &cli.namespace).await?;
        }

        Commands::GuestUnfreeze { name } => {
            handlers::vm::handle_guest_unfreeze(name, &cli.namespace).await?;
        }

        Commands::GuestSoftreboot { name } => {
            handlers::vm::handle_guest_softreboot(name, &cli.namespace).await?;
        }

        Commands::VolumeAdd {
            name,
            volume_name,
            pvc,
        } => {
            handlers::vm::handle_volume_add(name, volume_name, pvc, &cli.namespace).await?;
        }

        Commands::VolumeRemove { name, volume_name } => {
            handlers::vm::handle_volume_remove(name, volume_name, &cli.namespace).await?;
        }

        Commands::Resize { name, cpus, memory } => {
            handlers::vm::handle_resize(name, cpus, memory, &cli.namespace).await?;
        }

        Commands::Console { name } => {
            handlers::vm::handle_console(name, &cli.namespace).await?;
        }

        Commands::Ssh { name, user } => {
            handlers::vm::handle_ssh(name, user, &cli.namespace).await?;
        }

        Commands::SshKeyInject {
            name,
            public_key_file,
            public_key,
            guest_user,
            secret_name,
            config_drive,
        } => {
            handlers::vm::handle_ssh_key_inject(
                name,
                &cli.namespace,
                public_key_file,
                public_key,
                guest_user,
                secret_name,
                config_drive,
            )
            .await?;
        }

        Commands::Vnc { name } => {
            handlers::vm::handle_vnc(name, &cli.namespace).await?;
        }

        Commands::Logs { name, follow, tail } => {
            handlers::vm::handle_logs(name, follow, tail, &cli.namespace).await?;
        }

        Commands::Generate {
            name,
            template,
            from_file,
            cpus,
            memory,
            disk_size,
            output,
            format,
            kubevirt,
        } => {
            handlers::vm::handle_generate(
                name,
                template,
                from_file,
                cpus,
                memory,
                disk_size,
                output,
                format,
                kubevirt,
                &cli.namespace,
            )?;
        }

        Commands::Templates { by_family, source } => {
            handlers::vm::handle_templates(by_family, source.as_deref()).await?;
        }

        Commands::Catalog { action } => match action {
            cli::CatalogAction::Export { output } => {
                handlers::catalog::handle_catalog_export(output)?;
            }
            cli::CatalogAction::Sync { namespace } => {
                handlers::catalog::handle_catalog_sync(namespace).await?;
            }
            cli::CatalogAction::List {
                templates,
                profiles,
            } => {
                handlers::catalog::handle_catalog_list(templates, profiles).await?;
            }
        },

        Commands::Template { name, output } => {
            handlers::vm::handle_template(name, output)?;
        }

        Commands::Validate { file } => {
            handlers::vm::handle_validate(file)?;
        }

        Commands::Status {
            name,
            watch,
            interval,
        } => {
            handlers::vm::handle_status(name, watch, interval, &cli.namespace).await?;
        }

        Commands::Clone {
            source,
            target,
            start,
        } => {
            handlers::vm::handle_clone(source, target, start, &cli.namespace).await?;
        }

        Commands::Resources {
            all_namespaces,
            sort_by,
        } => {
            handlers::vm::handle_resources(all_namespaces, sort_by, &cli.namespace).await?;
        }

        Commands::Export {
            name,
            output,
            kubevirt,
        } => {
            handlers::vm::handle_export(name, output, kubevirt, &cli.namespace).await?;
        }

        Commands::Wizard { name } => {
            handlers::vm::handle_wizard(name, &cli.namespace).await?;
        }

        Commands::Batch {
            file,
            namespace,
            dry_run,
            continue_on_error,
        } => {
            handlers::vm::handle_batch(file, namespace, dry_run, continue_on_error, &cli.namespace)
                .await?;
        }

        // ========== INNOVATIVE FEATURES ==========
        Commands::Profiles { details } => {
            handlers::profiles::handle_profiles(details)?;
        }

        Commands::Profile { name, output } => {
            handlers::profiles::handle_profile(name, output)?;
        }

        Commands::ProfileCreate {
            name,
            cpus,
            sockets,
            threads,
            memory,
            disk_size,
            description,
            use_cases,
            recommended_os,
            from_file,
        } => {
            handlers::profiles::handle_profile_create(
                name,
                cpus,
                sockets,
                threads,
                memory,
                disk_size,
                description,
                use_cases,
                recommended_os,
                from_file,
            )?;
        }

        Commands::ProfileEdit {
            name,
            cpus,
            sockets,
            threads,
            memory,
            disk_size,
            description,
            use_cases,
            recommended_os,
        } => {
            handlers::profiles::handle_profile_edit(
                name,
                cpus,
                sockets,
                threads,
                memory,
                disk_size,
                description,
                use_cases,
                recommended_os,
            )?;
        }

        Commands::ProfileDelete { name, yes } => {
            handlers::profiles::handle_profile_delete(name, yes)?;
        }

        Commands::Blueprints { tag, details } => {
            handlers::profiles::handle_blueprints(tag, details)?;
        }

        Commands::Blueprint { name, output } => {
            handlers::profiles::handle_blueprint(name, output)?;
        }

        Commands::Deploy {
            blueprint,
            prefix,
            start,
            dry_run,
        } => {
            handlers::profiles::handle_deploy(
                blueprint,
                prefix,
                start,
                dry_run,
                cli.namespace.clone(),
            )
            .await?;
        }

        Commands::BlueprintCreate {
            name,
            from_file,
            description,
        } => {
            handlers::profiles::handle_blueprint_create(name, from_file, description)?;
        }

        Commands::BlueprintEdit { name, description } => {
            handlers::profiles::handle_blueprint_edit(name, description)?;
        }

        Commands::BlueprintDelete { name, yes } => {
            handlers::profiles::handle_blueprint_delete(name, yes)?;
        }

        Commands::BlueprintValidate { file, detailed } => {
            handlers::profiles::handle_blueprint_validate(file, detailed)?;
        }

        Commands::Health { target, detailed } => {
            handlers::profiles::handle_health(target, detailed, cli.namespace.clone()).await?;
        }

        Commands::Recommend {
            workload,
            alternatives,
        } => {
            handlers::profiles::handle_recommend(workload, alternatives)?;
        }

        // ========== VM SNAPSHOTS & BACKUP ==========
        Commands::SnapshotCreate {
            vm,
            name,
            description,
        } => handlers::infra::handle_snapshot_create(vm, name, description, &cli.namespace).await?,

        Commands::SnapshotList {
            vm,
            all_namespaces,
            output,
        } => {
            handlers::infra::handle_snapshot_list(vm, all_namespaces, output, &cli.namespace)
                .await?
        }

        Commands::SnapshotGet { name, output } => {
            handlers::infra::handle_snapshot_get(name, output, &cli.namespace).await?
        }

        Commands::SnapshotDelete { name, yes } => {
            handlers::infra::handle_snapshot_delete(name, yes, &cli.namespace).await?
        }

        Commands::SnapshotRestore {
            snapshot,
            target,
            in_place,
            start,
        } => {
            handlers::infra::handle_snapshot_restore(
                snapshot,
                target,
                in_place,
                start,
                &cli.namespace,
            )
            .await?
        }

        // ========== PERFORMANCE MONITORING ==========
        Commands::MonitorLive { vm, interval } => {
            handlers::infra::handle_monitor_live(vm, interval, &cli.namespace).await?
        }

        Commands::MonitorStats { vm, period, output } => {
            handlers::infra::handle_monitor_stats(vm, period, output, &cli.namespace).await?
        }

        Commands::MonitorCompare { vms, output } => {
            handlers::infra::handle_monitor_compare(vms, output, &cli.namespace).await?
        }

        Commands::MonitorTop {
            all_namespaces,
            sort_by,
            limit,
        } => {
            handlers::infra::handle_monitor_top(all_namespaces, sort_by, limit, &cli.namespace)
                .await?
        }

        // ========== DISK MANAGEMENT ==========
        Commands::DiskExpand {
            vm,
            disk,
            size,
            pvc,
            plan,
        } => handlers::infra::handle_disk_expand(vm, disk, size, pvc, plan, &cli.namespace).await?,

        Commands::DiskHealth { vm, detailed } => {
            handlers::infra::handle_disk_health(vm, detailed, &cli.namespace).await?
        }

        Commands::DiskScript {
            filesystem,
            device,
            output,
            dry_run,
        } => handlers::infra::handle_disk_script(filesystem, device, output, dry_run)?,

        Commands::DiskUsage {
            vm,
            sort_by,
            output,
        } => handlers::infra::handle_disk_usage(vm, sort_by, output, &cli.namespace).await?,

        // ========== NETWORK MANAGEMENT ==========
        Commands::NetworkList { vm, output } => {
            handlers::infra::handle_network_list(vm, output, &cli.namespace).await?
        }

        Commands::NetworkGet {
            vm,
            interface,
            output,
        } => handlers::infra::handle_network_get(vm, interface, output, &cli.namespace).await?,

        Commands::NetworkBandwidth {
            vm,
            interface,
            watch,
            interval,
        } => {
            handlers::infra::handle_network_bandwidth(
                vm,
                interface,
                watch,
                interval,
                &cli.namespace,
            )
            .await?
        }

        Commands::NetworkTraffic {
            vm,
            interface,
            period,
            top,
            output,
        } => handlers::infra::handle_network_traffic(vm, interface, period, top, output)?,

        Commands::NetworkPolicies {
            all_namespaces,
            output,
        } => {
            handlers::infra::handle_network_policies(all_namespaces, output, &cli.namespace).await?
        }

        Commands::NetworkPolicy { name, output } => {
            handlers::infra::handle_network_policy(name, output)?
        }

        Commands::Migrate {
            vm,
            target_node,
            migration_type,
            plan,
        } => {
            handlers::backup::handle_migrate(vm, target_node, migration_type, plan, &cli.namespace)
                .await?;
        }

        Commands::MigrationStatus {
            vm,
            watch,
            interval,
        } => {
            handlers::backup::handle_migration_status(vm, watch, interval, &cli.namespace).await?;
        }

        Commands::MigrationList {
            all_namespaces,
            state,
            output,
        } => {
            handlers::backup::handle_migration_list(all_namespaces, state, output, &cli.namespace)
                .await?;
        }

        Commands::HAConfig {
            vm,
            enable,
            disable,
            priority,
            eviction_strategy,
        } => {
            handlers::backup::handle_ha_config(
                vm,
                enable,
                disable,
                priority,
                eviction_strategy,
                &cli.namespace,
            )
            .await?;
        }

        Commands::HAStatus { vm, output } => {
            handlers::backup::handle_ha_status(vm, output, &cli.namespace)?;
        }

        Commands::EvacuateNode {
            node,
            reason,
            max_parallel,
            timeout,
            force,
            plan,
        } => {
            handlers::backup::handle_evacuate_node(
                node,
                reason,
                max_parallel,
                timeout,
                force,
                plan,
                &cli.namespace,
            )
            .await?;
        }

        Commands::EvacuationStatus { node, watch } => {
            handlers::backup::handle_evacuation_status(node, watch, &cli.namespace).await?;
        }

        Commands::BackupCreate {
            vm,
            name,
            backup_type,
            compression,
            no_encryption,
        } => {
            handlers::backup::handle_backup_create(
                vm,
                name,
                backup_type,
                compression,
                no_encryption,
                &cli.namespace,
            )
            .await?;
        }

        Commands::BackupList { vm, output } => {
            handlers::backup::handle_backup_list(vm, output, &cli.namespace).await?;
        }

        Commands::BackupGet { name, output } => {
            handlers::backup::handle_backup_get(name, output, &cli.namespace)?;
        }

        Commands::BackupDelete { name, yes } => {
            handlers::backup::handle_backup_delete(name, yes, &cli.namespace).await?;
        }

        Commands::BackupRestore {
            backup,
            target,
            start,
        } => {
            handlers::backup::handle_backup_restore(backup, target, start, &cli.namespace).await?;
        }

        Commands::BackupVerify {
            name,
            verification_type,
        } => {
            handlers::backup::handle_backup_verify(name, verification_type, &cli.namespace).await?;
        }

        Commands::BackupSchedules { output } => {
            handlers::backup::handle_backup_schedules(output, &cli.namespace)?;
        }

        Commands::BackupScheduleCreate { name, schedule, vm } => {
            handlers::backup::handle_backup_schedule_create(name, schedule, vm, &cli.namespace)?;
        }

        Commands::RecoveryPlan { name, output } => {
            handlers::backup::handle_recovery_plan(name, output, &cli.namespace)?;
        }

        Commands::RecoveryExecute { plan, dry_run } => {
            handlers::backup::handle_recovery_execute(plan, dry_run, &cli.namespace).await?;
        }

        // ========== SECURITY & COMPLIANCE ==========
        Commands::SecurityScan {
            vm,
            scan_type,
            containers,
            output,
        } => {
            handlers::security::handle_security_scan(
                vm,
                scan_type,
                containers,
                output,
                &cli.namespace,
            )?;
        }

        Commands::SecurityAssess { vm, output } => {
            handlers::security::handle_security_assess(vm, output, &cli.namespace)?;
        }

        Commands::SecurityHarden {
            vm,
            profile,
            verify_only,
        } => {
            handlers::security::handle_security_harden(vm, profile, verify_only, &cli.namespace)?;
        }

        Commands::SecurityProfiles { details } => {
            handlers::security::handle_security_profiles(details)?;
        }

        Commands::ComplianceCheck {
            vm,
            framework,
            output,
        } => {
            handlers::security::handle_compliance_check(vm, framework, output, &cli.namespace)
                .await?;
        }

        Commands::ComplianceReport {
            vm,
            report_id,
            output,
        } => {
            handlers::security::handle_compliance_report(vm, report_id, output, &cli.namespace)
                .await?;
        }

        Commands::AuditList {
            vm,
            event_type,
            severity,
            security_only,
            output,
        } => {
            handlers::security::handle_audit_list(
                vm,
                event_type,
                severity,
                security_only,
                output,
                &cli.namespace,
            )?;
        }

        Commands::AuditGet { log_id, output } => {
            handlers::security::handle_audit_get(log_id, output, &cli.namespace)?;
        }

        Commands::AuditStats { vm, period, output } => {
            handlers::security::handle_audit_stats(vm, period, output, &cli.namespace)?;
        }

        // ========== COST MANAGEMENT & OPTIMIZATION ==========
        Commands::CostAnalyze { vm, period, output } => {
            handlers::cost::handle_cost_analyze(vm, period, output, &cli.namespace).await?
        }

        Commands::CostSummary {
            namespace,
            period,
            group_by,
            output,
        } => handlers::cost::handle_cost_summary(namespace, period, group_by, output).await?,

        Commands::CostReport {
            report_type,
            format,
            output,
        } => handlers::cost::handle_cost_report(report_type, format, output)?,

        Commands::BudgetList { output } => handlers::cost::handle_budget_list(output)?,

        Commands::BudgetCreate {
            name,
            amount,
            period,
            scope,
            alert_threshold,
        } => handlers::cost::handle_budget_create(name, amount, period, scope, alert_threshold)?,

        Commands::BudgetStatus { name, output } => {
            handlers::cost::handle_budget_status(name, output)?
        }

        Commands::CostOptimize {
            vm,
            high_priority_only,
            output,
        } => handlers::cost::handle_cost_optimize(vm, high_priority_only, output)?,

        Commands::CostWaste {
            waste_type,
            min_waste,
            output,
        } => handlers::cost::handle_cost_waste(waste_type, min_waste, output)?,

        Commands::CostForecast {
            budget,
            period,
            output,
        } => handlers::cost::handle_cost_forecast(budget, period, output)?,

        // ========== AUTOMATION & ORCHESTRATION ==========
        Commands::AutomationList {
            enabled_only,
            output,
        } => handlers::automation::handle_automation_list(enabled_only, output)?,
        Commands::AutomationCreate {
            name,
            description,
            trigger,
            enable,
        } => handlers::automation::handle_automation_create(name, description, trigger, enable)?,
        Commands::AutomationGet { rule, output } => {
            handlers::automation::handle_automation_get(rule, output)?
        }
        Commands::AutomationRun { rule, dry_run } => {
            handlers::automation::handle_automation_run(rule, dry_run)?
        }
        Commands::WorkflowList { output } => handlers::automation::handle_workflow_list(output)?,
        Commands::WorkflowCreate {
            name,
            description,
            template,
        } => handlers::automation::handle_workflow_create(name, description, template)?,
        Commands::WorkflowGet { workflow, output } => {
            handlers::automation::handle_workflow_get(workflow, output)?
        }
        Commands::WorkflowRun { workflow, watch } => {
            handlers::automation::handle_workflow_run(workflow, watch)?
        }
        Commands::WorkflowExecutions {
            workflow,
            limit,
            output,
        } => handlers::automation::handle_workflow_executions(workflow, limit, output)?,
        Commands::ScheduleList {
            enabled_only,
            output,
        } => handlers::automation::handle_schedule_list(enabled_only, output)?,
        Commands::ScheduleCreate {
            name,
            rule,
            schedule,
            enable,
        } => handlers::automation::handle_schedule_create(name, rule, schedule, enable)?,

        // ========== OBSERVABILITY & ANALYTICS ==========
        Commands::LogsQuery {
            start,
            end,
            level,
            source,
            search,
            limit,
        } => handlers::observability::handle_logs_query(start, end, level, source, search, limit)?,
        Commands::LogsStats { group_by } => handlers::observability::handle_logs_stats(group_by)?,
        Commands::LogsPatterns { min_count } => {
            handlers::observability::handle_logs_patterns(min_count)?
        }
        Commands::MetricsCollect { vm } => {
            handlers::observability::handle_metrics_collect(vm).await?
        }
        Commands::MetricsQuery {
            name,
            start,
            end,
            aggregation,
        } => handlers::observability::handle_metrics_query(name, start, end, aggregation)?,
        Commands::MetricsSnapshot {
            vm,
            cpu_threshold,
            memory_threshold,
        } => handlers::observability::handle_metrics_snapshot(vm, cpu_threshold, memory_threshold)?,
        Commands::AlertsList {
            enabled_only,
            severity,
            output,
        } => handlers::observability::handle_alerts_list(enabled_only, severity, output)?,
        Commands::AlertsCreate {
            name,
            severity,
            metric,
            operator,
            threshold,
            duration,
        } => handlers::observability::handle_alerts_create(
            name, severity, metric, operator, threshold, duration,
        )?,
        Commands::AlertsActive { severity, output } => {
            handlers::observability::handle_alerts_active(severity, output)?
        }
        Commands::AlertsResolve { alert_id } => {
            handlers::observability::handle_alerts_resolve(alert_id)?
        }
        Commands::InsightsGenerate {
            vm,
            insight_type,
            min_severity,
        } => handlers::observability::handle_insights_generate(vm, insight_type, min_severity)?,
        Commands::Recommendations {
            category,
            min_priority,
            with_savings,
            output,
        } => handlers::observability::handle_recommendations(
            category,
            min_priority,
            with_savings,
            output,
        )?,
        Commands::TrendsAnalyze {
            metric,
            window,
            threshold,
        } => handlers::observability::handle_trends_analyze(metric, window, threshold)?,
        Commands::HealthCheck { component, output } => {
            handlers::observability::handle_health_check(component, output).await?
        }

        // ========== MULTI-TENANCY & RBAC ==========
        Commands::TenantsList {
            active_only,
            output,
        } => handlers::multitenancy::handle_tenants_list(active_only, output)?,
        Commands::TenantsCreate {
            name,
            owner,
            email,
            description,
            namespace,
        } => handlers::multitenancy::handle_tenants_create(
            name,
            owner,
            email,
            description,
            namespace,
        )?,
        Commands::TenantsShow { tenant, output } => {
            handlers::multitenancy::handle_tenants_show(tenant, output)?
        }
        Commands::TenantsDelete { tenant, yes } => {
            handlers::multitenancy::handle_tenants_delete(tenant, yes)?
        }
        Commands::UsersList {
            active_only,
            group,
            output,
        } => handlers::multitenancy::handle_users_list(active_only, group, output)?,
        Commands::UsersCreate {
            username,
            email,
            role,
            group,
        } => handlers::multitenancy::handle_users_create(username, email, role, group)?,
        Commands::UsersAssignRole { user, role, scope } => {
            handlers::multitenancy::handle_users_assign_role(user, role, scope)?
        }
        Commands::RolesList {
            builtin,
            custom,
            output,
        } => handlers::multitenancy::handle_roles_list(builtin, custom, output)?,
        Commands::RolesShow { role, output } => {
            handlers::multitenancy::handle_roles_show(role, output)?
        }
        Commands::RolesCreate {
            name,
            description,
            permissions,
        } => handlers::multitenancy::handle_roles_create(name, description, permissions)?,
        Commands::QuotasList {
            namespace,
            exceeded,
            output,
        } => handlers::multitenancy::handle_quotas_list(namespace, exceeded, output)?,
        Commands::QuotasCreate {
            name,
            namespace,
            preset,
        } => handlers::multitenancy::handle_quotas_create(name, namespace, preset).await?,
        Commands::QuotasShow {
            quota,
            utilization,
            output,
        } => handlers::multitenancy::handle_quotas_show(quota, utilization, output)?,
        Commands::GroupsList { output } => handlers::multitenancy::handle_groups_list(output)?,
        Commands::GroupsCreate {
            name,
            description,
            role,
        } => handlers::multitenancy::handle_groups_create(name, description, role)?,
        Commands::GroupsAddUser { group, user } => {
            handlers::multitenancy::handle_groups_add_user(group, user)?
        }

        // ========== DEVELOPER EXPERIENCE & TOOLING ==========
        Commands::Completions {
            shell,
            output,
            install,
        } => handlers::devexp::handle_completions(shell, output, install)?,

        Commands::ConfigSave {
            name,
            file,
            description,
            category,
            tags,
        } => handlers::devexp::handle_config_save(name, file, description, category, tags)?,

        Commands::ConfigLoad {
            name,
            output,
            format,
        } => handlers::devexp::handle_config_load(name, output, format)?,

        Commands::ConfigList {
            category,
            tag,
            sort_by,
            output,
        } => handlers::devexp::handle_config_list(category, tag, sort_by, output)?,

        Commands::ConfigDelete { name, yes } => handlers::devexp::handle_config_delete(name, yes)?,

        Commands::Diff {
            source,
            target,
            show_unchanged,
            output,
        } => handlers::devexp::handle_diff(source, target, show_unchanged, output)?,

        Commands::Init {
            name,
            project_type,
            directory,
            namespace,
            no_examples,
            ci,
            no_git,
        } => handlers::devexp::handle_init(
            name,
            project_type,
            directory,
            namespace,
            no_examples,
            ci,
            no_git,
        )?,

        Commands::Info {
            detailed,
            diagnostics,
            output,
        } => handlers::devexp::handle_info(detailed, diagnostics, output, &cli.namespace)?,
        // ========== API & REST INTERFACE ==========
        Commands::ApiServe {
            port,
            host,
            tls,
            tls_cert,
            tls_key,
            auth,
            rate_limit,
        } => {
            let port = port.unwrap_or(app_config.api.port);
            let host = host.unwrap_or_else(|| app_config.api.host.clone());
            let tls = tls || app_config.api.tls;
            let tls_cert = tls_cert.or(app_config.api.tls_cert.clone());
            let tls_key = tls_key.or(app_config.api.tls_key.clone());
            let auth = auth.unwrap_or_else(|| app_config.api.auth.clone());
            let rate_limit = rate_limit.unwrap_or(app_config.api.rate_limit);
            handlers::api::handle_api_serve(
                port,
                host,
                cli.namespace.clone(),
                tls,
                tls_cert,
                tls_key,
                auth,
                rate_limit,
            )
            .await?;
        }
        Commands::ApiStatus { output } => handlers::api::handle_api_status(output)?,
        Commands::ApiRoutes { method, output } => handlers::api::handle_api_routes(method, output)?,
        Commands::ApiSpec { format, output } => handlers::api::handle_api_spec(format, output)?,
        Commands::ApiKeyList {
            active_only,
            output,
        } => handlers::api::handle_api_key_list(active_only, output)?,
        Commands::ApiKeyCreate {
            name,
            permissions,
            rate_limit,
        } => handlers::api::handle_api_key_create(name, permissions, rate_limit)?,
        Commands::ApiKeyDelete { key, yes } => handlers::api::handle_api_key_delete(key, yes)?,
        Commands::WebhookList {
            active_only,
            output,
        } => handlers::api::handle_webhook_list(active_only, output)?,
        Commands::WebhookCreate {
            name,
            url,
            events,
            secret,
        } => handlers::api::handle_webhook_create(name, url, events, secret)?,
        Commands::WebhookDelete { webhook, yes } => {
            handlers::api::handle_webhook_delete(webhook, yes)?
        }
        Commands::EventList { vm, limit, output } => {
            handlers::api::handle_event_list(cli.namespace.clone(), vm, limit, output)?
        }
        Commands::EventRecent { limit, output } => {
            handlers::api::handle_event_recent(cli.namespace.clone(), limit, output)?
        }
        Commands::Tui {
            no_splash,
            theme,
            basic,
        } => handlers::api::handle_tui(cli.namespace.clone(), theme, basic, no_splash).await?,

        // ========== CONFIGURATION ==========
        Commands::ConfigShow { path } => {
            let user_path = AppConfig::user_path()?;
            let system_path = AppConfig::system_path();
            if path {
                println!("{}", user_path.display());
            } else {
                use tui::colors::cli as color;
                println!("{}", color::header("VMRogue Configuration"));
                println!();
                println!(
                    "  System config: {}  {}",
                    system_path.display(),
                    if system_path.exists() {
                        color::success("(loaded)")
                    } else {
                        color::muted("(not found)")
                    }
                );
                println!(
                    "  User config:   {}  {}",
                    user_path.display(),
                    if user_path.exists() {
                        color::success("(loaded)")
                    } else {
                        color::muted("(not found)")
                    }
                );
                println!();
                println!("  Priority: CLI args > user config > system config > defaults");
                println!();

                let content = toml::to_string_pretty(&app_config)?;
                println!("{}", content);
            }
        }

        Commands::ConfigInit { force } => {
            use tui::colors::cli as color;
            let config_path = AppConfig::default_path()?;

            if config_path.exists() && !force {
                println!(
                    "{}",
                    color::warning(&format!(
                        "Config file already exists: {}",
                        config_path.display()
                    ))
                );
                println!("  Use --force to overwrite");
                return Ok(());
            }

            AppConfig::default().save()?;
            println!(
                "{}",
                color::success(&format!("✓ Config file created: {}", config_path.display()))
            );
            println!();
            println!("Edit it to customize defaults:");
            println!("  namespace, API port/host, logging level, output format, etc.");
        }

        Commands::CommandList => {
            use tui::colors::cli as color;
            println!("{}", color::header("VMRogue Commands"));
            println!();
            let groups = [
                (
                    "VM Management",
                    vec![
                        "create",
                        "list",
                        "get",
                        "delete",
                        "start",
                        "stop",
                        "restart",
                        "pause",
                        "unpause",
                        "resize",
                        "console",
                        "ssh",
                        "vnc",
                        "logs",
                        "status",
                        "clone",
                        "resources",
                        "export",
                        "wizard",
                        "batch",
                    ],
                ),
                (
                    "Templates & Config",
                    vec!["generate", "templates", "template", "validate"],
                ),
                (
                    "Profiles & Blueprints",
                    vec![
                        "profiles",
                        "profile",
                        "profile-create",
                        "profile-edit",
                        "profile-delete",
                        "blueprints",
                        "blueprint",
                        "deploy",
                        "blueprint-create",
                        "blueprint-edit",
                        "blueprint-delete",
                        "blueprint-validate",
                        "health",
                        "recommend",
                    ],
                ),
                (
                    "Snapshots",
                    vec![
                        "snapshot-create",
                        "snapshot-list",
                        "snapshot-get",
                        "snapshot-delete",
                        "snapshot-restore",
                    ],
                ),
                (
                    "Monitoring",
                    vec![
                        "monitor-live",
                        "monitor-stats",
                        "monitor-compare",
                        "monitor-top",
                    ],
                ),
                (
                    "Disk Management",
                    vec!["disk-expand", "disk-health", "disk-script", "disk-usage"],
                ),
                (
                    "Network",
                    vec![
                        "network-list",
                        "network-get",
                        "network-bandwidth",
                        "network-traffic",
                        "network-policies",
                        "network-policy",
                    ],
                ),
                (
                    "Migration & HA",
                    vec![
                        "migrate",
                        "migration-status",
                        "migration-list",
                        "ha-config",
                        "ha-status",
                        "evacuate-node",
                        "evacuation-status",
                    ],
                ),
                (
                    "Backup & Recovery",
                    vec![
                        "backup-create",
                        "backup-list",
                        "backup-get",
                        "backup-delete",
                        "backup-restore",
                        "backup-verify",
                        "backup-schedules",
                        "backup-schedule-create",
                        "recovery-plan",
                        "recovery-execute",
                    ],
                ),
                (
                    "Security & Compliance",
                    vec![
                        "security-scan",
                        "security-assess",
                        "security-harden",
                        "security-profiles",
                        "compliance-check",
                        "compliance-report",
                        "audit-list",
                        "audit-get",
                        "audit-stats",
                    ],
                ),
                (
                    "Cost Management",
                    vec![
                        "cost-analyze",
                        "cost-summary",
                        "cost-report",
                        "budget-list",
                        "budget-create",
                        "budget-status",
                        "cost-optimize",
                        "cost-waste",
                        "cost-forecast",
                    ],
                ),
                (
                    "Automation",
                    vec![
                        "automation-list",
                        "automation-create",
                        "automation-get",
                        "automation-run",
                        "workflow-list",
                        "workflow-create",
                        "workflow-get",
                        "workflow-run",
                        "workflow-executions",
                        "schedule-list",
                        "schedule-create",
                    ],
                ),
                (
                    "Observability",
                    vec![
                        "logs-query",
                        "logs-stats",
                        "logs-patterns",
                        "metrics-collect",
                        "metrics-query",
                        "metrics-snapshot",
                        "alerts-list",
                        "alerts-create",
                        "alerts-active",
                        "alerts-resolve",
                        "insights-generate",
                        "recommendations",
                        "trends-analyze",
                        "health-check",
                    ],
                ),
                (
                    "Multi-Tenancy & RBAC",
                    vec![
                        "tenants-list",
                        "tenants-create",
                        "tenants-show",
                        "tenants-delete",
                        "users-list",
                        "users-create",
                        "users-assign-role",
                        "roles-list",
                        "roles-show",
                        "roles-create",
                        "quotas-list",
                        "quotas-create",
                        "quotas-show",
                        "groups-list",
                        "groups-create",
                        "groups-add-user",
                    ],
                ),
                (
                    "Developer Tools",
                    vec![
                        "completions",
                        "config-save",
                        "config-load",
                        "config-list",
                        "config-delete",
                        "diff",
                        "init",
                        "info",
                    ],
                ),
                (
                    "API & Interface",
                    vec![
                        "api-serve",
                        "api-status",
                        "api-routes",
                        "api-spec",
                        "api-key-list",
                        "api-key-create",
                        "api-key-delete",
                        "webhook-list",
                        "webhook-create",
                        "webhook-delete",
                        "tui",
                    ],
                ),
                (
                    "Configuration",
                    vec!["config-show", "config-init", "commands", "doctor"],
                ),
                (
                    "Infrastructure",
                    vec![
                        "events",
                        "nodes",
                        "pods",
                        "import",
                        "clusters-list",
                        "clusters-discover",
                    ],
                ),
                (
                    "GitOps",
                    vec!["gitops-export", "gitops-diff", "gitops-status"],
                ),
            ];

            for (group_name, cmds) in &groups {
                println!("  {}", color::header(group_name));
                for cmd in cmds {
                    println!("    {}", color::value(cmd));
                }
                println!();
            }

            println!(
                "{}",
                color::muted("Use 'vmrogue <command> --help' for details on a specific command")
            );
        }

        Commands::Doctor => {
            handlers::vm::handle_doctor(&cli.namespace).await?;
        }

        Commands::Ai { action, query } => {
            use cli::AiCommands;
            match action {
                Some(AiCommands::Doctor { name }) => {
                    handlers::copilot::handle_ai_doctor(&name, &cli.namespace, "text").await?;
                }
                Some(AiCommands::Scheduling { name }) => {
                    handlers::copilot::handle_ai_scheduling(&name, &cli.namespace, "text").await?;
                }
                Some(AiCommands::Explain { message }) => {
                    handlers::copilot::handle_ai_explain(&message, &cli.namespace).await?;
                }
                Some(AiCommands::Yaml {
                    name,
                    template,
                    cpus,
                    memory,
                    disk,
                    output,
                }) => {
                    handlers::copilot::handle_ai_yaml(
                        &cli.namespace,
                        name.as_deref(),
                        template.as_deref(),
                        cpus,
                        memory.as_deref(),
                        disk.as_deref(),
                        &output,
                    )
                    .await?;
                }
                Some(AiCommands::Recommend { description }) => {
                    handlers::copilot::handle_ai_recommend(&description).await?;
                }
                Some(AiCommands::Backup) => {
                    handlers::copilot::handle_ai_backup(&cli.namespace).await?;
                }
                Some(AiCommands::Cost { name, output }) => {
                    handlers::copilot::handle_ai_cost(&cli.namespace, name.as_deref(), &output)
                        .await?;
                }
                Some(AiCommands::Network { name }) => {
                    handlers::copilot::handle_ai_network(&name, &cli.namespace).await?;
                }
                Some(AiCommands::Guest { name }) => {
                    handlers::copilot::handle_ai_guest(&name, &cli.namespace, "text").await?;
                }
                Some(AiCommands::Filesystem { name, output }) => {
                    handlers::copilot::handle_ai_filesystem(&name, &cli.namespace, &output).await?;
                }
                Some(AiCommands::Storage) => {
                    handlers::copilot::handle_ai_storage(&cli.namespace, "text").await?;
                }
                Some(AiCommands::Security { name }) => {
                    handlers::copilot::handle_ai_security(&cli.namespace, name.as_deref(), "text")
                        .await?;
                }
                Some(AiCommands::Performance { output }) => {
                    handlers::copilot::handle_ai_performance(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Gitops { output }) => {
                    handlers::copilot::handle_ai_gitops(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Forecast { output }) => {
                    handlers::copilot::handle_ai_forecast(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Integrations { output }) => {
                    handlers::copilot::handle_ai_integrations(&output).await?;
                }
                Some(AiCommands::Pending { output }) => {
                    handlers::copilot::handle_ai_pending(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Compliance { output }) => {
                    handlers::copilot::handle_ai_compliance(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Observability { output }) => {
                    handlers::copilot::handle_ai_observability(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Cilium { output }) => {
                    handlers::copilot::handle_ai_cilium(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Nodes { output }) => {
                    handlers::copilot::handle_ai_nodes(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Drift { output }) => {
                    handlers::copilot::handle_ai_drift(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Alerts { output }) => {
                    handlers::copilot::handle_ai_alerts(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Slo { output }) => {
                    handlers::copilot::handle_ai_slo(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Migrations { output }) => {
                    handlers::copilot::handle_ai_migrations(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Quotas { output }) => {
                    handlers::copilot::handle_ai_quotas(&cli.namespace, &output).await?;
                }
                Some(AiCommands::Catalog { output }) => {
                    handlers::copilot::handle_ai_catalog(&cli.namespace, &output).await?;
                }
                Some(AiCommands::VeleroDr { output }) => {
                    handlers::copilot::handle_ai_velero_dr(&cli.namespace, &output).await?;
                }
                None => {
                    let q = query.join(" ");
                    if q.trim().is_empty() {
                        println!("Usage: vmrogue ai \"<question>\"  or  vmrogue ai doctor <name>");
                    } else {
                        handlers::copilot::handle_ai_ask(&q, &cli.namespace, None).await?;
                    }
                }
            }
        }

        // ========== GITOPS ==========
        Commands::GitopsExport { directory } => {
            handlers::gitops::handle_gitops_export(directory, &cli.namespace).await?;
        }

        Commands::GitopsDiff { directory } => {
            handlers::gitops::handle_gitops_diff(directory, &cli.namespace).await?;
        }

        Commands::GitopsStatus => {
            handlers::gitops::handle_gitops_status(&cli.namespace).await?;
        }

        // ========== MULTI-CLUSTER ==========
        Commands::ClustersList { output } => {
            handlers::vm::handle_clusters_list(output).await?;
        }

        Commands::ClustersDiscover => {
            handlers::vm::handle_clusters_discover().await?;
        }

        // ========== IMPORT ==========
        Commands::Import {
            file,
            start,
            dry_run,
        } => {
            handlers::vm::handle_import(file, start, dry_run, &cli.namespace).await?;
        }

        // ========== EVENTS, NODES, PODS (TOP-LEVEL) ==========
        Commands::EventsList { limit, output } => {
            handlers::vm::handle_events_list(limit, output, &cli.namespace).await?;
        }

        Commands::NodesList { output } => {
            handlers::vm::handle_nodes_list(output, &cli.namespace).await?;
        }

        Commands::PodsList { output } => {
            handlers::vm::handle_pods_list(output, &cli.namespace).await?;
        }

        // ========== SEARCH & TROUBLESHOOT ==========
        Commands::Search { query } => {
            handlers::vm::handle_search(query, &cli.namespace).await?;
        }

        Commands::Troubleshoot { name } => {
            handlers::vm::handle_troubleshoot(name, &cli.namespace).await?;
        }

        // ========== CAPACITY & PLACEMENT ==========
        Commands::Capacity { detailed, output } => {
            handlers::vm::handle_capacity(detailed, output, &cli.namespace).await?;
        }

        Commands::Placement { name, strategy } => {
            handlers::vm::handle_placement(name, strategy, &cli.namespace).await?;
        }

        Commands::ListJson => {
            handlers::vm::handle_list_json(&cli.namespace).await?;
        }

        // ========== OPERATOR CRD MANAGEMENT ==========
        Commands::VrvmList => {
            handlers::crds::handle_vrvm_list(&cli.namespace).await?;
        }
        Commands::VrvmGet { name } => {
            handlers::crds::handle_vrvm_get(&cli.namespace, &name).await?;
        }
        Commands::VrvmCreate {
            name,
            template,
            cpus,
            memory,
            disk,
            cdrom,
            network,
            cloud_init,
            firmware,
            secure_boot,
            tpm,
            no_rng,
            no_internet,
            machine_type,
            eviction_strategy,
            label,
            from_file,
            dry_run,
            start,
        } => {
            handlers::crds::handle_vrvm_create(
                &cli.namespace,
                &name,
                handlers::crds::VrvmCreateArgs {
                    template,
                    cpus,
                    memory,
                    disks: disk,
                    cdrom,
                    networks: network,
                    cloud_init,
                    firmware,
                    secure_boot,
                    tpm,
                    no_rng,
                    no_internet,
                    machine_type,
                    eviction_strategy,
                    labels: label,
                    from_file,
                    dry_run,
                    start,
                },
            )
            .await?;
        }
        Commands::VrvmApply { file, dry_run } => {
            handlers::crds::handle_vrvm_apply(&cli.namespace, &file, dry_run).await?;
        }
        Commands::VrvmDelete { name, yes } => {
            if !yes {
                use dialoguer::Confirm;
                let confirmed = Confirm::new()
                    .with_prompt(format!("Delete VMRogueVM '{}'?", name))
                    .default(false)
                    .interact()?;
                if !confirmed {
                    return Ok(());
                }
            }
            handlers::crds::handle_vrvm_delete(&cli.namespace, &name).await?;
        }
        Commands::VrbpList => {
            handlers::crds::handle_vrbp_list(&cli.namespace).await?;
        }
        Commands::VrbpGet { name } => {
            handlers::crds::handle_vrbp_get(&cli.namespace, &name).await?;
        }
        Commands::VrbpDelete { name } => {
            handlers::crds::handle_vrbp_delete(&cli.namespace, &name).await?;
        }
        Commands::VrpolList => {
            handlers::crds::handle_vrpol_list(&cli.namespace).await?;
        }
        Commands::VrpolGet { name } => {
            handlers::crds::handle_vrpol_get(&cli.namespace, &name).await?;
        }
        Commands::VrpolDelete { name } => {
            handlers::crds::handle_vrpol_delete(&cli.namespace, &name).await?;
        }
        Commands::VrinList => {
            handlers::crds::handle_vrin_list(&cli.namespace).await?;
        }
        Commands::VractList => {
            handlers::crds::handle_vract_list(&cli.namespace).await?;
        }
        Commands::VractApprove { name } => {
            handlers::crds::handle_vract_approve(&cli.namespace, &name).await?;
        }
    }

    Ok(())
}
