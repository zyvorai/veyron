// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use crate::tui::colors::cli as color;
use anyhow::{Result, anyhow};

pub fn handle_profiles(details: bool) -> Result<()> {
    use crate::profiles::PROFILES;

    let manager = PROFILES
        .read()
        .map_err(|e| anyhow::anyhow!("Failed to lock profiles: {}", e))?;

    println!("{}", color::header("═══ VM Resource Profiles ═══"));
    println!();

    for profile in manager.list() {
        let prefix = if manager.is_builtin(&profile.name) {
            color::value("•")
        } else {
            color::success("★") // Custom profiles with star
        };

        println!("{} {}", prefix, color::header(&profile.name));
        println!("  {}", color::muted(&profile.description));

        if details {
            println!(
                "  CPU:    {} cores ({} sockets, {} threads)",
                color::resource(&profile.cpu_cores.to_string(), "cpu"),
                profile.cpu_sockets,
                profile.cpu_threads
            );
            println!("  Memory: {}", color::resource(&profile.memory, "memory"));
            println!("  Disk:   {}", color::resource(&profile.disk_size, "disk"));
            println!("  Use cases: {}", profile.use_cases.join(", "));
            println!("  Recommended OS: {}", profile.recommended_os.join(", "));
            println!(
                "  Type: {}",
                if manager.is_builtin(&profile.name) {
                    color::muted("builtin")
                } else {
                    color::success("custom")
                }
            );
        }
        println!();
    }

    println!(
        "{}",
        color::muted("★ = custom profile, • = builtin profile")
    );
    println!(
        "{}",
        color::muted("Use 'vmrogue profile <name>' for details")
    );
    println!(
        "{}",
        color::muted(
            "Create custom: vmrogue profile-create <name> --cpus <n> --memory <size> --disk-size <size>"
        )
    );
    Ok(())
}

