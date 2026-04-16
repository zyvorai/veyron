pub mod batch;
pub mod cron;
pub mod error;
pub mod schedule;

pub use batch::BatchConfig;
pub use error::VMRogueError;

/// Get the VMRogue data directory (`~/.local/share/vmrogue` or platform equivalent).
///
/// Returns an error instead of silently falling back to `/tmp`, which would
/// be world-readable and a security risk on shared systems.
pub fn data_dir() -> anyhow::Result<std::path::PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "Could not determine data directory. Set XDG_DATA_HOME or HOME environment variable."
        )
    })?;
    Ok(base.join("vmrogue"))
}

/// Atomically write a serializable value to `path` with restrictive permissions.
///
/// Writes to a `.tmp` sibling first, then renames into place so readers never
/// see a partially-written file. On Unix the file is created with mode 0600.
/// The temp file is cleaned up on any error.
pub fn atomic_write(path: &std::path::Path, value: &impl serde::Serialize) -> anyhow::Result<()> {
    use anyhow::Context;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(value).context("Failed to serialize data")?;

    let tmp_path = path.with_extension(format!("tmp.{:08x}", rand::random::<u32>()));

    let write_result = atomic_write_inner(&tmp_path, &content);
    if let Err(ref _e) = write_result {
        // Clean up temp file on failure
        let _ = std::fs::remove_file(&tmp_path);
    }
    write_result?;

    std::fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "Failed to rename {} -> {}",
            tmp_path.display(),
            path.display()
        )
    })?;

    Ok(())
}

#[cfg(unix)]
fn atomic_write_inner(tmp_path: &std::path::Path, content: &str) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(tmp_path)
        .with_context(|| format!("Failed to open {}", tmp_path.display()))?;
    f.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write {}", tmp_path.display()))?;
    f.flush()?;
    f.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn atomic_write_inner(tmp_path: &std::path::Path, content: &str) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::io::Write;

    let mut f = std::fs::File::create(tmp_path)
        .with_context(|| format!("Failed to create {}", tmp_path.display()))?;
    f.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write {}", tmp_path.display()))?;
    f.sync_all()?;
    Ok(())
}

/// Generate a unique ID using microsecond timestamp + random suffix.
///
/// ```
/// use vmrogue::generate_id;
/// let id = generate_id("vm", "web-server");
/// assert!(id.starts_with("vm-"));
/// ```
pub fn generate_id(prefix: &str, name: &str) -> String {
    use chrono::Utc;
    let sanitized = name.to_lowercase().replace(' ', "-");
    let truncated_name = if sanitized.chars().count() > 20 {
        &sanitized[..sanitized
            .char_indices()
            .nth(20)
            .map(|(i, _)| i)
            .unwrap_or(sanitized.len())]
    } else {
        &sanitized
    };
    format!(
        "{}-{}-{:x}-{:08x}",
        prefix,
        truncated_name,
        Utc::now().timestamp_micros() as u64,
        rand::random::<u32>()
    )
}

/// Formats bytes using human-readable binary suffixes (KiB, MiB, GiB, TiB).
///
/// ```
/// use vmrogue::format_bytes;
/// assert_eq!(format_bytes(0), "0 B");
/// assert_eq!(format_bytes(1024), "1.00 KiB");
/// assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GiB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const KI: u64 = 1024;
    const MI: u64 = KI * 1024;
    const GI: u64 = MI * 1024;
    const TI: u64 = GI * 1024;

    if bytes >= TI {
        format!("{:.2} TiB", bytes as f64 / TI as f64)
    } else if bytes >= GI {
        format!("{:.2} GiB", bytes as f64 / GI as f64)
    } else if bytes >= MI {
        format!("{:.2} MiB", bytes as f64 / MI as f64)
    } else if bytes >= KI {
        format!("{:.2} KiB", bytes as f64 / KI as f64)
    } else {
        format!("{} B", bytes)
    }
}

// ── Kubernetes quantity parsing ─────────────────────────────────

