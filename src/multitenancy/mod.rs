// Multi-tenancy & RBAC - Role-based access control and tenant isolation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub mod permissions;
pub mod quotas;
pub mod roles;
pub mod tenants;

/// User identity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub groups: Vec<String>,
    pub roles: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub active: bool,
}

impl User {
    pub fn new(username: impl Into<String>, email: impl Into<String>) -> Self {
        let username_str = username.into();
        let id = format!("user-{}", username_str.to_lowercase());

        Self {
            id,
            username: username_str,
            email: email.into(),
            groups: Vec::new(),
            roles: Vec::new(),
            created_at: Utc::now(),
            last_login: None,
            active: true,
        }
    }

    pub fn add_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    pub fn add_group(mut self, group: impl Into<String>) -> Self {
        self.groups.push(group.into());
        self
    }

    pub fn deactivate(mut self) -> Self {
        self.active = false;
        self
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    pub fn in_group(&self, group: &str) -> bool {
        self.groups.iter().any(|g| g == group)
    }

    pub fn record_login(&mut self) {
        self.last_login = Some(Utc::now());
    }
}

/// Group of users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: String,
    pub members: HashSet<String>, // User IDs
    pub roles: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl Group {
    pub fn new(name: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!("group-{}", name_str.to_lowercase().replace(' ', "-"));

        Self {
            id,
            name: name_str,
            description: String::new(),
            members: HashSet::new(),
            roles: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn add_member(&mut self, user_id: impl Into<String>) {
        self.members.insert(user_id.into());
    }

    pub fn remove_member(&mut self, user_id: &str) {
        self.members.remove(user_id);
    }

    pub fn add_role(&mut self, role: impl Into<String>) {
        let role = role.into();
        if !self.roles.contains(&role) {
            self.roles.push(role);
        }
    }

    pub fn has_member(&self, user_id: &str) -> bool {
        self.members.contains(user_id)
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

/// Resource scope
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceScope {
    Cluster,                                          // Cluster-wide
    Namespace { namespace: String },                  // Specific namespace
    Resource { namespace: String, resource: String }, // Specific resource
}

/// Access decision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccessDecision {
    Allow,
    Deny { reason: String },
}

impl AccessDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, AccessDecision::Allow)
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        AccessDecision::Deny {
            reason: reason.into(),
        }
    }
}

/// Authorization context
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user: User,
    pub action: String,
    pub resource_type: String,
    pub resource_scope: ResourceScope,
    pub metadata: HashMap<String, String>,
}

impl AuthContext {
    pub fn new(
        user: User,
        action: impl Into<String>,
        resource_type: impl Into<String>,
        resource_scope: ResourceScope,
    ) -> Self {
        Self {
            user,
            action: action.into(),
            resource_type: resource_type.into(),
            resource_scope,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Access control manager
pub struct AccessControlManager {
    users: HashMap<String, User>,
    groups: HashMap<String, Group>,
}

impl AccessControlManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            groups: HashMap::new(),
        }
    }

    pub fn add_user(&mut self, user: User) {
        self.users.insert(user.id.clone(), user);
    }

    pub fn get_user(&self, user_id: &str) -> Option<&User> {
        self.users.get(user_id)
    }

    pub fn get_user_mut(&mut self, user_id: &str) -> Option<&mut User> {
        self.users.get_mut(user_id)
    }

    pub fn remove_user(&mut self, user_id: &str) {
        self.users.remove(user_id);
    }

    pub fn add_group(&mut self, group: Group) {
        self.groups.insert(group.id.clone(), group);
    }

    pub fn get_group(&self, group_id: &str) -> Option<&Group> {
        self.groups.get(group_id)
    }

    pub fn get_group_mut(&mut self, group_id: &str) -> Option<&mut Group> {
        self.groups.get_mut(group_id)
    }

    pub fn remove_group(&mut self, group_id: &str) {
        self.groups.remove(group_id);
    }

    pub fn list_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }

    pub fn list_groups(&self) -> Vec<&Group> {
        self.groups.values().collect()
    }

    pub fn active_user_count(&self) -> usize {
        self.users.values().filter(|u| u.active).count()
    }

    /// Get all roles for a user (including from groups)
    pub fn get_user_roles(&self, user_id: &str) -> HashSet<String> {
        let mut roles = HashSet::new();

        if let Some(user) = self.users.get(user_id) {
            // Add direct roles
            for role in &user.roles {
                roles.insert(role.clone());
            }

            // Add roles from groups
            for group_id in &user.groups {
                if let Some(group) = self.groups.get(group_id) {
                    for role in &group.roles {
                        roles.insert(role.clone());
                    }
                }
            }
        }

        roles
    }

    /// Add user to group
    pub fn add_user_to_group(&mut self, user_id: &str, group_id: &str) {
        if let Some(group) = self.groups.get_mut(group_id) {
            group.add_member(user_id);
        }

        if let Some(user) = self.users.get_mut(user_id) {
            if !user.groups.contains(&group_id.to_string()) {
                user.groups.push(group_id.to_string());
            }
        }
    }

    /// Remove user from group
    pub fn remove_user_from_group(&mut self, user_id: &str, group_id: &str) {
        if let Some(group) = self.groups.get_mut(group_id) {
            group.remove_member(user_id);
        }

        if let Some(user) = self.users.get_mut(user_id) {
            user.groups.retain(|g| g != group_id);
        }
    }
}

