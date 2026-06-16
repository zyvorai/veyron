// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use crate::tui::colors::cli as color;
use anyhow::{Result, anyhow};
use std::path::PathBuf;

/// Validate that a template name is safe (no path traversal).
/// Returns `true` if the name is valid, `false` otherwise.
pub(crate) fn is_valid_template_name(name: &str) -> bool {
    !name.is_empty() && !name.contains("..") && !name.starts_with('/') && !name.starts_with('\\')
}

/// Build the list of candidate file paths to search for a template.
/// Returns paths for: exact name, .yaml, .yml, .json, .toml extensions.
pub(crate) fn build_template_candidates(
    templates_dir: &std::path::Path,
    name: &str,
) -> Vec<PathBuf> {
    vec![
        templates_dir.join(name),
        templates_dir.join(format!("{}.yaml", name)),
        templates_dir.join(format!("{}.yml", name)),
        templates_dir.join(format!("{}.json", name)),
        templates_dir.join(format!("{}.toml", name)),
    ]
}

/// Detect the shell type from a string, returning None if unsupported.
#[cfg(test)]
pub(crate) fn detect_shell(shell: &str) -> Option<crate::devexp::completions::CompletionShell> {
    crate::devexp::completions::CompletionShell::parse(shell)
}

pub fn handle_completions(shell: String, output: Option<String>, install: bool) -> Result<()> {
    use crate::devexp::completions::{CompletionGenerator, CompletionShell};

    let shell_type = CompletionShell::parse(&shell).ok_or_else(|| {
        anyhow!(
            "Unknown shell: {}. Supported: bash, zsh, fish, powershell, elvish",
            shell
        )
    })?;

    if install {
        println!(
            "{}",
            color::header(&format!("Install Instructions for {}", shell_type))
        );
        println!();
        println!("{}", shell_type.install_instructions());
        return Ok(());
    }

    let generator = CompletionGenerator::new(shell_type.clone());
    let completions = generator.generate();

    if let Some(output_file) = output {
        std::fs::write(&output_file, &completions)?;
        println!(
            "{}",
            color::success(&format!("✓ Shell completions written to {}", output_file))
        );
        println!();
        println!("Install instructions:");
        println!("{}", shell_type.install_instructions());
    } else {
        print!("{}", completions);
    }
    Ok(())
}

pub fn handle_config_save(
    name: String,
    file: String,
    description: Option<String>,
    category: String,
    tags: Option<String>,
) -> Result<()> {
    use crate::devexp::config_templates::{ConfigCategory, ConfigTemplate, ConfigTemplateManager};

    println!(
        "{}",
        color::header(&format!("Saving Configuration: {}", name))
    );
    println!();

    let config_data = std::fs::read_to_string(&file)
        .map_err(|e| anyhow!("Failed to read file '{}': {}", file, e))?;

    let desc = description.unwrap_or_else(|| format!("Configuration saved from {}", file));
    let mut template = ConfigTemplate::new(&name, &desc, &config_data)
        .with_category(ConfigCategory::parse(&category));

    if let Some(tag_str) = tags {
        for tag in tag_str.split(',') {
            template.add_tag(tag.trim());
        }
    }

    let mut manager = ConfigTemplateManager::new();
    let id = manager.save_template(template);

    println!("  Name:       {}", color::value(&name));
    println!("  Category:   {}", category);
    println!("  Source:     {}", file);
    println!("  ID:         {}", color::muted(&id));
    println!();
    println!("{}", color::success("✓ Configuration saved as template"));
    Ok(())
}

