// Permission System - Fine-grained permission definitions

use serde::{Deserialize, Serialize};

/// Permission for specific actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    // VM Management
    VMCreate,
    VMDelete,
    VMUpdate,
    VMView,
    VMStart,
    VMStop,
    VMRestart,
    VMPause,
    VMUnpause,
    VMMigrate,

    // Snapshot Management
    SnapshotCreate,
    SnapshotDelete,
    SnapshotRestore,
    SnapshotView,

    // Backup Management
    BackupCreate,
    BackupDelete,
    BackupRestore,
    BackupView,

    // Network Management
    NetworkCreate,
    NetworkDelete,
    NetworkUpdate,
    NetworkView,

    // Storage Management
    StorageCreate,
    StorageDelete,
    StorageUpdate,
    StorageView,

    // Security Operations
    SecurityScan,
    SecurityHarden,
    ComplianceView,
    AuditView,

    // Cost Management
    CostView,
    BudgetManage,

    // Automation
    AutomationCreate,
    AutomationDelete,
    AutomationExecute,
    AutomationView,

    // Observability
    LogsView,
    MetricsView,
    AlertsManage,

    // Tenant Management
    TenantCreate,
    TenantDelete,
    TenantUpdate,
    TenantView,

    // Role Management
    RoleCreate,
    RoleDelete,
    RoleAssign,
    RoleView,

    // All permissions (super admin)
    All,
}

impl Permission {
    /// Check if this permission implies another permission
    pub fn implies(&self, other: &Permission) -> bool {
        if self == other {
            return true;
        }

        // All permission implies everything
        if matches!(self, Permission::All) {
            return true;
        }

        false
    }

    // VM permissions
    pub fn vm_create() -> Self {
        Permission::VMCreate
    }
    pub fn vm_delete() -> Self {
        Permission::VMDelete
    }
    pub fn vm_update() -> Self {
        Permission::VMUpdate
    }
    pub fn vm_view() -> Self {
        Permission::VMView
    }
    pub fn vm_start() -> Self {
        Permission::VMStart
    }
    pub fn vm_stop() -> Self {
        Permission::VMStop
    }
    pub fn vm_restart() -> Self {
        Permission::VMRestart
    }
    pub fn vm_pause() -> Self {
        Permission::VMPause
    }
    pub fn vm_unpause() -> Self {
        Permission::VMUnpause
    }
    pub fn vm_migrate() -> Self {
        Permission::VMMigrate
    }

    // Snapshot permissions
    pub fn snapshot_create() -> Self {
        Permission::SnapshotCreate
    }
    pub fn snapshot_delete() -> Self {
        Permission::SnapshotDelete
    }
    pub fn snapshot_restore() -> Self {
        Permission::SnapshotRestore
    }
    pub fn snapshot_view() -> Self {
        Permission::SnapshotView
    }

    // Backup permissions
    pub fn backup_create() -> Self {
        Permission::BackupCreate
    }
    pub fn backup_delete() -> Self {
        Permission::BackupDelete
    }
    pub fn backup_restore() -> Self {
        Permission::BackupRestore
    }
    pub fn backup_view() -> Self {
        Permission::BackupView
    }

    // Network permissions
    pub fn network_create() -> Self {
        Permission::NetworkCreate
    }
    pub fn network_delete() -> Self {
        Permission::NetworkDelete
    }
    pub fn network_update() -> Self {
        Permission::NetworkUpdate
    }
    pub fn network_view() -> Self {
        Permission::NetworkView
    }

    // Storage permissions
    pub fn storage_create() -> Self {
        Permission::StorageCreate
    }
    pub fn storage_delete() -> Self {
        Permission::StorageDelete
    }
    pub fn storage_update() -> Self {
        Permission::StorageUpdate
    }
    pub fn storage_view() -> Self {
        Permission::StorageView
    }

