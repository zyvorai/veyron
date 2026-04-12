// Manifest Management - Kubernetes/KubeVirt manifest operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manifest format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ManifestFormat {
    YAML,
    JSON,
}

/// Kubernetes manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub api_version: String,
    pub kind: String,
    pub metadata: ManifestMetadata,
    pub spec: HashMap<String, serde_json::Value>,
    pub status: Option<HashMap<String, serde_json::Value>>,
}

impl Manifest {
    pub fn new(
        api_version: impl Into<String>,
        kind: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            api_version: api_version.into(),
            kind: kind.into(),
            metadata: ManifestMetadata::new(name),
            spec: HashMap::new(),
            status: None,
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.metadata.namespace = Some(namespace.into());
        self
    }

    pub fn add_label(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.labels.insert(key.into(), value.into());
    }

    pub fn add_annotation(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.annotations.insert(key.into(), value.into());
    }

    pub fn is_namespaced(&self) -> bool {
        self.metadata.namespace.is_some()
    }

    pub fn full_name(&self) -> String {
        if let Some(ref ns) = self.metadata.namespace {
            format!("{}/{}/{}", self.kind, ns, self.metadata.name)
        } else {
            format!("{}/{}", self.kind, self.metadata.name)
        }
    }
}

/// Manifest metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMetadata {
    pub name: String,
    pub namespace: Option<String>,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub uid: Option<String>,
    pub resource_version: Option<String>,
}

impl ManifestMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            labels: HashMap::new(),
            annotations: HashMap::new(),
            uid: None,
            resource_version: None,
        }
    }
}

/// Manifest collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestCollection {
    pub manifests: Vec<Manifest>,
    pub source_path: String,
    pub loaded_at: DateTime<Utc>,
}

impl ManifestCollection {
    pub fn new(source_path: impl Into<String>) -> Self {
        Self {
            manifests: Vec::new(),
            source_path: source_path.into(),
            loaded_at: Utc::now(),
        }
    }

    pub fn add_manifest(&mut self, manifest: Manifest) {
        self.manifests.push(manifest);
    }

    pub fn count(&self) -> usize {
        self.manifests.len()
    }

    pub fn by_kind(&self, kind: &str) -> Vec<&Manifest> {
        self.manifests.iter().filter(|m| m.kind == kind).collect()
    }

    pub fn by_namespace(&self, namespace: &str) -> Vec<&Manifest> {
        self.manifests
            .iter()
            .filter(|m| m.metadata.namespace.as_deref() == Some(namespace))
            .collect()
    }

    pub fn get(&self, kind: &str, namespace: Option<&str>, name: &str) -> Option<&Manifest> {
        self.manifests.iter().find(|m| {
            m.kind == kind
                && m.metadata.name == name
                && m.metadata.namespace.as_deref() == namespace
        })
    }

    pub fn kinds(&self) -> Vec<String> {
        let mut kinds: Vec<String> = self.manifests.iter().map(|m| m.kind.clone()).collect();
        kinds.sort();
        kinds.dedup();
        kinds
    }
}

/// Manifest validator
pub struct ManifestValidator;

impl ManifestValidator {
    /// Validate manifest structure
    pub fn validate(manifest: &Manifest) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate API version
        if manifest.api_version.is_empty() {
            errors.push("API version is required".to_string());
        }

        // Validate kind
        if manifest.kind.is_empty() {
            errors.push("Kind is required".to_string());
        }

        // Validate name
        if manifest.metadata.name.is_empty() {
            errors.push("Name is required".to_string());
        }

        // Validate name format (DNS-1123 subdomain)
        if !Self::is_valid_name(&manifest.metadata.name) {
            errors.push(format!("Invalid name format: {}", manifest.metadata.name));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Check if name follows DNS-1123 subdomain rules
    fn is_valid_name(name: &str) -> bool {
        if name.is_empty() || name.len() > 253 {
            return false;
        }

        // Simple validation - alphanumeric and hyphens
        name.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '.')
    }

