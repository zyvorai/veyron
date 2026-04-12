// Role Management - Role definitions and assignments

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::permissions::Permission;

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: String,
    pub permissions: HashSet<Permission>,
    pub builtin: bool,
    pub created_at: DateTime<Utc>,
}

impl Role {
    pub fn new(name: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!("role-{}", name_str.to_lowercase().replace(' ', "-"));

        Self {
            id,
            name: name_str,
            description: String::new(),
            permissions: HashSet::new(),
            builtin: false,
            created_at: Utc::now(),
        }
    }

    pub fn builtin(name: impl Into<String>) -> Self {
        let mut role = Self::new(name);
        role.builtin = true;
        role
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn add_permission(mut self, permission: Permission) -> Self {
        self.permissions.insert(permission);
        self
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    pub fn permission_count(&self) -> usize {
        self.permissions.len()
    }
}

/// Built-in role templates
pub struct RoleTemplates;

impl RoleTemplates {
    /// Cluster administrator - full access
    pub fn cluster_admin() -> Role {
        Role::builtin("cluster-admin")
            .with_description("Full cluster administration access")
            .add_permission(Permission::all())
    }

    /// Namespace administrator - full access within namespace
    pub fn namespace_admin() -> Role {
        Role::builtin("namespace-admin")
            .with_description("Full access within namespace")
            .add_permission(Permission::vm_create())
            .add_permission(Permission::vm_delete())
            .add_permission(Permission::vm_update())
            .add_permission(Permission::vm_view())
            .add_permission(Permission::vm_start())
            .add_permission(Permission::vm_stop())
            .add_permission(Permission::snapshot_create())
            .add_permission(Permission::snapshot_delete())
            .add_permission(Permission::backup_create())
    }

    /// Developer - create and manage VMs
    pub fn developer() -> Role {
        Role::builtin("developer")
            .with_description("Create and manage VMs")
            .add_permission(Permission::vm_create())
            .add_permission(Permission::vm_update())
            .add_permission(Permission::vm_view())
            .add_permission(Permission::vm_start())
            .add_permission(Permission::vm_stop())
            .add_permission(Permission::snapshot_create())
            .add_permission(Permission::snapshot_view())
    }

    /// Operator - manage existing VMs
    pub fn operator() -> Role {
        Role::builtin("operator")
            .with_description("Manage existing VMs")
            .add_permission(Permission::vm_view())
            .add_permission(Permission::vm_start())
            .add_permission(Permission::vm_stop())
            .add_permission(Permission::vm_restart())
            .add_permission(Permission::snapshot_view())
    }

    /// Viewer - read-only access
    pub fn viewer() -> Role {
        Role::builtin("viewer")
            .with_description("Read-only access")
            .add_permission(Permission::vm_view())
            .add_permission(Permission::snapshot_view())
            .add_permission(Permission::backup_view())
    }

    /// Auditor - audit and compliance access
    pub fn auditor() -> Role {
        Role::builtin("auditor")
            .with_description("Audit and compliance access")
            .add_permission(Permission::audit_view())
            .add_permission(Permission::compliance_view())
            .add_permission(Permission::vm_view())
    }

    /// Backup operator - backup and restore
    pub fn backup_operator() -> Role {
        Role::builtin("backup-operator")
            .with_description("Backup and restore operations")
            .add_permission(Permission::backup_create())
            .add_permission(Permission::backup_delete())
            .add_permission(Permission::backup_view())
            .add_permission(Permission::snapshot_create())
            .add_permission(Permission::snapshot_view())
    }

    /// Security officer - security operations
    pub fn security_officer() -> Role {
        Role::builtin("security-officer")
            .with_description("Security and compliance operations")
            .add_permission(Permission::security_scan())
            .add_permission(Permission::compliance_view())
            .add_permission(Permission::audit_view())
            .add_permission(Permission::vm_view())
    }

    /// Get all built-in roles
    pub fn all() -> Vec<Role> {
        vec![
            Self::cluster_admin(),
            Self::namespace_admin(),
            Self::developer(),
            Self::operator(),
            Self::viewer(),
            Self::auditor(),
            Self::backup_operator(),
            Self::security_officer(),
        ]
    }

    /// Get role by name
    pub fn get(name: &str) -> Option<Role> {
        Self::all().into_iter().find(|r| r.name == name)
    }
}

/// Role binding - assigns roles to users or groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleBinding {
    pub id: String,
    pub role_id: String,
    pub subject: Subject,
    pub scope: BindingScope,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

impl RoleBinding {
    pub fn new(role_id: impl Into<String>, subject: Subject, scope: BindingScope) -> Self {
        Self {
            id: format!("binding-{}", Utc::now().timestamp_millis()),
            role_id: role_id.into(),
            subject,
            scope,
            created_at: Utc::now(),
            created_by: "system".to_string(),
        }
    }

    pub fn by(mut self, user: impl Into<String>) -> Self {
        self.created_by = user.into();
        self
    }
}

/// Subject of a role binding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Subject {
    User { user_id: String },
    Group { group_id: String },
}

/// Scope of a role binding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BindingScope {
    Cluster,
    Namespace { namespace: String },
}