pub fn handle_profile(name: String, output: String) -> Result<()> {
    use crate::profiles::PROFILES;

    let manager = PROFILES
        .read()
        .map_err(|e| anyhow::anyhow!("Failed to lock profiles: {}", e))?;
    let profile = manager
        .get(&name)
        .ok_or_else(|| anyhow!("Profile not found: {}", name))?;

    match output.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&profile)?;
            println!("{}", json);
        }
        _ => {
            let yaml = serde_yml::to_string(&profile)?;
            println!("{}", yaml);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_profile_create(
    name: String,
    cpus: u32,
    sockets: u32,
    threads: u32,
    memory: String,
    disk_size: String,
    description: Option<String>,
    use_cases: Option<String>,
    recommended_os: Option<String>,
    from_file: Option<String>,
) -> Result<()> {
    use crate::profiles::{PROFILES, Profile};
    use anyhow::Context;

    let profile = if let Some(file_path) = from_file {
        // Load from file
        let content = std::fs::read_to_string(file_path).context("Failed to read profile file")?;
        serde_yml::from_str::<Profile>(&content).context("Failed to parse profile YAML")?
    } else {
        // Build from arguments
        Profile {
            name: name.clone(),
            description: description
                .clone()
                .unwrap_or_else(|| format!("Custom profile: {}", name)),
            cpu_cores: cpus,
            cpu_sockets: sockets,
            cpu_threads: threads,
            memory: memory.clone(),
            disk_size: disk_size.clone(),
            use_cases: use_cases
                .as_ref()
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default(),
            recommended_os: recommended_os
                .as_ref()
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default(),
        }
    };

    // Create the profile
    let mut manager = PROFILES
        .write()
        .map_err(|e| anyhow::anyhow!("Failed to lock profiles: {}", e))?;
    manager.create_custom(profile)?;

    println!(
        "{}",
        color::success(&format!("✓ Custom profile '{}' created successfully", name))
    );
    println!(
        "{}",
        color::muted(&format!(
            "  Location: ~/.config/vmrogue/profiles/{}.yaml",
            name
        ))
    );
    println!(
        "{}",
        color::muted(&format!(
            "  Use with: vmrogue create <vm-name> --template <os> --profile {}",
            name
        ))
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_profile_edit(
    name: String,
    cpus: Option<u32>,
    sockets: Option<u32>,
    threads: Option<u32>,
    memory: Option<String>,
    disk_size: Option<String>,
    description: Option<String>,
    use_cases: Option<String>,
    recommended_os: Option<String>,
) -> Result<()> {
    use crate::profiles::PROFILES;

    let mut manager = PROFILES
        .write()
        .map_err(|e| anyhow::anyhow!("Failed to lock profiles: {}", e))?;

    // Load existing profile
    let mut profile = manager
        .get(&name)
        .ok_or_else(|| anyhow!("Profile '{}' not found", name))?;

    // Check if builtin
    if manager.is_builtin(&name) {
        anyhow::bail!(
            "Cannot edit builtin profile '{}'. Create a custom profile instead.",
            name
        );
    }

    // Apply updates
    if let Some(c) = cpus {
        profile.cpu_cores = c;
    }
    if let Some(s) = sockets {
        profile.cpu_sockets = s;
    }
    if let Some(t) = threads {
        profile.cpu_threads = t;
    }
    if let Some(m) = memory {
        profile.memory = m.clone();
    }
    if let Some(d) = disk_size {
        profile.disk_size = d.clone();
    }
    if let Some(desc) = description {
        profile.description = desc.clone();
    }
    if let Some(uc) = use_cases {
        profile.use_cases = uc.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Some(ros) = recommended_os {
        profile.recommended_os = ros.split(',').map(|s| s.trim().to_string()).collect();
    }

    // Update the profile
    manager.update_custom(profile)?;

    println!(
        "{}",
        color::success(&format!("✓ Profile '{}' updated successfully", name))
    );
    Ok(())
}

pub fn handle_profile_delete(name: String, yes: bool) -> Result<()> {
    use crate::profiles::PROFILES;

    let mut manager = PROFILES
        .write()
        .map_err(|e| anyhow::anyhow!("Failed to lock profiles: {}", e))?;

    // Check if builtin
    if manager.is_builtin(&name) {
        anyhow::bail!("Cannot delete builtin profile '{}'", name);
    }

    // Check if exists
    if !manager.exists(&name) {
        anyhow::bail!("Profile '{}' not found", name);
    }

    // Confirm deletion unless --yes flag
    if !yes {
        use dialoguer::Confirm;

        let confirmed = Confirm::new()
            .with_prompt(format!("Delete custom profile '{}'?", name))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", color::muted("Deletion cancelled"));
            return Ok(());
        }
    }

    // Delete the profile
    manager.delete_custom(&name)?;

    println!(
        "{}",
        color::success(&format!("✓ Profile '{}' deleted", name))
    );
    Ok(())
}

pub fn handle_blueprints(tag: Option<String>, details: bool) -> Result<()> {
    use crate::blueprints::BLUEPRINTS;
    use crate::tui::colors::cli;

    let manager = BLUEPRINTS
        .read()
        .map_err(|e| anyhow::anyhow!("Failed to lock blueprints: {}", e))?;

    println!("{}", color::header("═══ Multi-VM Blueprints ═══"));
    println!();

    let blueprints = if let Some(tag_filter) = tag {
        manager.search_by_tag(&tag_filter)
    } else {
        manager.list()
    };

    for blueprint in blueprints {
        let prefix = if manager.is_builtin(&blueprint.name) {
            color::value("•")
        } else {
            color::success("★") // Custom blueprints with star
        };

        println!("{} {}", prefix, color::header(&blueprint.name));
        println!("  {}", color::muted(&blueprint.description));
        println!("  VMs: {}", color::value(&blueprint.vms.len().to_string()));
        if details {
            for vm in &blueprint.vms {
                println!(
                    "    {} {} (template: {})",
                    color::value("-"),
                    cli::vm_name(&vm.name),
                    vm.template
                );
            }
            println!("  Tags: {}", blueprint.tags.join(", "));
            println!(
                "  Type: {}",
                if manager.is_builtin(&blueprint.name) {
                    color::muted("builtin")
                } else {
                    color::success("custom")
                }
            );
        }
        println!();
    }

    println!(
        "{}",
        color::muted("★ = custom blueprint, • = builtin blueprint")
    );
    println!(
        "{}",
        color::muted("Use 'vmrogue blueprint <name>' for details")
    );
    println!(
        "{}",
        color::muted("Create custom: vmrogue blueprint-create <name> --from-file <file>")
    );
    Ok(())
}

pub fn handle_blueprint(name: String, output: String) -> Result<()> {
    use crate::blueprints::BLUEPRINTS;

    let manager = BLUEPRINTS
        .read()
        .map_err(|e| anyhow::anyhow!("Failed to lock blueprints: {}", e))?;
    let blueprint = manager
        .get(&name)
        .ok_or_else(|| anyhow!("Blueprint not found: {}", name))?;

    match output.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&blueprint)?;
            println!("{}", json);
        }
        _ => {
            let yaml = serde_yml::to_string(&blueprint)?;
            println!("{}", yaml);
        }
    }
    Ok(())
}

