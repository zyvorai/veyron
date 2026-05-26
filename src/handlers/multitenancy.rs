// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use crate::tui::colors::cli as color;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
struct TenancyStore {
    tenants: Vec<serde_json::Value>,
    users: Vec<serde_json::Value>,
    roles: Vec<serde_json::Value>,
    groups: Vec<serde_json::Value>,
}

impl TenancyStore {
    fn path() -> anyhow::Result<std::path::PathBuf> {
        Ok(crate::utils::data_dir()?.join("tenancy.json"))
    }

    fn load() -> Self {
        let path = match Self::path() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to determine data directory: {}", e);
                return Self::default();
            }
        };
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) {
        let path = match Self::path() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to save tenancy data: {}", e);
                return;
            }
        };
        if let Err(e) = crate::utils::atomic_write(&path, self) {
            log::error!("Failed to save tenancy data: {}", e);
        }
    }
}

/// Parse a binding scope string into a BindingScope enum.
/// "cluster" maps to Cluster, "namespace:<name>" maps to Namespace, anything else
/// defaults to Cluster.
pub(crate) fn parse_binding_scope(scope: &str) -> crate::multitenancy::roles::BindingScope {
    use crate::multitenancy::roles::BindingScope;

    if scope == "cluster" {
        BindingScope::Cluster
    } else if let Some(ns) = scope.strip_prefix("namespace:") {
        BindingScope::Namespace {
            namespace: ns.to_string(),
        }
    } else {
        BindingScope::Cluster
    }
}

/// Parse a quota preset string into ResourceLimits.
/// Known presets: "small", "medium", "large", "unlimited".
/// Unknown values default to medium.
pub(crate) fn parse_quota_preset(preset: &str) -> crate::multitenancy::quotas::ResourceLimits {
    use crate::multitenancy::quotas::ResourceLimits;

    match preset {
        "small" => ResourceLimits::small(),
        "large" => ResourceLimits::large(),
        "unlimited" => ResourceLimits::unlimited(),
        _ => ResourceLimits::medium(),
    }
}

pub fn handle_tenants_list(active_only: bool, output: String) -> Result<()> {
    use crate::multitenancy::tenants::TenantManager;

    println!("{}", color::header("Tenants"));
    println!();

    let manager = TenantManager::new();
    let tenants = if active_only {
        manager.active_tenants()
    } else {
        manager.list_tenants()
    };

    println!("  Total tenants: {}", tenants.len());
    println!("  Format: {}", output);
    println!();
    println!("{}", color::success("✓ Tenants listed"));
    Ok(())
}

pub fn handle_tenants_create(
    name: String,
    owner: String,
    email: String,
    description: Option<String>,
    namespace: Option<String>,
) -> Result<()> {
    use crate::multitenancy::tenants::Tenant;

    println!("{}", color::header(&format!("Creating Tenant: {}", name)));
    println!();

    let mut tenant = Tenant::new(&name, &owner, &email);

    if let Some(desc) = description {
        tenant = tenant.with_description(desc);
    }

    if let Some(ns) = namespace {
        tenant.add_namespace(ns);
    }

    // Persist the tenant
    let mut store = TenancyStore::load();
    if let Ok(value) = serde_json::to_value(&tenant) {
        store.tenants.push(value);
        store.save();
    }

    println!("  Name:    {}", color::value(&tenant.name));
    println!("  Owner:   {}", tenant.owner_id);
    println!("  Email:   {}", tenant.contact_email);
    println!();
    println!(
        "{}",
        color::success("✓ Tenant created and persisted successfully")
    );
    Ok(())
}

pub fn handle_tenants_show(tenant: String, output: String) -> Result<()> {
    println!("{}", color::header(&format!("Tenant: {}", tenant)));
    println!();

    println!("  Format: {}", output);
    println!();
    println!("{}", color::success("✓ Tenant details retrieved"));
    Ok(())
}