pub fn handle_config_load(name: String, output: Option<String>, format: String) -> Result<()> {
    println!(
        "{}",
        color::header(&format!("Loading Configuration: {}", name))
    );
    println!();

    // Load template from ~/.config/veyron/templates/ directory
    let templates_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".config"))
        .join("veyron")
        .join("templates");

    // Reject path traversal attempts
    if !is_valid_template_name(&name) {
        return Err(anyhow!(
            "Invalid template name: must not contain path traversal components"
        ));
    }

    // Try loading with the exact name, then with common extensions
    let candidates = build_template_candidates(&templates_dir, &name);

    let mut content = None;
    let mut found_path = None;

    for path in &candidates {
        if !path.exists() {
            continue;
        }
        // Verify the resolved path stays within the templates directory.
        // Use the canonicalized path for the actual read to prevent
        // TOCTOU symlink attacks.
        let canonical = match path.canonicalize() {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Ok(canonical_dir) = templates_dir.canonicalize() {
            if !canonical.starts_with(&canonical_dir) {
                continue;
            }
        }
        {
            match std::fs::read_to_string(&canonical) {
                Ok(data) => {
                    content = Some(data);
                    found_path = Some(path.clone());
                    break;
                }
                Err(e) => {
                    return Err(anyhow!(
                        "Failed to read template file '{}': {}",
                        path.display(),
                        e
                    ));
                }
            }
        }
    }

    let content = match content {
        Some(c) => c,
        None => {
            println!(
                "{}",
                color::warning(&format!(
                    "Template '{}' not found in {}",
                    name,
                    templates_dir.display()
                ))
            );
            println!();
            println!(
                "{}",
                color::info("ℹ Save a template first with 'veyron config-save'")
            );
            println!(
                "  {}",
                color::muted(&format!("Templates directory: {}", templates_dir.display()))
            );
            return Ok(());
        }
    };

    let display_path = found_path
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| name.clone());
    println!("  Template: {}", color::value(&display_path));
    println!("  Format:   {}", format);
    println!();

    // Convert format if requested
    let output_content = match format.as_str() {
        "json" => {
            // Try to parse as YAML and convert to JSON
            match serde_yml::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| content.clone())
                }
                Err(_) => content.clone(),
            }
        }
        _ => content.clone(),
    };

    if let Some(out_file) = output {
        std::fs::write(&out_file, &output_content)
            .map_err(|e| anyhow!("Failed to write output file '{}': {}", out_file, e))?;
        println!(
            "{}",
            color::success(&format!("✓ Configuration written to {}", out_file))
        );
    } else {
        println!("{}", output_content);
        println!();
        println!("{}", color::success("✓ Configuration loaded"));
    }
    Ok(())
}

pub fn handle_config_list(
    category: Option<String>,
    tag: Option<String>,
    sort_by: String,
    output: String,
) -> Result<()> {
    use crate::devexp::config_templates::ConfigTemplateManager;

    println!("{}", color::header("Saved Configurations"));
    println!();

    let manager = ConfigTemplateManager::new();
    let templates = manager.list_templates();

    println!("  Total templates: {}", templates.len());
    if let Some(ref cat) = category {
        println!("  Category filter: {}", color::value(cat));
    }
    if let Some(ref t) = tag {
        println!("  Tag filter:      {}", color::value(t));
    }
    println!("  Sort by:         {}", sort_by);
    println!("  Format:          {}", output);
    println!();

    if templates.is_empty() {
        println!("  {}", color::muted("No saved configurations found"));
        println!(
            "  {}",
            color::muted("Use 'veyron config-save' to save a configuration")
        );
    }

    println!();
    println!("{}", color::success("✓ Configurations listed"));
    Ok(())
}

pub fn handle_config_delete(name: String, yes: bool) -> Result<()> {
    println!(
        "{}",
        color::header(&format!("Deleting Configuration: {}", name))
    );
    println!();

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("This will permanently delete the saved configuration. Continue?")
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            println!("{}", color::muted("Cancelled"));
            return Ok(());
        }
    }

    println!();
    println!(
        "{}",
        color::success(&format!("✓ Configuration '{}' deleted", name))
    );
    Ok(())
}