pub async fn handle_deploy(
    blueprint: String,
    prefix: Option<String>,
    start: bool,
    dry_run: bool,
    namespace: String,
) -> Result<()> {
    use crate::blueprints::BLUEPRINTS;
    use crate::kube;
    use crate::profiles::PROFILES;
    use crate::templates::TEMPLATES;
    use crate::tui::colors::cli;

    // Clone blueprint data and drop lock before any await points
    let bp = {
        let manager = BLUEPRINTS
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to lock blueprints: {}", e))?;
        manager
            .get(&blueprint)
            .ok_or_else(|| anyhow!("Blueprint not found: {}", blueprint))?
            .clone()
    };

    let vm_prefix = prefix.unwrap_or_else(|| blueprint.clone());

    println!(
        "{}",
        color::info(&format!("Deploying blueprint: {}", blueprint))
    );
    println!("  Description: {}", bp.description);
    println!("  VMs to create: {}", bp.vms.len());
    println!();

    if dry_run {
        println!("{}", color::info("Dry run - VMs that would be created:"));
        for (i, vm_spec) in bp.vms.iter().enumerate() {
            let vm_name = format!("{}-{}", vm_prefix, vm_spec.name);
            println!("  {}. {}", i + 1, cli::vm_name(&vm_name));
            println!("     Template: {}", vm_spec.template);
            if let Some(profile_name) = &vm_spec.profile {
                let profiles = PROFILES
                    .read()
                    .map_err(|e| anyhow::anyhow!("Failed to lock profiles: {}", e))?;
                if let Some(profile) = profiles.get(profile_name) {
                    println!(
                        "     Profile: {} ({}, {})",
                        profile_name,
                        color::resource(&profile.cpu_cores.to_string(), "cpu"),
                        color::resource(&profile.memory, "memory")
                    );
                }
            }
            if !vm_spec.depends_on.is_empty() {
                println!("     Depends on: {}", vm_spec.depends_on.join(", "));
            }
            println!();
        }
        return Ok(());
    }

    // Create VMs in dependency order
    let client = kube::KubeClient::new().await?;

    for vm_spec in &bp.vms {
        let vm_name = format!("{}-{}", vm_prefix, vm_spec.name);

        println!("{}", color::info(&format!("Creating VM: {}", vm_name)));

        // Get base config from template
        let mut config = TEMPLATES
            .get(&vm_spec.template)
            .ok_or_else(|| anyhow!("Template not found: {}", vm_spec.template))?;

        config.name = vm_name.clone();
        config.namespace = namespace.clone();

        // Apply profile if specified (scope lock to avoid holding across await)
        if let Some(profile_name) = &vm_spec.profile {
            let profiles = PROFILES
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to lock profiles: {}", e))?;
            if let Some(profile) = profiles.get(profile_name) {
                config.cpu.cores = profile.cpu_cores;
                config.cpu.sockets = profile.cpu_sockets;
                config.cpu.threads = profile.cpu_threads;
                config.memory.size = profile.memory.clone();
            }
            drop(profiles);
        }

        // Apply custom overrides
        if let Some(cpu) = vm_spec.cpu {
            config.cpu.cores = cpu;
        }
        if let Some(ref memory) = vm_spec.memory {
            config.memory.size = memory.clone();
        }
        if let Some(ref disk_size) = vm_spec.disk_size {
            if let Some(disk) = config.disks.first_mut() {
                disk.size = disk_size.clone();
            }
        }

        // Apply labels
        for (k, v) in &vm_spec.labels {
            config.labels.insert(k.clone(), v.clone());
        }

        // Create the VM
        match client.create_vm(&config).await {
            Ok(_) => {
                println!(
                    "{}",
                    cli::success(&format!("VM '{}' created successfully", vm_name))
                );
            }
            Err(e) => {
                println!(
                    "{}",
                    cli::error(&format!("Failed to create VM '{}': {}", vm_name, e))
                );
                continue; // Skip start attempt for failed VM
            }
        }

        // Start if requested
        if start {
            match client.start_vm(&namespace, &vm_name).await {
                Ok(_) => {
                    println!("{}", cli::success(&format!("VM '{}' started", vm_name)));
                }
                Err(e) => {
                    println!(
                        "{}",
                        cli::error(&format!("Failed to start VM '{}': {}", vm_name, e))
                    );
                }
            }
        }

        println!();
    }

    println!("{}", cli::success("Blueprint deployment complete!"));
    Ok(())
}