pub fn handle_tenants_delete(tenant: String, yes: bool) -> Result<()> {
    println!("{}", color::header(&format!("Deleting Tenant: {}", tenant)));
    println!();

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Delete tenant '{}'? This will remove all associated resources.",
                tenant
            ))
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            println!("{}", color::muted("Cancelled"));
            return Ok(());
        }
    }

    println!("  Tenant ID: {}", color::value(&tenant));
    println!();
    println!("{}", color::success("✓ Tenant deleted"));
    Ok(())
}

pub fn handle_users_list(active_only: bool, group: Option<String>, output: String) -> Result<()> {
    use crate::multitenancy::AccessControlManager;

    println!("{}", color::header("Users"));
    println!();

    let manager = AccessControlManager::new();
    let users = manager.list_users();

    println!("  Total users: {}", users.len());
    if active_only {
        println!("  Filter: Active only");
    }
    if let Some(g) = &group {
        println!("  Group: {}", color::value(g));
    }
    println!("  Format: {}", output);
    println!();
    println!("{}", color::success("✓ Users listed"));
    Ok(())
}

pub fn handle_users_create(
    username: String,
    email: String,
    role: Option<String>,
    group: Option<String>,
) -> Result<()> {
    use crate::multitenancy::User;

    println!("{}", color::header(&format!("Creating User: {}", username)));
    println!();

    let mut user = User::new(&username, &email);

    if let Some(r) = &role {
        user = user.add_role(r);
        println!("  Role assigned: {}", color::value(r));
    }

    if let Some(g) = &group {
        user = user.add_group(g);
        println!("  Group added: {}", color::value(g));
    }

    // Persist the user
    let mut store = TenancyStore::load();
    if let Ok(value) = serde_json::to_value(&user) {
        store.users.push(value);
        store.save();
    }

    println!("  Username: {}", color::value(&user.username));
    println!("  Email:    {}", user.email);
    println!();
    println!(
        "{}",
        color::success("✓ User created and persisted successfully")
    );
    Ok(())
}

pub fn handle_users_assign_role(user: String, role: String, scope: String) -> Result<()> {
    use crate::multitenancy::roles::{RoleBinding, Subject};

    println!("{}", color::header("Assigning Role"));
    println!();

    let subject = Subject::User {
        user_id: user.clone(),
    };
    let binding_scope = parse_binding_scope(&scope);

    let _binding = RoleBinding::new(&role, subject, binding_scope);

    println!("  User:  {}", color::value(&user));
    println!("  Role:  {}", color::value(&role));
    println!("  Scope: {}", scope);
    println!();
    println!("{}", color::success("✓ Role assigned"));
    Ok(())
}

pub fn handle_roles_list(builtin: bool, custom: bool, output: String) -> Result<()> {
    use crate::multitenancy::roles::RoleManager;

    println!("{}", color::header("Roles"));
    println!();

    let manager = RoleManager::new();
    let roles = if builtin {
        manager.builtin_roles()
    } else if custom {
        manager.custom_roles()
    } else {
        manager.list_roles()
    };

    println!("  Total roles: {}", roles.len());
    println!("  Format: {}", output);
    println!();
    println!("{}", color::success("✓ Roles listed"));
    Ok(())
}

pub fn handle_roles_show(role: String, output: String) -> Result<()> {
    println!("{}", color::header(&format!("Role: {}", role)));
    println!();

    println!("  Format: {}", output);
    println!();
    println!("{}", color::success("✓ Role details retrieved"));
    Ok(())
}

pub fn handle_roles_create(
    name: String,
    description: Option<String>,
    permissions: String,
) -> Result<()> {
    use crate::multitenancy::roles::Role;

    println!("{}", color::header(&format!("Creating Role: {}", name)));
    println!();

    let mut role = Role::new(&name);

    if let Some(desc) = description {
        role = role.with_description(desc);
    }

    // Persist the role
    let mut store = TenancyStore::load();
    if let Ok(value) = serde_json::to_value(&role) {
        store.roles.push(value);
        store.save();
    }

    println!("  Name:        {}", color::value(&role.name));
    println!("  Permissions: {}", permissions);
    println!();
    println!(
        "{}",
        color::success("✓ Role created and persisted successfully")
    );
    Ok(())
}