impl Default for AccessControlManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new("john.doe", "john@example.com")
            .add_role("developer")
            .add_group("engineering");

        assert_eq!(user.username, "john.doe");
        assert_eq!(user.email, "john@example.com");
        assert!(user.has_role("developer"));
        assert!(user.in_group("engineering"));
        assert!(user.active);
    }

    #[test]
    fn test_user_deactivation() {
        let user = User::new("test", "test@example.com").deactivate();

        assert!(!user.active);
    }

    #[test]
    fn test_user_login_tracking() {
        let mut user = User::new("test", "test@example.com");
        assert!(user.last_login.is_none());

        user.record_login();
        assert!(user.last_login.is_some());
    }

    #[test]
    fn test_group_creation() {
        let group = Group::new("Engineering Team").with_description("Software engineering team");

        assert_eq!(group.name, "Engineering Team");
        assert!(!group.description.is_empty());
    }

    #[test]
    fn test_group_members() {
        let mut group = Group::new("Test Group");

        group.add_member("user-1");
        group.add_member("user-2");

        assert_eq!(group.member_count(), 2);
        assert!(group.has_member("user-1"));
        assert!(group.has_member("user-2"));

        group.remove_member("user-1");
        assert_eq!(group.member_count(), 1);
        assert!(!group.has_member("user-1"));
    }

    #[test]
    fn test_resource_scope() {
        let cluster = ResourceScope::Cluster;
        assert_eq!(cluster, ResourceScope::Cluster);

        let ns = ResourceScope::Namespace {
            namespace: "default".to_string(),
        };
        assert!(matches!(ns, ResourceScope::Namespace { .. }));
    }

    #[test]
    fn test_access_decision() {
        let allow = AccessDecision::Allow;
        assert!(allow.is_allowed());

        let deny = AccessDecision::deny("Insufficient permissions");
        assert!(!deny.is_allowed());
    }

    #[test]
    fn test_auth_context() {
        let user = User::new("test", "test@example.com");
        let context = AuthContext::new(
            user,
            "create",
            "vm",
            ResourceScope::Namespace {
                namespace: "default".to_string(),
            },
        )
        .with_metadata("team", "engineering");

        assert_eq!(context.action, "create");
        assert_eq!(context.resource_type, "vm");
        assert_eq!(
            context.metadata.get("team"),
            Some(&"engineering".to_string())
        );
    }

    #[test]
    fn test_access_control_manager() {
        let mut manager = AccessControlManager::new();

        let user = User::new("alice", "alice@example.com");
        manager.add_user(user);

        assert_eq!(manager.list_users().len(), 1);
        assert!(manager.get_user("user-alice").is_some());
    }

    #[test]
    fn test_user_group_association() {
        let mut manager = AccessControlManager::new();

        let user = User::new("bob", "bob@example.com");
        let group = Group::new("Developers");

        manager.add_user(user);
        manager.add_group(group);

        manager.add_user_to_group("user-bob", "group-developers");

        let user = manager.get_user("user-bob").unwrap();
        assert!(user.in_group("group-developers"));

        let group = manager.get_group("group-developers").unwrap();
        assert!(group.has_member("user-bob"));
    }

    #[test]
    fn test_remove_user_from_group() {
        let mut manager = AccessControlManager::new();

        let user = User::new("charlie", "charlie@example.com");
        let group = Group::new("Team A");

        manager.add_user(user);
        manager.add_group(group);

        manager.add_user_to_group("user-charlie", "group-team-a");
        manager.remove_user_from_group("user-charlie", "group-team-a");

        let user = manager.get_user("user-charlie").unwrap();
        assert!(!user.in_group("group-team-a"));
    }

    #[test]
    fn test_get_user_roles() {
        let mut manager = AccessControlManager::new();

        let user = User::new("dave", "dave@example.com").add_role("developer");

        let mut group = Group::new("Admins");
        group.add_role("admin".to_string());

        manager.add_user(user);
        manager.add_group(group);
        manager.add_user_to_group("user-dave", "group-admins");

        let roles = manager.get_user_roles("user-dave");
        assert!(roles.contains("developer"));
        assert!(roles.contains("admin"));
    }

    #[test]
    fn test_active_user_count() {
        let mut manager = AccessControlManager::new();

        manager.add_user(User::new("user1", "user1@example.com"));
        manager.add_user(User::new("user2", "user2@example.com"));
        manager.add_user(User::new("user3", "user3@example.com").deactivate());

        assert_eq!(manager.active_user_count(), 2);
    }

    #[test]
    fn test_remove_user() {
        let mut manager = AccessControlManager::new();

        manager.add_user(User::new("temp", "temp@example.com"));
        assert_eq!(manager.list_users().len(), 1);

        manager.remove_user("user-temp");
        assert_eq!(manager.list_users().len(), 0);
    }

    #[test]
    fn test_group_roles() {
        let mut group = Group::new("Test");

        group.add_role("viewer");
        group.add_role("editor");

        assert_eq!(group.roles.len(), 2);
        assert!(group.roles.contains(&"viewer".to_string()));
    }
}