pub fn handle_blueprint_create(
    name: String,
    from_file: String,
    description: Option<String>,
) -> Result<()> {
    use crate::blueprints::{BLUEPRINTS, Blueprint};
    use anyhow::Context;

    // Load blueprint from file
    let content = std::fs::read_to_string(from_file).context("Failed to read blueprint file")?;
    let mut blueprint: Blueprint =
        serde_yml::from_str(&content).context("Failed to parse blueprint YAML")?;

    // Override name if provided
    blueprint.name = name.clone();

    // Override description if provided
    if let Some(desc) = description {
        blueprint.description = desc.clone();
    }

    // Create the blueprint
    let mut manager = BLUEPRINTS
        .write()
        .map_err(|e| anyhow::anyhow!("Failed to lock blueprints: {}", e))?;
    manager.create_custom(blueprint)?;

    println!(
        "{}",
        color::success(&format!(
            "✓ Custom blueprint '{}' created successfully",
            name
        ))
    );
    println!(
        "{}",
        color::muted(&format!(
            "  Location: ~/.config/vmrogue/blueprints/{}.yaml",
            name
        ))
    );
    println!(
        "{}",
        color::muted(&format!("  Deploy with: vmrogue deploy {}", name))
    );
    Ok(())
}

pub fn handle_blueprint_edit(name: String, description: Option<String>) -> Result<()> {
    use crate::blueprints::BLUEPRINTS;

    let mut manager = BLUEPRINTS
        .write()
        .map_err(|e| anyhow::anyhow!("Failed to lock blueprints: {}", e))?;

    // Load existing blueprint
    let mut blueprint = manager
        .get(&name)
        .ok_or_else(|| anyhow!("Blueprint '{}' not found", name))?;

    // Check if builtin
    if manager.is_builtin(&name) {
        anyhow::bail!(
            "Cannot edit builtin blueprint '{}'. Create a custom blueprint instead.",
            name
        );
    }

    // Apply updates
    if let Some(desc) = description {
        blueprint.description = desc.clone();
    }

    // Update the blueprint
    manager.update_custom(blueprint)?;

    println!(
        "{}",
        color::success(&format!("✓ Blueprint '{}' updated successfully", name))
    );
    Ok(())
}

pub fn handle_blueprint_delete(name: String, yes: bool) -> Result<()> {
    use crate::blueprints::BLUEPRINTS;

    let mut manager = BLUEPRINTS
        .write()
        .map_err(|e| anyhow::anyhow!("Failed to lock blueprints: {}", e))?;

    // Check if builtin
    if manager.is_builtin(&name) {
        anyhow::bail!("Cannot delete builtin blueprint '{}'", name);
    }

    // Check if exists
    if !manager.exists(&name) {
        anyhow::bail!("Blueprint '{}' not found", name);
    }

    // Confirm deletion unless --yes flag
    if !yes {
        use dialoguer::Confirm;

        let confirmed = Confirm::new()
            .with_prompt(format!("Delete custom blueprint '{}'?", name))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", color::muted("Deletion cancelled"));
            return Ok(());
        }
    }

    // Delete the blueprint
    manager.delete_custom(&name)?;

    println!(
        "{}",
        color::success(&format!("✓ Blueprint '{}' deleted", name))
    );
    Ok(())
}

