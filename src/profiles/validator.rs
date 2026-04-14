// Profile Validation Logic

use super::Profile;
use anyhow::{Result, anyhow};
use regex::Regex;
use std::sync::LazyLock;

// Compiled regex for validation
static NAME_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]([a-z0-9-]*[a-z0-9])?$").expect("invalid name regex"));

static MEMORY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d+([KMGT]i|[KMGT])$").expect("invalid memory regex"));

static DISK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d+([KMGT]i|[KMGT])$").expect("invalid disk regex"));

/// Reserved profile names that cannot be used for custom profiles
const RESERVED_NAMES: &[&str] = &[
    "dev",
    "test",
    "prod",
    "high-perf",
    "microservice",
    "database",
    "web",
    "minimal",
];

/// Validate a profile
pub fn validate_profile(profile: &Profile) -> Result<()> {
    // Validate name format
    validate_name(&profile.name)?;

    // Validate CPU values
    validate_cpu(profile.cpu_cores, profile.cpu_sockets, profile.cpu_threads)?;

    // Validate memory format
    validate_memory(&profile.memory)?;

    // Validate disk size format
    validate_disk_size(&profile.disk_size)?;

    // Validate description is not empty
    if profile.description.trim().is_empty() {
        return Err(anyhow!("Profile description cannot be empty"));
    }

    Ok(())
}

/// Validate profile name format
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("Profile name cannot be empty"));
    }

    if name.len() > 63 {
        return Err(anyhow!("Profile name cannot exceed 63 characters"));
    }

    if !NAME_PATTERN.is_match(name) {
        return Err(anyhow!(
            "Profile name must be lowercase alphanumeric with hyphens, \
            starting and ending with alphanumeric character"
        ));
    }

    if RESERVED_NAMES.contains(&name) {
        return Err(anyhow!(
            "Cannot use reserved builtin profile name '{}'. \
            Reserved names: {}",
            name,
            RESERVED_NAMES.join(", ")
        ));
    }

    Ok(())
}

/// Validate CPU configuration
pub fn validate_cpu(cores: u32, sockets: u32, threads: u32) -> Result<()> {
    if cores == 0 {
        return Err(anyhow!("CPU cores must be greater than 0"));
    }

    if sockets == 0 {
        return Err(anyhow!("CPU sockets must be greater than 0"));
    }

    if threads == 0 {
        return Err(anyhow!("CPU threads must be greater than 0"));
    }

    // Reasonable limits
    if cores > 128 {
        return Err(anyhow!("CPU cores cannot exceed 128"));
    }

    if sockets > 4 {
        return Err(anyhow!("CPU sockets cannot exceed 4"));
    }

    if threads > 2 {
        return Err(anyhow!("CPU threads cannot exceed 2"));
    }

    Ok(())
}

/// Validate memory format (e.g., "4Gi", "512Mi", "16G")
pub fn validate_memory(memory: &str) -> Result<()> {
    if memory.is_empty() {
        return Err(anyhow!("Memory value cannot be empty"));
    }

    if !MEMORY_PATTERN.is_match(memory) {
        return Err(anyhow!(
            "Invalid memory format '{}'. \
            Expected format: number followed by unit (K/M/G/T or Ki/Mi/Gi/Ti). \
            Examples: 4Gi, 512Mi, 16G",
            memory
        ));
    }

    Ok(())
}

/// Validate disk size format (e.g., "10Gi", "500G")
pub fn validate_disk_size(disk_size: &str) -> Result<()> {
    if disk_size.is_empty() {
        return Err(anyhow!("Disk size cannot be empty"));
    }

    if !DISK_PATTERN.is_match(disk_size) {
        return Err(anyhow!(
            "Invalid disk size format '{}'. \
            Expected format: number followed by unit (K/M/G/T or Ki/Mi/Gi/Ti). \
            Examples: 10Gi, 500G",
            disk_size
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name() {
        // Valid names
        assert!(validate_name("my-profile").is_ok());
        assert!(validate_name("profile123").is_ok());
        assert!(validate_name("a").is_ok());
        assert!(validate_name("my-custom-profile-2").is_ok());

        // Invalid names
        assert!(validate_name("").is_err());
        assert!(validate_name("My-Profile").is_err()); // uppercase
        assert!(validate_name("-profile").is_err()); // starts with hyphen
        assert!(validate_name("profile-").is_err()); // ends with hyphen
        assert!(validate_name("profile_name").is_err()); // underscore
        assert!(validate_name("dev").is_err()); // reserved
        assert!(validate_name("prod").is_err()); // reserved
    }

    #[test]
    fn test_validate_cpu() {
        // Valid
        assert!(validate_cpu(1, 1, 1).is_ok());
        assert!(validate_cpu(4, 2, 1).is_ok());
        assert!(validate_cpu(128, 4, 2).is_ok());

        // Invalid
        assert!(validate_cpu(0, 1, 1).is_err()); // zero cores
        assert!(validate_cpu(1, 0, 1).is_err()); // zero sockets
        assert!(validate_cpu(1, 1, 0).is_err()); // zero threads
        assert!(validate_cpu(129, 1, 1).is_err()); // too many cores
        assert!(validate_cpu(1, 5, 1).is_err()); // too many sockets
        assert!(validate_cpu(1, 1, 3).is_err()); // too many threads
    }

    #[test]
    fn test_validate_memory() {
        // Valid
        assert!(validate_memory("4Gi").is_ok());
        assert!(validate_memory("512Mi").is_ok());
        assert!(validate_memory("16G").is_ok());
        assert!(validate_memory("1Ti").is_ok());
        assert!(validate_memory("100M").is_ok());

        // Invalid
        assert!(validate_memory("").is_err());
        assert!(validate_memory("4GB").is_err()); // wrong unit
        assert!(validate_memory("4gi").is_err()); // lowercase
        assert!(validate_memory("Gi").is_err()); // no number
        assert!(validate_memory("4").is_err()); // no unit
    }

    #[test]
    fn test_validate_disk_size() {
        // Valid
        assert!(validate_disk_size("10Gi").is_ok());
        assert!(validate_disk_size("500G").is_ok());
        assert!(validate_disk_size("1Ti").is_ok());

        // Invalid
        assert!(validate_disk_size("").is_err());
        assert!(validate_disk_size("10GB").is_err());
        assert!(validate_disk_size("10").is_err());
    }

    #[test]
    fn test_validate_profile() {
        let valid_profile = Profile {
            name: "my-profile".to_string(),
            description: "Test profile".to_string(),
            cpu_cores: 4,
            cpu_sockets: 1,
            cpu_threads: 1,
            memory: "8Gi".to_string(),
            disk_size: "40Gi".to_string(),
            use_cases: vec![],
            recommended_os: vec![],
        };

        assert!(validate_profile(&valid_profile).is_ok());

        // Invalid profile
        let mut invalid = valid_profile.clone();
        invalid.name = "dev".to_string(); // reserved
        assert!(validate_profile(&invalid).is_err());

        invalid = valid_profile.clone();
        invalid.cpu_cores = 0;
        assert!(validate_profile(&invalid).is_err());

        invalid = valid_profile.clone();
        invalid.memory = "invalid".to_string();
        assert!(validate_profile(&invalid).is_err());
    }
}