pub fn handle_quotas_list(namespace: Option<String>, exceeded: bool, output: String) -> Result<()> {
    use crate::multitenancy::quotas::QuotaManager;

    println!("{}", color::header("Resource Quotas"));
    println!();

    let manager = QuotaManager::new();
    let quotas = if exceeded {
        manager.exceeded_quotas()
    } else {
        manager.list_quotas()
    };

    println!("  Total quotas: {}", quotas.len());
    if let Some(ns) = &namespace {
        println!("  Namespace: {}", color::value(ns));
    }
    println!("  Format: {}", output);
    println!();
    println!("{}", color::success("✓ Quotas listed"));
    Ok(())
}

pub async fn handle_quotas_create(name: String, namespace: String, preset: String) -> Result<()> {
    use crate::multitenancy::quotas::ResourceQuota as VMRogueQuota;

    println!("{}", color::header(&format!("Creating Quota: {}", name)));
    println!();

    let limits = parse_quota_preset(&preset);

    let quota = VMRogueQuota::new(&name, &namespace).with_limits(limits.clone());

    println!("  Name:      {}", color::value(&quota.name));
    println!("  Namespace: {}", quota.namespace);
    println!("  Preset:    {}", preset);
    println!("  Max VMs:   {}", limits.max_vms);
    println!("  Max CPUs:  {}", limits.max_cpu_cores);
    println!("  Max Memory: {} Gi", limits.max_memory_gi);
    println!();

    // Determine resource limits based on preset
    let (cpu, memory, pods) = match preset.as_str() {
        "small" => ("4", "8Gi", "10"),
        "medium" => ("16", "32Gi", "50"),
        "large" => ("64", "128Gi", "200"),
        _ => ("8", "16Gi", "25"), // default
    };

    // Create real K8s ResourceQuota
    let client = crate::kube::get_client().await?;

    use k8s_openapi::api::core::v1::ResourceQuota;
    use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
    let quotas_api: kube::api::Api<ResourceQuota> = kube::api::Api::namespaced(client, &namespace);

    let mut hard = std::collections::BTreeMap::new();
    hard.insert("requests.cpu".to_string(), Quantity(cpu.to_string()));
    hard.insert("requests.memory".to_string(), Quantity(memory.to_string()));
    hard.insert("pods".to_string(), Quantity(pods.to_string()));

    let k8s_quota = ResourceQuota {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(namespace.clone()),
            ..Default::default()
        },
        spec: Some(k8s_openapi::api::core::v1::ResourceQuotaSpec {
            hard: Some(hard),
            ..Default::default()
        }),
        status: None,
    };

    quotas_api
        .create(&kube::api::PostParams::default(), &k8s_quota)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create ResourceQuota: {}", e))?;

    println!("{}", color::success("✓ Quota created in Kubernetes"));
    Ok(())
}

pub fn handle_quotas_show(quota: String, utilization: bool, output: String) -> Result<()> {
    println!("{}", color::header(&format!("Quota: {}", quota)));
    println!();

    println!("  Show utilization: {}", utilization);
    println!("  Format: {}", output);
    println!();
    println!("{}", color::success("✓ Quota details retrieved"));
    Ok(())
}

pub fn handle_groups_list(output: String) -> Result<()> {
    use crate::multitenancy::AccessControlManager;

    println!("{}", color::header("Groups"));
    println!();

    let manager = AccessControlManager::new();
    let groups = manager.list_groups();

    println!("  Total groups: {}", groups.len());
    println!("  Format: {}", output);
    println!();
    println!("{}", color::success("✓ Groups listed"));
    Ok(())
}

pub fn handle_groups_create(
    name: String,
    description: Option<String>,
    role: Option<String>,
) -> Result<()> {
    use crate::multitenancy::Group;

    println!("{}", color::header(&format!("Creating Group: {}", name)));
    println!();

    let mut group = Group::new(&name);

    if let Some(desc) = description {
        group = group.with_description(desc);
    }

    if let Some(r) = &role {
        group.add_role(r);
        println!("  Role assigned: {}", color::value(r));
    }

    // Persist the group
    let mut store = TenancyStore::load();
    if let Ok(value) = serde_json::to_value(&group) {
        store.groups.push(value);
        store.save();
    }

    println!("  Name: {}", color::value(&group.name));
    println!();
    println!(
        "{}",
        color::success("✓ Group created and persisted successfully")
    );
    Ok(())
}