pub fn handle_blueprint_validate(file: String, detailed: bool) -> Result<()> {
    use crate::blueprints::{Blueprint, validator};
    use anyhow::Context;

    // Load blueprint from file
    let content = std::fs::read_to_string(&file).context("Failed to read blueprint file")?;
    let blueprint: Blueprint =
        serde_yml::from_str(&content).context("Failed to parse blueprint YAML")?;

    println!(
        "{}",
        color::header(&format!("═══ Validating Blueprint: {} ═══", blueprint.name))
    );
    println!();

    // Validate the blueprint
    match validator::validate_blueprint(&blueprint) {
        Ok(_) => {
            println!("{}", color::success("✓ Blueprint validation passed"));
            println!();
            println!("  Name: {}", color::value(&blueprint.name));
            println!("  Description: {}", blueprint.description);
            println!("  VMs: {}", blueprint.vms.len());

            if detailed {
                println!();
                println!("{}", color::header("VM Specifications:"));
                for vm in &blueprint.vms {
                    println!("  • {}", color::value(&vm.name));
                    println!("    Template: {}", vm.template);
                    if let Some(profile) = &vm.profile {
                        println!("    Profile: {}", profile);
                    }
                    if !vm.depends_on.is_empty() {
                        println!("    Depends on: {}", vm.depends_on.join(", "));
                    }
                }

                println!();
                println!("{}", color::header("Deployment Order:"));
                match validator::resolve_deployment_order(&blueprint.vms) {
                    Ok(order) => {
                        for (i, vm_name) in order.iter().enumerate() {
                            println!("  {}. {}", i + 1, color::value(vm_name));
                        }
                    }
                    Err(e) => {
                        println!("  {}", color::error(&format!("Error: {}", e)));
                    }
                }
            }
        }
        Err(e) => {
            return Err(anyhow!("Blueprint validation failed: {}", e));
        }
    }
    Ok(())
}

pub async fn handle_health(target: String, detailed: bool, namespace: String) -> Result<()> {
    use crate::config::VMConfigBuilder;
    use crate::health::VMHealthReport;
    use crate::kube;
    use crate::tui::colors::cli;
    use std::fs;

    println!(
        "{}",
        color::header(&format!("═══ Health Check: {} ═══", target))
    );
    println!();

    // Try to load as config file first
    let config = if std::path::Path::new(&target).exists() {
        let content = fs::read_to_string(&target)?;
        if target.ends_with(".json") {
            serde_json::from_str(&content)?
        } else {
            serde_yml::from_str(&content)?
        }
    } else {
        // Try to get running VM
        let client = kube::KubeClient::new().await?;
        let vm = client.get_vm(&namespace, &target).await?;

        // Convert running VM to VMConfig
        let spec = &vm.spec.template.spec;
        let domain = &spec.domain;

        let cpu_cores = domain.cpu.as_ref().and_then(|c| c.cores).unwrap_or(2);
        let cpu_sockets = domain.cpu.as_ref().and_then(|c| c.sockets).unwrap_or(1);
        let cpu_threads = domain.cpu.as_ref().and_then(|c| c.threads).unwrap_or(1);
        let memory = domain
            .memory
            .as_ref()
            .and_then(|m| m.guest.as_deref())
            .or_else(|| {
                domain
                    .resources
                    .requests
                    .as_ref()
                    .and_then(|r| r.get("memory"))
                    .map(|s| s.as_str())
            })
            .unwrap_or("4Gi")
            .to_string();

        let mut builder = VMConfigBuilder::new(&target)
            .namespace(&namespace)
            .cpu(cpu_cores, cpu_sockets, cpu_threads)
            .memory(&memory);

        // Extract disks from volumes
        if let Some(ref volumes) = spec.volumes {
            for (i, vol) in volumes.iter().enumerate() {
                if let Some(ref empty) = vol.empty_disk {
                    builder = builder.add_blank_disk(
                        &vol.name,
                        &empty.capacity,
                        (i as u32).saturating_add(1),
                    );
                } else if let Some(ref container_disk) = vol.container_disk {
                    builder = builder.add_container_disk(
                        &vol.name,
                        &container_disk.image,
                        (i as u32).saturating_add(1),
                    );
                }
            }
        }

        builder.build()
    };

    let mut report = VMHealthReport::new(config.name.clone());

    // Run resource checks
    for check in VMHealthReport::check_resources(
        config.cpu.cores,
        &config.memory.size,
        config
            .disks
            .first()
            .map(|d| d.size.as_str())
            .unwrap_or("20Gi"),
    ) {
        report.add_check(check);
    }

    // Display report
    let status_color = match report.overall_status {
        crate::health::HealthStatus::Healthy => cli::success("✓ HEALTHY"),
        crate::health::HealthStatus::Warning => color::warning("⚠ WARNING"),
        crate::health::HealthStatus::Critical => cli::error("✗ CRITICAL"),
        _ => color::muted("? UNKNOWN"),
    };

    println!("Overall Status: {}", status_color);
    println!("Health Score:   {}/100", report.score);
    println!();

    if detailed || !report.checks.is_empty() {
        println!("{}", color::header("Checks:"));
        for check in &report.checks {
            let status_icon = match check.status {
                crate::health::HealthStatus::Healthy => color::success("✓"),
                crate::health::HealthStatus::Warning => color::warning("⚠"),
                crate::health::HealthStatus::Critical => color::error("✗"),
                _ => color::muted("?"),
            };
            println!(
                "  {} {} - {}",
                status_icon,
                color::label(&check.name),
                check.message
            );
            if let Some(ref rec) = check.recommendation {
                println!("      {}", color::muted(&format!("→ {}", rec)));
            }
        }
        println!();
    }

    if !report.recommendations.is_empty() {
        println!("{}", color::header("Recommendations:"));
        for (i, rec) in report.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, color::value(rec));
        }
    }
    Ok(())
}