/// Role manager
pub struct RoleManager {
    roles: std::collections::HashMap<String, Role>,
    bindings: Vec<RoleBinding>,
}

impl RoleManager {
    pub fn new() -> Self {
        let mut manager = Self {
            roles: std::collections::HashMap::new(),
            bindings: Vec::new(),
        };

        // Add built-in roles
        for role in RoleTemplates::all() {
            manager.add_role(role);
        }

        manager
    }

    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.id.clone(), role);
    }

    pub fn get_role(&self, role_id: &str) -> Option<&Role> {
        self.roles.get(role_id)
    }

    pub fn remove_role(&mut self, role_id: &str) -> bool {
        if let Some(role) = self.roles.get(role_id) {
            if role.builtin {
                return false; // Cannot remove built-in roles
            }
        }
        self.roles.remove(role_id).is_some()
    }

    pub fn list_roles(&self) -> Vec<&Role> {
        self.roles.values().collect()
    }

    pub fn builtin_roles(&self) -> Vec<&Role> {
        self.roles.values().filter(|r| r.builtin).collect()
    }

    pub fn custom_roles(&self) -> Vec<&Role> {
        self.roles.values().filter(|r| !r.builtin).collect()
    }

    pub fn add_binding(&mut self, binding: RoleBinding) {
        self.bindings.push(binding);
    }

    pub fn remove_binding(&mut self, binding_id: &str) {
        self.bindings.retain(|b| b.id != binding_id);
    }

    pub fn get_bindings_for_user(&self, user_id: &str) -> Vec<&RoleBinding> {
        self.bindings
            .iter()
            .filter(|b| matches!(&b.subject, Subject::User { user_id: uid } if uid == user_id))
            .collect()
    }

    pub fn get_bindings_for_group(&self, group_id: &str) -> Vec<&RoleBinding> {
        self.bindings
            .iter()
            .filter(|b| matches!(&b.subject, Subject::Group { group_id: gid } if gid == group_id))
            .collect()
    }

    pub fn get_user_permissions(&self, user_id: &str, groups: &[String]) -> HashSet<Permission> {
        let mut permissions = HashSet::new();

        // Get permissions from user bindings
        for binding in self.get_bindings_for_user(user_id) {
            if let Some(role) = self.get_role(&binding.role_id) {
                permissions.extend(role.permissions.clone());
            }
        }

        // Get permissions from group bindings
        for group_id in groups {
            for binding in self.get_bindings_for_group(group_id) {
                if let Some(role) = self.get_role(&binding.role_id) {
                    permissions.extend(role.permissions.clone());
                }
            }
        }

        permissions
    }
}

impl Default for RoleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_creation() {
        let role = Role::new("custom-role")
            .with_description("Custom role for testing")
            .add_permission(Permission::vm_view());