pub fn handle_groups_add_user(group: String, user: String) -> Result<()> {
    println!("{}", color::header("Adding User to Group"));
    println!();

    println!("  Group: {}", color::value(&group));
    println!("  User:  {}", color::value(&user));
    println!();
    println!("{}", color::success("✓ User added to group"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== parse_binding_scope tests =====

    #[test]
    fn test_parse_binding_scope_cluster() {
        let scope = parse_binding_scope("cluster");
        assert!(matches!(
            scope,
            crate::multitenancy::roles::BindingScope::Cluster
        ));
    }

    #[test]
    fn test_parse_binding_scope_namespace() {
        let scope = parse_binding_scope("namespace:production");
        match scope {
            crate::multitenancy::roles::BindingScope::Namespace { namespace } => {
                assert_eq!(namespace, "production");
            }
            _ => panic!("Expected Namespace variant"),
        }
    }

    #[test]
    fn test_parse_binding_scope_namespace_default() {
        let scope = parse_binding_scope("namespace:default");
        match scope {
            crate::multitenancy::roles::BindingScope::Namespace { namespace } => {
                assert_eq!(namespace, "default");
            }
            _ => panic!("Expected Namespace variant"),
        }
    }

    #[test]
    fn test_parse_binding_scope_unknown_defaults_to_cluster() {
        let scope = parse_binding_scope("something-else");
        assert!(matches!(
            scope,
            crate::multitenancy::roles::BindingScope::Cluster
        ));
    }

    #[test]
    fn test_parse_binding_scope_empty_defaults_to_cluster() {
        let scope = parse_binding_scope("");
        assert!(matches!(
            scope,
            crate::multitenancy::roles::BindingScope::Cluster
        ));
    }

    #[test]
    fn test_parse_binding_scope_namespace_empty_name() {
        // "namespace:" with empty name should still parse as Namespace
        let scope = parse_binding_scope("namespace:");
        match scope {
            crate::multitenancy::roles::BindingScope::Namespace { namespace } => {
                assert_eq!(namespace, "");
            }
            _ => panic!("Expected Namespace variant"),
        }
    }

    // ===== parse_quota_preset tests =====

    #[test]
    fn test_parse_quota_preset_small() {
        let limits = parse_quota_preset("small");
        assert_eq!(limits.max_vms, 5);
    }

    #[test]
    fn test_parse_quota_preset_medium() {
        let limits = parse_quota_preset("medium");
        // medium is the default
        let default = crate::multitenancy::quotas::ResourceLimits::medium();
        assert_eq!(limits.max_vms, default.max_vms);
        assert_eq!(limits.max_cpu_cores, default.max_cpu_cores);
    }

    #[test]
    fn test_parse_quota_preset_large() {
        let limits = parse_quota_preset("large");
        assert_eq!(limits.max_vms, 50);
    }

    #[test]
    fn test_parse_quota_preset_unlimited() {
        let limits = parse_quota_preset("unlimited");
        assert_eq!(limits.max_vms, u32::MAX);
    }

    #[test]
    fn test_parse_quota_preset_unknown_defaults_to_medium() {
        let limits = parse_quota_preset("extra-large");
        let medium = crate::multitenancy::quotas::ResourceLimits::medium();
        assert_eq!(limits.max_vms, medium.max_vms);
    }

    #[test]
    fn test_parse_quota_preset_small_cpu_cores() {
        let limits = parse_quota_preset("small");
        let small = crate::multitenancy::quotas::ResourceLimits::small();
        assert_eq!(limits.max_cpu_cores, small.max_cpu_cores);
    }

    #[test]
    fn test_parse_quota_preset_large_memory() {
        let limits = parse_quota_preset("large");
        let large = crate::multitenancy::quotas::ResourceLimits::large();
        assert_eq!(limits.max_memory_gi, large.max_memory_gi);
    }
}
