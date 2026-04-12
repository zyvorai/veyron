pub mod batch;
pub mod cron;
pub mod error;
pub mod schedule;

pub use batch::BatchConfig;
pub use error::VMRogueError;

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
        &sanitized[..sanitized.char_indices().nth(20).map(|(i, _)| i).unwrap_or(sanitized.len())]
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