    // Security permissions
    pub fn security_scan() -> Self {
        Permission::SecurityScan
    }
    pub fn security_harden() -> Self {
        Permission::SecurityHarden
    }
    pub fn compliance_view() -> Self {
        Permission::ComplianceView
    }
    pub fn audit_view() -> Self {
        Permission::AuditView
    }

    // Cost permissions
    pub fn cost_view() -> Self {
        Permission::CostView
    }
    pub fn budget_manage() -> Self {
        Permission::BudgetManage
    }

    // Automation permissions
    pub fn automation_create() -> Self {
        Permission::AutomationCreate
    }
    pub fn automation_delete() -> Self {
        Permission::AutomationDelete
    }
    pub fn automation_execute() -> Self {
        Permission::AutomationExecute
    }
    pub fn automation_view() -> Self {
        Permission::AutomationView
    }

    // Observability permissions
    pub fn logs_view() -> Self {
        Permission::LogsView
    }
    pub fn metrics_view() -> Self {
        Permission::MetricsView
    }
    pub fn alerts_manage() -> Self {
        Permission::AlertsManage
    }

    // Tenant permissions
    pub fn tenant_create() -> Self {
        Permission::TenantCreate
    }
    pub fn tenant_delete() -> Self {
        Permission::TenantDelete
    }
    pub fn tenant_update() -> Self {
        Permission::TenantUpdate
    }
    pub fn tenant_view() -> Self {
        Permission::TenantView
    }

    // Role permissions
    pub fn role_create() -> Self {
        Permission::RoleCreate
    }
    pub fn role_delete() -> Self {
        Permission::RoleDelete
    }
    pub fn role_assign() -> Self {
        Permission::RoleAssign
    }
    pub fn role_view() -> Self {
        Permission::RoleView
    }

    // Super admin
    pub fn all() -> Self {
        Permission::All
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Permission::VMCreate => write!(f, "vm:create"),
            Permission::VMDelete => write!(f, "vm:delete"),
            Permission::VMUpdate => write!(f, "vm:update"),
            Permission::VMView => write!(f, "vm:view"),
            Permission::VMStart => write!(f, "vm:start"),
            Permission::VMStop => write!(f, "vm:stop"),
            Permission::VMRestart => write!(f, "vm:restart"),
            Permission::VMPause => write!(f, "vm:pause"),
            Permission::VMUnpause => write!(f, "vm:unpause"),
            Permission::VMMigrate => write!(f, "vm:migrate"),

            Permission::SnapshotCreate => write!(f, "snapshot:create"),
            Permission::SnapshotDelete => write!(f, "snapshot:delete"),
            Permission::SnapshotRestore => write!(f, "snapshot:restore"),
            Permission::SnapshotView => write!(f, "snapshot:view"),

            Permission::BackupCreate => write!(f, "backup:create"),
            Permission::BackupDelete => write!(f, "backup:delete"),
            Permission::BackupRestore => write!(f, "backup:restore"),
            Permission::BackupView => write!(f, "backup:view"),

            Permission::NetworkCreate => write!(f, "network:create"),
            Permission::NetworkDelete => write!(f, "network:delete"),
            Permission::NetworkUpdate => write!(f, "network:update"),
            Permission::NetworkView => write!(f, "network:view"),

            Permission::StorageCreate => write!(f, "storage:create"),
            Permission::StorageDelete => write!(f, "storage:delete"),
            Permission::StorageUpdate => write!(f, "storage:update"),
            Permission::StorageView => write!(f, "storage:view"),

            Permission::SecurityScan => write!(f, "security:scan"),
            Permission::SecurityHarden => write!(f, "security:harden"),
            Permission::ComplianceView => write!(f, "compliance:view"),
            Permission::AuditView => write!(f, "audit:view"),

            Permission::CostView => write!(f, "cost:view"),
            Permission::BudgetManage => write!(f, "budget:manage"),

            Permission::AutomationCreate => write!(f, "automation:create"),
            Permission::AutomationDelete => write!(f, "automation:delete"),
            Permission::AutomationExecute => write!(f, "automation:execute"),
            Permission::AutomationView => write!(f, "automation:view"),

            Permission::LogsView => write!(f, "logs:view"),
            Permission::MetricsView => write!(f, "metrics:view"),
            Permission::AlertsManage => write!(f, "alerts:manage"),

            Permission::TenantCreate => write!(f, "tenant:create"),
            Permission::TenantDelete => write!(f, "tenant:delete"),
            Permission::TenantUpdate => write!(f, "tenant:update"),
            Permission::TenantView => write!(f, "tenant:view"),

            Permission::RoleCreate => write!(f, "role:create"),
            Permission::RoleDelete => write!(f, "role:delete"),
            Permission::RoleAssign => write!(f, "role:assign"),
            Permission::RoleView => write!(f, "role:view"),

            Permission::All => write!(f, "*"),
        }
    }
}

