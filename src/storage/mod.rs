// Storage management - PVC and StorageClass operations for KubeVirt VMs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// PVC access modes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessMode {
    ReadWriteOnce,
    ReadWriteMany,
    ReadOnlyMany,
}

impl std::fmt::Display for AccessMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessMode::ReadWriteOnce => write!(f, "ReadWriteOnce"),
            AccessMode::ReadWriteMany => write!(f, "ReadWriteMany"),
            AccessMode::ReadOnlyMany => write!(f, "ReadOnlyMany"),
        }
    }
}

/// PVC status phases
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PvcPhase {
    Pending,
    Bound,
    Lost,
}

impl std::fmt::Display for PvcPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PvcPhase::Pending => write!(f, "Pending"),
            PvcPhase::Bound => write!(f, "Bound"),
            PvcPhase::Lost => write!(f, "Lost"),
        }
    }
}

/// Volume mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeMode {
    Filesystem,
    Block,
}

impl std::fmt::Display for VolumeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VolumeMode::Filesystem => write!(f, "Filesystem"),
            VolumeMode::Block => write!(f, "Block"),
        }
    }
}

/// PVC specification for VM disks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvcSpec {
    pub name: String,
    pub namespace: String,
    pub storage_class: Option<String>,
    pub size: String,
    pub access_modes: Vec<AccessMode>,
    pub volume_mode: VolumeMode,
    pub labels: std::collections::HashMap<String, String>,
}

impl PvcSpec {
    pub fn new(name: impl Into<String>, size: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: "default".to_string(),
            storage_class: None,
            size: size.into(),
            access_modes: vec![AccessMode::ReadWriteOnce],
            volume_mode: VolumeMode::Filesystem,
            labels: std::collections::HashMap::new(),
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn with_storage_class(mut self, class: impl Into<String>) -> Self {
        self.storage_class = Some(class.into());
        self
    }

    pub fn with_access_mode(mut self, mode: AccessMode) -> Self {
        self.access_modes = vec![mode];
        self
    }

    pub fn with_block_mode(mut self) -> Self {
        self.volume_mode = VolumeMode::Block;
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

/// PVC status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvcStatus {
    pub name: String,
    pub namespace: String,
    pub phase: PvcPhase,
    pub capacity: String,
    pub access_modes: Vec<AccessMode>,
    pub storage_class: Option<String>,
    pub volume_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl PvcStatus {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: namespace.into(),
            phase: PvcPhase::Pending,
            capacity: String::new(),
            access_modes: vec![AccessMode::ReadWriteOnce],
            storage_class: None,
            volume_name: None,
            created_at: Utc::now(),
        }
    }

    pub fn is_bound(&self) -> bool {
        self.phase == PvcPhase::Bound
    }

    pub fn age_display(&self) -> String {
        let duration = Utc::now().signed_duration_since(self.created_at);
        let days = duration.num_days();
        let hours = duration.num_hours() % 24;

        if days > 0 {
            format!("{}d{}h", days, hours)
        } else {
            format!("{}h", hours)
        }
    }
}

/// Storage class information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageClassInfo {
    pub name: String,
    pub provisioner: String,
    pub reclaim_policy: ReclaimPolicy,
    pub volume_binding_mode: VolumeBindingMode,
    pub allow_volume_expansion: bool,
    pub is_default: bool,
}

impl StorageClassInfo {
    pub fn new(name: impl Into<String>, provisioner: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            provisioner: provisioner.into(),
            reclaim_policy: ReclaimPolicy::Delete,
            volume_binding_mode: VolumeBindingMode::Immediate,
            allow_volume_expansion: false,
            is_default: false,
        }
    }

    pub fn with_reclaim_policy(mut self, policy: ReclaimPolicy) -> Self {
        self.reclaim_policy = policy;
        self
    }

    pub fn with_expansion(mut self, allow: bool) -> Self {
        self.allow_volume_expansion = allow;
        self
    }

    pub fn set_default(mut self) -> Self {
        self.is_default = true;
        self
    }
}

/// PV reclaim policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReclaimPolicy {
    Delete,
    Retain,
    Recycle,
}

impl std::fmt::Display for ReclaimPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReclaimPolicy::Delete => write!(f, "Delete"),
            ReclaimPolicy::Retain => write!(f, "Retain"),
            ReclaimPolicy::Recycle => write!(f, "Recycle"),
        }
    }
}

/// Volume binding mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeBindingMode {
    Immediate,
    WaitForFirstConsumer,
}

impl std::fmt::Display for VolumeBindingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VolumeBindingMode::Immediate => write!(f, "Immediate"),
            VolumeBindingMode::WaitForFirstConsumer => write!(f, "WaitForFirstConsumer"),
        }
    }
}