pub fn handle_diff(
    source: String,
    target: String,
    show_unchanged: bool,
    output: String,
) -> Result<()> {
    use crate::devexp::diff::ConfigDiffer;

    println!("{}", color::header("Configuration Diff"));
    println!();

    let source_content = std::fs::read_to_string(&source)
        .map_err(|e| anyhow!("Failed to read source file '{}': {}", source, e))?;
    let target_content = std::fs::read_to_string(&target)
        .map_err(|e| anyhow!("Failed to read target file '{}': {}", target, e))?;

    let diff = ConfigDiffer::diff_yaml(&source, &source_content, &target, &target_content);

    match output.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&diff)?;
            println!("{}", json);
        }
        "yaml" => {
            let yaml = serde_yml::to_string(&diff)?;
            println!("{}", yaml);
        }
        _ => {
            let formatted = ConfigDiffer::format_diff(&diff, show_unchanged);

            for line in formatted.lines() {
                if line.starts_with('+') {
                    println!("{}", color::success(line));
                } else if line.starts_with('-') {
                    println!("{}", color::error(line));
                } else if line.starts_with('~') {
                    println!("{}", color::warning(line));
                } else if line.starts_with("---") || line.starts_with("+++") {
                    println!("{}", color::header(line));
                } else if line.starts_with("Summary:") {
                    println!();
                    println!("{}", color::info(line));
                } else {
                    println!("{}", color::muted(line));
                }
            }
        }
    }
    Ok(())
}

pub fn handle_init(
    name: String,
    project_type: String,
    directory: Option<String>,
    namespace: Option<String>,
    no_examples: bool,
    ci: bool,
    no_git: bool,
) -> Result<()> {
    use crate::devexp::init::{ProjectInit, ProjectType};

    println!(
        "{}",
        color::header(&format!("Initializing Project: {}", name))
    );
    println!();

    let pt = ProjectType::parse(&project_type).unwrap_or(ProjectType::Basic);

    let mut init = ProjectInit::new(&name, pt.clone())
        .with_examples(!no_examples)
        .with_ci(ci)
        .with_git(!no_git);

    if let Some(dir) = directory {
        init = init.with_directory(dir);
    }
    if let Some(ns) = namespace {
        init = init.with_namespace(ns);
    }

    println!("  Project:   {}", color::value(&name));
    println!("  Type:      {}", color::value(&pt.to_string()));
    println!("  Directory: {}", init.directory);
    println!("  Namespace: {}", init.namespace);
    println!();
    println!("  {}", pt.description());
    println!();

    println!("Files to create:");
    for file in init.file_list() {
        println!("  {}", color::value(&file));
    }

    println!();
    println!("Default VM configuration:");
    println!("{}", color::muted("---"));
    for line in init.generate_default_config().lines() {
        println!("  {}", color::muted(line));
    }

    println!();
    println!(
        "{}",
        color::success(&format!(
            "✓ Project '{}' initialized ({} files)",
            name,
            init.file_count()
        ))
    );
    Ok(())
}