/// Parse a Kubernetes memory quantity string to GiB.
/// Supports binary suffixes (Ti, Gi, Mi, Ki), decimal SI suffixes (T, G, M, k),
/// and raw bytes.
pub fn parse_memory_gib(s: &str) -> f64 {
    // Binary suffixes (powers of 1024)
    if let Some(v) = s.strip_suffix("Ti") {
        v.parse::<f64>().unwrap_or(0.0) * 1024.0
    } else if let Some(v) = s.strip_suffix("Gi") {
        v.parse::<f64>().unwrap_or(0.0)
    } else if let Some(v) = s.strip_suffix("Mi") {
        v.parse::<f64>().unwrap_or(0.0) / 1024.0
    } else if let Some(v) = s.strip_suffix("Ki") {
        v.parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
    // Decimal SI suffixes (powers of 1000)
    } else if let Some(v) = s.strip_suffix('T') {
        v.parse::<f64>().unwrap_or(0.0) * 1_000_000_000_000.0 / (1024.0 * 1024.0 * 1024.0)
    } else if let Some(v) = s.strip_suffix('G') {
        v.parse::<f64>().unwrap_or(0.0) * 1_000_000_000.0 / (1024.0 * 1024.0 * 1024.0)
    } else if let Some(v) = s.strip_suffix('M') {
        v.parse::<f64>().unwrap_or(0.0) * 1_000_000.0 / (1024.0 * 1024.0 * 1024.0)
    } else if let Some(v) = s.strip_suffix('k') {
        v.parse::<f64>().unwrap_or(0.0) * 1_000.0 / (1024.0 * 1024.0 * 1024.0)
    } else {
        // Raw bytes
        s.parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0 * 1024.0)
    }
}

/// Parse a Kubernetes memory quantity string to bytes.
/// Supports binary suffixes (Ti, Gi, Mi, Ki), decimal SI suffixes (T, G, M, k),
/// fractional values (e.g. "1.5Gi"), and raw bytes.
pub fn parse_memory_bytes(s: &str) -> u64 {
    // Binary suffixes (powers of 1024)
    if let Some(v) = s.strip_suffix("Ti") {
        (v.parse::<f64>().unwrap_or(0.0) * (1024.0 * 1024.0 * 1024.0 * 1024.0)) as u64
    } else if let Some(v) = s.strip_suffix("Gi") {
        (v.parse::<f64>().unwrap_or(0.0) * (1024.0 * 1024.0 * 1024.0)) as u64
    } else if let Some(v) = s.strip_suffix("Mi") {
        (v.parse::<f64>().unwrap_or(0.0) * (1024.0 * 1024.0)) as u64
    } else if let Some(v) = s.strip_suffix("Ki") {
        (v.parse::<f64>().unwrap_or(0.0) * 1024.0) as u64
    // Decimal SI suffixes (powers of 1000)
    } else if let Some(v) = s.strip_suffix('T') {
        (v.parse::<f64>().unwrap_or(0.0) * 1_000_000_000_000.0) as u64
    } else if let Some(v) = s.strip_suffix('G') {
        (v.parse::<f64>().unwrap_or(0.0) * 1_000_000_000.0) as u64
    } else if let Some(v) = s.strip_suffix('M') {
        (v.parse::<f64>().unwrap_or(0.0) * 1_000_000.0) as u64
    } else if let Some(v) = s.strip_suffix('k') {
        (v.parse::<f64>().unwrap_or(0.0) * 1_000.0) as u64
    } else {
        s.parse::<f64>().unwrap_or(0.0) as u64
    }
}

/// Parse a Kubernetes CPU quantity string to nanocores.
/// Supports suffixes: n (nanocores), m (millicores), and whole cores.
pub fn parse_cpu_nanocores(s: &str) -> u64 {
    if let Some(n) = s.strip_suffix('n') {
        n.parse::<u64>().unwrap_or(0)
    } else if let Some(m) = s.strip_suffix('m') {
        m.parse::<u64>().unwrap_or(0) * 1_000_000
    } else {
        (s.parse::<f64>().unwrap_or(0.0) * 1_000_000_000.0) as u64
    }
}

/// Convert a percentage (0.0-100.0) to u8 with saturation.
///
/// Values below 0 are clamped to 0, values above 100 are clamped to 100.
///
/// ```
/// use vmrogue::percent_to_u8;
/// assert_eq!(percent_to_u8(50.0), 50);
/// assert_eq!(percent_to_u8(150.0), 100);
/// assert_eq!(percent_to_u8(-10.0), 0);
/// ```
pub fn percent_to_u8(pct: f64) -> u8 {
    if pct.is_nan() {
        return 0;
    }
    if pct <= 0.0 {
        0
    } else if pct >= 100.0 {
        100
    } else {
        pct.round() as u8
    }
}