        assert_eq!(role.name, "custom-role");
        assert!(!role.builtin);
        assert_eq!(role.permission_count(), 1);
    }

    #[test]
    fn test_builtin_role() {
        let role = Role::builtin("test");
        assert!(role.builtin);
    }

    #[test]
    fn test_role_permissions() {
        let role = Role::new("test")
            .add_permission(Permission::vm_view())
            .add_permission(Permission::vm_create());

        assert!(role.has_permission(&Permission::vm_view()));
        assert!(role.has_permission(&Permission::vm_create()));
        assert!(!role.has_permission(&Permission::vm_delete()));
    }

    #[test]
    fn test_cluster_admin_template() {
        let role = RoleTemplates::cluster_admin();
        assert_eq!(role.name, "cluster-admin");
        assert!(role.builtin);
        assert!(role.has_permission(&Permission::all()));
    }

    #[test]
    fn test_developer_template() {
        let role = RoleTemplates::developer();
        assert!(role.has_permission(&Permission::vm_create()));
        assert!(role.has_permission(&Permission::vm_view()));
        assert!(!role.has_permission(&Permission::vm_delete()));
    }

    #[test]
    fn test_viewer_template() {
        let role = RoleTemplates::viewer();
        assert!(role.has_permission(&Permission::vm_view()));
        assert!(!role.has_permission(&Permission::vm_create()));
        assert!(!role.has_permission(&Permission::vm_delete()));
    }

    #[test]
    fn test_role_templates_all() {
        let roles = RoleTemplates::all();
        assert!(roles.len() >= 7); // At least 7 built-in roles
        assert!(roles.iter().all(|r| r.builtin));
    }

    #[test]
    fn test_role_templates_get() {
        let role = RoleTemplates::get("developer");
        assert!(role.is_some());
        assert_eq!(role.unwrap().name, "developer");

        let missing = RoleTemplates::get("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_role_binding() {
        let binding = RoleBinding::new(
            "role-admin",
            Subject::User {
                user_id: "user-1".to_string(),
            },
            BindingScope::Namespace {
                namespace: "default".to_string(),
            },
        )
        .by("admin");

        assert_eq!(binding.role_id, "role-admin");
        assert_eq!(binding.created_by, "admin");
    }

    #[test]
    fn test_binding_subject() {
        let user_subject = Subject::User {
            user_id: "user-1".to_string(),
        };
        let group_subject = Subject::Group {
            group_id: "group-1".to_string(),
        };

        assert!(matches!(user_subject, Subject::User { .. }));
        assert!(matches!(group_subject, Subject::Group { .. }));
    }

    #[test]
    fn test_binding_scope() {
        let cluster = BindingScope::Cluster;
        let namespace = BindingScope::Namespace {
            namespace: "test".to_string(),
        };

        assert_eq!(cluster, BindingScope::Cluster);
        assert!(matches!(namespace, BindingScope::Namespace { .. }));
    }

    #[test]
    fn test_role_manager() {
        let manager = RoleManager::new();

        // Should have built-in roles
        assert!(!manager.builtin_roles().is_empty());
        assert!(manager.get_role("role-viewer").is_some());
    }

    #[test]
    fn test_add_custom_role() {
        let mut manager = RoleManager::new();
        let builtin_count = manager.builtin_roles().len();

        let custom = Role::new("custom").add_permission(Permission::vm_view());

        manager.add_role(custom);

        assert_eq!(manager.builtin_roles().len(), builtin_count);
        assert_eq!(manager.custom_roles().len(), 1);
    }

    #[test]
    fn test_cannot_remove_builtin_roles() {
        let mut manager = RoleManager::new();
        let result = manager.remove_role("role-viewer");
        assert!(!result); // Should fail
    }

    #[test]
    fn test_can_remove_custom_roles() {
        let mut manager = RoleManager::new();

        let custom = Role::new("temporary");
        manager.add_role(custom);

        let result = manager.remove_role("role-temporary");
        assert!(result); // Should succeed
    }

    #[test]
    fn test_role_bindings() {
        let mut manager = RoleManager::new();

        let binding = RoleBinding::new(
            "role-developer",
            Subject::User {
                user_id: "user-alice".to_string(),
            },
            BindingScope::Cluster,
        );

        manager.add_binding(binding);

        let user_bindings = manager.get_bindings_for_user("user-alice");
        assert_eq!(user_bindings.len(), 1);
    }

    #[test]
    fn test_group_bindings() {
        let mut manager = RoleManager::new();

        let binding = RoleBinding::new(
            "role-viewer",
            Subject::Group {
                group_id: "group-eng".to_string(),
            },
            BindingScope::Namespace {
                namespace: "dev".to_string(),
            },
        );

        manager.add_binding(binding);

        let group_bindings = manager.get_bindings_for_group("group-eng");
        assert_eq!(group_bindings.len(), 1);
    }

    #[test]
    fn test_get_user_permissions() {
        let mut manager = RoleManager::new();

        // Create user binding
        let user_binding = RoleBinding::new(
            "role-developer",
            Subject::User {
                user_id: "user-bob".to_string(),
            },
            BindingScope::Cluster,
        );

        manager.add_binding(user_binding);

        let permissions = manager.get_user_permissions("user-bob", &[]);
        assert!(!permissions.is_empty());
    }

    #[test]
    fn test_get_permissions_from_groups() {
        let mut manager = RoleManager::new();

        // Create group binding
        let group_binding = RoleBinding::new(
            "role-viewer",
            Subject::Group {
                group_id: "group-readonly".to_string(),
            },
            BindingScope::Cluster,
        );

        manager.add_binding(group_binding);

        let permissions =
            manager.get_user_permissions("user-charlie", &["group-readonly".to_string()]);
        assert!(!permissions.is_empty());
    }

    #[test]
    fn test_remove_binding() {
        let mut manager = RoleManager::new();

        let binding = RoleBinding::new(
            "role-test",
            Subject::User {
                user_id: "user-test".to_string(),
            },
            BindingScope::Cluster,
        );

        let binding_id = binding.id.clone();
        manager.add_binding(binding);

        manager.remove_binding(&binding_id);

        let bindings = manager.get_bindings_for_user("user-test");
        assert_eq!(bindings.len(), 0);
    }
}