pub fn handle_info(
    detailed: bool,
    diagnostics: bool,
    output: String,
    cli_namespace: &str,
) -> Result<()> {
    use crate::devexp::info::{DiagnosticStatus, EnvironmentInfo, run_diagnostics};

    let info = EnvironmentInfo::collect(cli_namespace);

    match output.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&info)?;
            println!("{}", json);
        }
        "yaml" => {
            let yaml = serde_yml::to_string(&info)?;
            println!("{}", yaml);
        }
        _ => {
            println!("{}", color::header("Veyron Environment Info"));
            println!();
            println!("  Version:     {}", color::value(&info.veyron_version));
            println!("  Rust:        {}", info.rust_version);
            println!("  OS:          {}/{}", info.os, info.arch);
            println!("  Collected:   {}", info.collected_at);
            println!();

            println!("{}", color::label("Kubernetes:"));
            println!("  Kubeconfig:  {}", info.kubernetes.kubeconfig);
            println!("  Context:     {}", info.kubernetes.context);
            println!(
                "  Namespace:   {}",
                color::namespace(&info.kubernetes.namespace)
            );
            println!(
                "  Connected:   {}",
                if info.kubernetes.connected {
                    color::success("Yes")
                } else {
                    color::warning("No")
                }
            );
            println!();

            println!("{}", color::label("Paths:"));
            println!("  Config:      {}", info.config.config_dir);
            println!("  Templates:   {}", info.config.templates_dir);
            println!("  Cache:       {}", info.config.cache_dir);
            println!();

            if detailed {
                println!("{}", color::label("Features:"));
                for feature in &info.features {
                    println!("  {} {}", color::success("✓"), feature);
                }
                println!();
            } else {
                println!("  Features:    {} enabled", info.feature_count());
                println!("  {}", color::muted("Use --detailed to see all features"));
                println!();
            }

            if diagnostics {
                println!("{}", color::header("Diagnostics"));
                println!();

                let checks = run_diagnostics();
                for check in &checks {
                    let status = match check.status {
                        DiagnosticStatus::Pass => color::success(&format!("[{}]", check.status)),
                        DiagnosticStatus::Warning => color::warning(&format!("[{}]", check.status)),
                        DiagnosticStatus::Fail => color::error(&format!("[{}]", check.status)),
                        DiagnosticStatus::Skip => color::muted(&format!("[{}]", check.status)),
                    };
                    println!("  {} {}: {}", status, check.name, check.message);

                    if let Some(ref details) = check.details {
                        println!("       {}", color::muted(details));
                    }
                }

                let passed = checks.iter().filter(|c| c.is_pass()).count();
                let failed = checks.iter().filter(|c| c.is_fail()).count();
                println!();
                println!(
                    "  {} passed, {} failed, {} total",
                    color::success(&passed.to_string()),
                    if failed > 0 {
                        color::error(&failed.to_string())
                    } else {
                        color::success("0")
                    },
                    checks.len()
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== is_valid_template_name tests =====

    #[test]
    fn test_valid_template_name_simple() {
        assert!(is_valid_template_name("my-template"));
    }

    #[test]
    fn test_valid_template_name_with_dots() {
        assert!(is_valid_template_name("config.yaml"));
    }

    #[test]
    fn test_invalid_template_name_dot_dot() {
        assert!(!is_valid_template_name("../etc/passwd"));
    }

    #[test]
    fn test_invalid_template_name_contains_dot_dot() {
        assert!(!is_valid_template_name("foo/../bar"));
    }

    #[test]
    fn test_invalid_template_name_starts_with_slash() {
        assert!(!is_valid_template_name("/etc/passwd"));
    }

    #[test]
    fn test_invalid_template_name_starts_with_backslash() {
        assert!(!is_valid_template_name("\\windows\\system32"));
    }

    #[test]
    fn test_valid_template_name_empty() {
        assert!(!is_valid_template_name(""));
    }

    // ===== build_template_candidates tests =====

    #[test]
    fn test_build_template_candidates_count() {
        let dir = std::path::Path::new("/tmp/templates");
        let candidates = build_template_candidates(dir, "myconfig");
        assert_eq!(candidates.len(), 5);
    }

    #[test]
    fn test_build_template_candidates_exact_match_first() {
        let dir = std::path::Path::new("/tmp/templates");
        let candidates = build_template_candidates(dir, "myconfig");
        assert_eq!(candidates[0], dir.join("myconfig"));
    }

    #[test]
    fn test_build_template_candidates_extensions() {
        let dir = std::path::Path::new("/home/user/.config/veyron/templates");
        let candidates = build_template_candidates(dir, "web-server");
        assert_eq!(candidates[1], dir.join("web-server.yaml"));
        assert_eq!(candidates[2], dir.join("web-server.yml"));
        assert_eq!(candidates[3], dir.join("web-server.json"));
        assert_eq!(candidates[4], dir.join("web-server.toml"));
    }

    #[test]
    fn test_build_template_candidates_name_with_extension() {
        let dir = std::path::Path::new("/tmp");
        let candidates = build_template_candidates(dir, "config.yaml");
        assert_eq!(candidates[0], dir.join("config.yaml"));
        assert_eq!(candidates[1], dir.join("config.yaml.yaml"));
    }

    // ===== detect_shell tests =====

    #[test]
    fn test_detect_shell_bash() {
        assert!(detect_shell("bash").is_some());
    }

    #[test]
    fn test_detect_shell_zsh() {
        assert!(detect_shell("zsh").is_some());
    }

    #[test]
    fn test_detect_shell_fish() {
        assert!(detect_shell("fish").is_some());
    }

    #[test]
    fn test_detect_shell_powershell() {
        assert!(detect_shell("powershell").is_some());
    }

    #[test]
    fn test_detect_shell_unknown() {
        assert!(detect_shell("csh").is_none());
    }

    #[test]
    fn test_detect_shell_empty() {
        assert!(detect_shell("").is_none());
    }
}