/// Parse a storage size string (e.g., "10Gi", "500Mi") to bytes
pub fn parse_size_to_bytes(size: &str) -> Option<u64> {
    let size = size.trim();
    if let Some(num) = size.strip_suffix("Ei") {
        num.parse::<f64>()
            .ok()
            .map(|v| (v * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64)
    } else if let Some(num) = size.strip_suffix("Pi") {
        num.parse::<f64>()
            .ok()
            .map(|v| (v * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64)
    } else if let Some(num) = size.strip_suffix("Ti") {
        num.parse::<f64>()
            .ok()
            .map(|v| (v * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64)
    } else if let Some(num) = size.strip_suffix("Gi") {
        num.parse::<f64>()
            .ok()
            .map(|v| (v * 1024.0 * 1024.0 * 1024.0) as u64)
    } else if let Some(num) = size.strip_suffix("Mi") {
        num.parse::<f64>()
            .ok()
            .map(|v| (v * 1024.0 * 1024.0) as u64)
    } else if let Some(num) = size.strip_suffix("Ki") {
        num.parse::<f64>().ok().map(|v| (v * 1024.0) as u64)
    } else {
        size.parse::<u64>().ok()
    }
}

/// Formats bytes using Kubernetes-style binary suffixes (Ki, Mi, Gi, Ti).
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 * 1024 {
        format!(
            "{:.1}Ti",
            bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)
        )
    } else if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1}Gi", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1}Mi", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1}Ki", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pvc_spec_builder() {
        let spec = PvcSpec::new("data-disk", "100Gi")
            .with_namespace("production")
            .with_storage_class("fast-ssd")
            .with_access_mode(AccessMode::ReadWriteMany)
            .with_label("app", "database");

        assert_eq!(spec.name, "data-disk");
        assert_eq!(spec.size, "100Gi");
        assert_eq!(spec.namespace, "production");
        assert_eq!(spec.storage_class, Some("fast-ssd".to_string()));
        assert_eq!(spec.access_modes, vec![AccessMode::ReadWriteMany]);
        assert_eq!(spec.volume_mode, VolumeMode::Filesystem);
        assert_eq!(spec.labels.get("app"), Some(&"database".to_string()));
    }

    #[test]
    fn test_pvc_spec_block_mode() {
        let spec = PvcSpec::new("raw-disk", "50Gi").with_block_mode();
        assert_eq!(spec.volume_mode, VolumeMode::Block);
    }

    #[test]
    fn test_pvc_status() {
        let mut status = PvcStatus::new("my-pvc", "default");
        assert!(!status.is_bound());
        assert_eq!(status.phase, PvcPhase::Pending);

        status.phase = PvcPhase::Bound;
        assert!(status.is_bound());
    }

    #[test]
    fn test_storage_class_info() {
        let sc = StorageClassInfo::new("fast-ssd", "kubernetes.io/aws-ebs")
            .with_reclaim_policy(ReclaimPolicy::Retain)
            .with_expansion(true)
            .set_default();

        assert_eq!(sc.name, "fast-ssd");
        assert_eq!(sc.provisioner, "kubernetes.io/aws-ebs");
        assert_eq!(sc.reclaim_policy, ReclaimPolicy::Retain);
        assert!(sc.allow_volume_expansion);
        assert!(sc.is_default);
    }

    #[test]
    fn test_parse_size_to_bytes() {
        assert_eq!(parse_size_to_bytes("1Ki"), Some(1024));
        assert_eq!(parse_size_to_bytes("1Mi"), Some(1024 * 1024));
        assert_eq!(parse_size_to_bytes("1Gi"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size_to_bytes("1Ti"), Some(1024 * 1024 * 1024 * 1024));
        assert_eq!(parse_size_to_bytes("100Gi"), Some(100 * 1024 * 1024 * 1024));
        assert_eq!(parse_size_to_bytes("512Mi"), Some(512 * 1024 * 1024));
        assert_eq!(parse_size_to_bytes("invalid"), None);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B");
        assert_eq!(format_bytes(1024), "1.0Ki");
        assert_eq!(format_bytes(1024 * 1024), "1.0Mi");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0Gi");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.0Ti");
        assert_eq!(format_bytes(10 * 1024 * 1024 * 1024), "10.0Gi");
    }

    #[test]
    fn test_parse_format_roundtrip() {
        let sizes = ["1Ki", "100Mi", "50Gi", "2Ti"];
        for size in sizes {
            let bytes = parse_size_to_bytes(size).unwrap();
            let formatted = format_bytes(bytes);
            assert_eq!(
                formatted,
                format!("{}.0{}", &size[..size.len() - 2], &size[size.len() - 2..])
            );
        }
    }

    #[test]
    fn test_display_traits() {
        assert_eq!(AccessMode::ReadWriteOnce.to_string(), "ReadWriteOnce");
        assert_eq!(AccessMode::ReadWriteMany.to_string(), "ReadWriteMany");
        assert_eq!(AccessMode::ReadOnlyMany.to_string(), "ReadOnlyMany");
        assert_eq!(PvcPhase::Pending.to_string(), "Pending");
        assert_eq!(PvcPhase::Bound.to_string(), "Bound");
        assert_eq!(PvcPhase::Lost.to_string(), "Lost");
        assert_eq!(VolumeMode::Filesystem.to_string(), "Filesystem");
        assert_eq!(VolumeMode::Block.to_string(), "Block");
        assert_eq!(ReclaimPolicy::Delete.to_string(), "Delete");
        assert_eq!(ReclaimPolicy::Retain.to_string(), "Retain");
        assert_eq!(VolumeBindingMode::Immediate.to_string(), "Immediate");
        assert_eq!(
            VolumeBindingMode::WaitForFirstConsumer.to_string(),
            "WaitForFirstConsumer"
        );
    }

    #[test]
    fn test_pvc_default_values() {
        let spec = PvcSpec::new("test", "10Gi");
        assert_eq!(spec.namespace, "default");
        assert_eq!(spec.access_modes, vec![AccessMode::ReadWriteOnce]);
        assert_eq!(spec.volume_mode, VolumeMode::Filesystem);
        assert!(spec.storage_class.is_none());
        assert!(spec.labels.is_empty());
    }
}