    /// Validate collection
    pub fn validate_collection(collection: &ManifestCollection) -> HashMap<String, Vec<String>> {
        let mut errors = HashMap::new();

        for manifest in &collection.manifests {
            if let Err(manifest_errors) = Self::validate(manifest) {
                errors.insert(manifest.full_name(), manifest_errors);
            }
        }

        errors
    }
}

/// Manifest diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestDiff {
    pub manifest_name: String,
    pub changes: Vec<DiffChange>,
}

impl ManifestDiff {
    pub fn new(manifest_name: impl Into<String>) -> Self {
        Self {
            manifest_name: manifest_name.into(),
            changes: Vec::new(),
        }
    }

    pub fn add_change(&mut self, change: DiffChange) {
        self.changes.push(change);
    }

    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    pub fn change_count(&self) -> usize {
        self.changes.len()
    }
}

/// Diff change type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffChange {
    Added {
        field: String,
        value: String,
    },
    Modified {
        field: String,
        old_value: String,
        new_value: String,
    },
    Removed {
        field: String,
        value: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_creation() {
        let manifest =
            Manifest::new("kubevirt.io/v1", "VirtualMachine", "test-vm").with_namespace("default");

        assert_eq!(manifest.api_version, "kubevirt.io/v1");
        assert_eq!(manifest.kind, "VirtualMachine");
        assert_eq!(manifest.metadata.name, "test-vm");
        assert_eq!(manifest.metadata.namespace, Some("default".to_string()));
    }

    #[test]
    fn test_manifest_labels() {
        let mut manifest = Manifest::new("v1", "Pod", "test");

        manifest.add_label("app", "web");
        manifest.add_label("env", "production");

        assert_eq!(manifest.metadata.labels.len(), 2);
        assert_eq!(
            manifest.metadata.labels.get("app"),
            Some(&"web".to_string())
        );
    }

    #[test]
    fn test_manifest_annotations() {
        let mut manifest = Manifest::new("v1", "Pod", "test");

        manifest.add_annotation("description", "Test pod");
        manifest.add_annotation("owner", "team-a");

        assert_eq!(manifest.metadata.annotations.len(), 2);
    }

    #[test]
    fn test_manifest_namespaced() {
        let namespaced = Manifest::new("v1", "Pod", "test").with_namespace("default");

        assert!(namespaced.is_namespaced());

        let cluster_scoped = Manifest::new("v1", "Node", "node-1");
        assert!(!cluster_scoped.is_namespaced());
    }

    #[test]
    fn test_manifest_full_name() {
        let namespaced = Manifest::new("v1", "Pod", "test-pod").with_namespace("default");

        assert_eq!(namespaced.full_name(), "Pod/default/test-pod");

        let cluster = Manifest::new("v1", "Node", "node-1");
        assert_eq!(cluster.full_name(), "Node/node-1");
    }

    #[test]
    fn test_manifest_collection() {
        let mut collection = ManifestCollection::new("/path/to/manifests");

        let manifest1 = Manifest::new("v1", "Pod", "pod-1");
        let manifest2 = Manifest::new("v1", "Service", "svc-1");

        collection.add_manifest(manifest1);
        collection.add_manifest(manifest2);

        assert_eq!(collection.count(), 2);
    }

    #[test]
    fn test_collection_by_kind() {
        let mut collection = ManifestCollection::new("/path");

        collection.add_manifest(Manifest::new("v1", "Pod", "pod-1"));
        collection.add_manifest(Manifest::new("v1", "Pod", "pod-2"));
        collection.add_manifest(Manifest::new("v1", "Service", "svc-1"));

        let pods = collection.by_kind("Pod");
        assert_eq!(pods.len(), 2);
    }

    #[test]
    fn test_collection_by_namespace() {
        let mut collection = ManifestCollection::new("/path");

        collection.add_manifest(Manifest::new("v1", "Pod", "pod-1").with_namespace("default"));
        collection.add_manifest(Manifest::new("v1", "Pod", "pod-2").with_namespace("kube-system"));

        let default_manifests = collection.by_namespace("default");
        assert_eq!(default_manifests.len(), 1);
    }

    #[test]
    fn test_collection_get() {
        let mut collection = ManifestCollection::new("/path");

        collection.add_manifest(Manifest::new("v1", "Pod", "test-pod").with_namespace("default"));

        let found = collection.get("Pod", Some("default"), "test-pod");
        assert!(found.is_some());

        let not_found = collection.get("Pod", Some("default"), "other-pod");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_collection_kinds() {
        let mut collection = ManifestCollection::new("/path");

        collection.add_manifest(Manifest::new("v1", "Pod", "pod-1"));
        collection.add_manifest(Manifest::new("v1", "Service", "svc-1"));
        collection.add_manifest(Manifest::new("v1", "Pod", "pod-2"));

        let kinds = collection.kinds();
        assert_eq!(kinds.len(), 2);
        assert!(kinds.contains(&"Pod".to_string()));
        assert!(kinds.contains(&"Service".to_string()));
    }

    #[test]
    fn test_manifest_validator() {
        let valid = Manifest::new("v1", "Pod", "test-pod");
        assert!(ManifestValidator::validate(&valid).is_ok());

        let invalid = Manifest::new("", "", "");
        assert!(ManifestValidator::validate(&invalid).is_err());
    }

    #[test]
    fn test_validator_name_format() {
        let valid = Manifest::new("v1", "Pod", "my-pod-123");
        assert!(ManifestValidator::validate(&valid).is_ok());

        let invalid = Manifest::new("v1", "Pod", "my pod");
        assert!(ManifestValidator::validate(&invalid).is_err());
    }

    #[test]
    fn test_validate_collection() {
        let mut collection = ManifestCollection::new("/path");

        collection.add_manifest(Manifest::new("v1", "Pod", "valid-pod"));
        collection.add_manifest(Manifest::new("", "Pod", ""));

        let errors = ManifestValidator::validate_collection(&collection);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_manifest_diff() {
        let mut diff = ManifestDiff::new("test-pod");

        diff.add_change(DiffChange::Added {
            field: "spec.replicas".to_string(),
            value: "3".to_string(),
        });

        diff.add_change(DiffChange::Modified {
            field: "spec.image".to_string(),
            old_value: "v1.0".to_string(),
            new_value: "v1.1".to_string(),
        });

        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 2);
    }

    #[test]
    fn test_diff_change_types() {
        let added = DiffChange::Added {
            field: "test".to_string(),
            value: "value".to_string(),
        };
        assert!(matches!(added, DiffChange::Added { .. }));

        let modified = DiffChange::Modified {
            field: "test".to_string(),
            old_value: "old".to_string(),
            new_value: "new".to_string(),
        };
        assert!(matches!(modified, DiffChange::Modified { .. }));

        let removed = DiffChange::Removed {
            field: "test".to_string(),
            value: "value".to_string(),
        };
        assert!(matches!(removed, DiffChange::Removed { .. }));
    }

    #[test]
    fn test_manifest_format() {
        assert_eq!(ManifestFormat::YAML, ManifestFormat::YAML);
        assert_ne!(ManifestFormat::YAML, ManifestFormat::JSON);
    }

    #[test]
    fn test_manifest_metadata() {
        let metadata = ManifestMetadata::new("test");

        assert_eq!(metadata.name, "test");
        assert!(metadata.namespace.is_none());
        assert!(metadata.labels.is_empty());
    }

    #[test]
    fn test_is_valid_name() {
        assert!(ManifestValidator::is_valid_name("my-app"));
        assert!(ManifestValidator::is_valid_name("app-123"));
        assert!(ManifestValidator::is_valid_name("my.app"));

        assert!(!ManifestValidator::is_valid_name(""));
        assert!(!ManifestValidator::is_valid_name("my app"));
        assert!(!ManifestValidator::is_valid_name("my_app"));
    }
}