/// Permission checker
pub struct PermissionChecker;

impl PermissionChecker {
    /// Check if a set of permissions allows a specific permission
    pub fn has_permission(
        user_permissions: &std::collections::HashSet<Permission>,
        required: &Permission,
    ) -> bool {
        user_permissions.iter().any(|p| p.implies(required))
    }

    /// Check if user has all required permissions
    pub fn has_all_permissions(
        user_permissions: &std::collections::HashSet<Permission>,
        required: &[Permission],
    ) -> bool {
        required
            .iter()
            .all(|req| Self::has_permission(user_permissions, req))
    }

    /// Check if user has any of the required permissions
    pub fn has_any_permission(
        user_permissions: &std::collections::HashSet<Permission>,
        required: &[Permission],
    ) -> bool {
        required
            .iter()
            .any(|req| Self::has_permission(user_permissions, req))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_permission_equality() {
        assert_eq!(Permission::VMCreate, Permission::VMCreate);
        assert_ne!(Permission::VMCreate, Permission::VMDelete);
    }

    #[test]
    fn test_permission_implies() {
        let all = Permission::All;
        assert!(all.implies(&Permission::VMCreate));
        assert!(all.implies(&Permission::VMDelete));
        assert!(all.implies(&Permission::All));
    }

    #[test]
    fn test_permission_implies_self() {
        let perm = Permission::VMCreate;
        assert!(perm.implies(&Permission::VMCreate));
        assert!(!perm.implies(&Permission::VMDelete));
    }

    #[test]
    fn test_permission_display() {
        assert_eq!(Permission::VMCreate.to_string(), "vm:create");
        assert_eq!(Permission::SnapshotView.to_string(), "snapshot:view");
        assert_eq!(Permission::All.to_string(), "*");
    }

    #[test]
    fn test_permission_constructors() {
        assert_eq!(Permission::vm_create(), Permission::VMCreate);
        assert_eq!(Permission::vm_view(), Permission::VMView);
        assert_eq!(Permission::snapshot_create(), Permission::SnapshotCreate);
    }

    #[test]
    fn test_permission_checker_has_permission() {
        let mut perms = HashSet::new();
        perms.insert(Permission::VMCreate);
        perms.insert(Permission::VMView);

        assert!(PermissionChecker::has_permission(
            &perms,
            &Permission::VMCreate
        ));
        assert!(PermissionChecker::has_permission(
            &perms,
            &Permission::VMView
        ));
        assert!(!PermissionChecker::has_permission(
            &perms,
            &Permission::VMDelete
        ));
    }

    #[test]
    fn test_permission_checker_with_all() {
        let mut perms = HashSet::new();
        perms.insert(Permission::All);

        assert!(PermissionChecker::has_permission(
            &perms,
            &Permission::VMCreate
        ));
        assert!(PermissionChecker::has_permission(
            &perms,
            &Permission::VMDelete
        ));
        assert!(PermissionChecker::has_permission(
            &perms,
            &Permission::SnapshotCreate
        ));
    }

    #[test]
    fn test_permission_checker_has_all() {
        let mut perms = HashSet::new();
        perms.insert(Permission::VMCreate);
        perms.insert(Permission::VMView);
        perms.insert(Permission::VMDelete);

        let required = vec![Permission::VMCreate, Permission::VMView];
        assert!(PermissionChecker::has_all_permissions(&perms, &required));

        let missing = vec![Permission::VMCreate, Permission::SnapshotCreate];
        assert!(!PermissionChecker::has_all_permissions(&perms, &missing));
    }

    #[test]
    fn test_permission_checker_has_any() {
        let mut perms = HashSet::new();
        perms.insert(Permission::VMView);

        let required = vec![Permission::VMCreate, Permission::VMView];
        assert!(PermissionChecker::has_any_permission(&perms, &required));

        let none_match = vec![Permission::VMCreate, Permission::VMDelete];
        assert!(!PermissionChecker::has_any_permission(&perms, &none_match));
    }

    #[test]
    fn test_vm_permissions() {
        assert_eq!(Permission::vm_start(), Permission::VMStart);
        assert_eq!(Permission::vm_stop(), Permission::VMStop);
        assert_eq!(Permission::vm_restart(), Permission::VMRestart);
        assert_eq!(Permission::vm_pause(), Permission::VMPause);
        assert_eq!(Permission::vm_unpause(), Permission::VMUnpause);
        assert_eq!(Permission::vm_migrate(), Permission::VMMigrate);
    }

    #[test]
    fn test_snapshot_permissions() {
        assert_eq!(Permission::snapshot_create(), Permission::SnapshotCreate);
        assert_eq!(Permission::snapshot_delete(), Permission::SnapshotDelete);
        assert_eq!(Permission::snapshot_restore(), Permission::SnapshotRestore);
        assert_eq!(Permission::snapshot_view(), Permission::SnapshotView);
    }

    #[test]
    fn test_backup_permissions() {
        assert_eq!(Permission::backup_create(), Permission::BackupCreate);
        assert_eq!(Permission::backup_delete(), Permission::BackupDelete);
        assert_eq!(Permission::backup_restore(), Permission::BackupRestore);
        assert_eq!(Permission::backup_view(), Permission::BackupView);
    }

    #[test]
    fn test_security_permissions() {
        assert_eq!(Permission::security_scan(), Permission::SecurityScan);
        assert_eq!(Permission::security_harden(), Permission::SecurityHarden);
        assert_eq!(Permission::compliance_view(), Permission::ComplianceView);
        assert_eq!(Permission::audit_view(), Permission::AuditView);
    }

    #[test]
    fn test_automation_permissions() {
        assert_eq!(
            Permission::automation_create(),
            Permission::AutomationCreate
        );
        assert_eq!(
            Permission::automation_delete(),
            Permission::AutomationDelete
        );
        assert_eq!(
            Permission::automation_execute(),
            Permission::AutomationExecute
        );
        assert_eq!(Permission::automation_view(), Permission::AutomationView);
    }

    #[test]
    fn test_tenant_permissions() {
        assert_eq!(Permission::tenant_create(), Permission::TenantCreate);
        assert_eq!(Permission::tenant_delete(), Permission::TenantDelete);
        assert_eq!(Permission::tenant_update(), Permission::TenantUpdate);
        assert_eq!(Permission::tenant_view(), Permission::TenantView);
    }

    #[test]
    fn test_role_permissions() {
        assert_eq!(Permission::role_create(), Permission::RoleCreate);
        assert_eq!(Permission::role_delete(), Permission::RoleDelete);
        assert_eq!(Permission::role_assign(), Permission::RoleAssign);
        assert_eq!(Permission::role_view(), Permission::RoleView);
    }

    #[test]
    fn test_permission_hash() {
        let mut set = HashSet::new();
        set.insert(Permission::VMCreate);
        set.insert(Permission::VMCreate); // Duplicate

        assert_eq!(set.len(), 1); // Should only have one entry
    }
}