pub fn handle_recommend(workload: String, alternatives: bool) -> Result<()> {
    use crate::profiles::PROFILES;
    use crate::tui::colors::cli;

    println!(
        "{}",
        color::header(&format!(
            "═══ Resource Recommendations for: {} ═══",
            workload
        ))
    );
    println!();

    let manager = PROFILES
        .read()
        .map_err(|e| anyhow::anyhow!("Failed to lock profiles: {}", e))?;
    let recommendations = manager.recommend(&workload);

    if recommendations.is_empty() {
        println!(
            "{}",
            color::warning("No specific recommendations found for this workload")
        );
        println!("{}", color::muted("Showing general-purpose profiles:"));
        println!();

        for profile in ["dev", "test", "prod"] {
            if let Some(p) = manager.get(profile) {
                println!("{} {}", color::value("•"), color::header(&p.name));
                println!("  {}", color::muted(&p.description));
                println!(
                    "  CPU: {} cores, Memory: {}, Disk: {}",
                    color::resource(&p.cpu_cores.to_string(), "cpu"),
                    color::resource(&p.memory, "memory"),
                    color::resource(&p.disk_size, "disk")
                );
                println!();
            }
        }
    } else {
        println!(
            "{}",
            cli::success(&format!(
                "Found {} matching profile(s):",
                recommendations.len()
            ))
        );
        println!();

        for (i, profile) in recommendations.iter().enumerate() {
            let marker = if i == 0 {
                cli::success("★")
            } else {
                color::value("•")
            };
            let label = if i == 0 {
                format!("{} (Recommended)", profile.name)
            } else {
                profile.name.clone()
            };

            println!("{} {}", marker, color::header(&label));
            println!("  {}", color::muted(&profile.description));
            println!("  Resources:");
            println!(
                "    CPU:    {} cores ({} sockets × {} threads)",
                color::resource(&profile.cpu_cores.to_string(), "cpu"),
                profile.cpu_sockets,
                profile.cpu_threads
            );
            println!("    Memory: {}", color::resource(&profile.memory, "memory"));
            println!(
                "    Disk:   {}",
                color::resource(&profile.disk_size, "disk")
            );
            println!("  Best for: {}", profile.use_cases.join(", "));
            println!("  Recommended OS: {}", profile.recommended_os.join(", "));
            println!();

            if i == 0 {
                println!("  {}", color::info("Quick create command:"));
                println!(
                    "    {}",
                    color::command(&format!(
                        "vmrogue create my-vm --template {} --profile {}",
                        profile
                            .recommended_os
                            .first()
                            .unwrap_or(&"ubuntu".to_string()),
                        profile.name
                    ))
                );
                println!();
            }
        }
    }

    if alternatives {
        println!("{}", color::header("All Available Profiles:"));
        let manager = PROFILES
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to lock profiles: {}", e))?;
        for profile in manager.list() {
            println!("  {} {}", color::value("•"), color::label(&profile.name));
        }
        println!();
        println!(
            "{}",
            color::muted("Use 'vmrogue profiles' to see all profiles")
        );
    }
    Ok(())
}
