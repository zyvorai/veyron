// RBAC Visualizer - Visual RBAC analysis and role mapping

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacVisualizer {
    pub roles: Vec<RbacRole>,
    pub bindings: Vec<RoleBinding>,
    pub subjects: Vec<RbacSubject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacRole {
    pub name: String,
    pub namespace: Option<String>,
    pub is_cluster_role: bool,
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub api_groups: Vec<String>,
    pub resources: Vec<String>,
    pub verbs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleBinding {
    pub name: String,
    pub namespace: Option<String>,
    pub role_ref: String,
    pub subjects: Vec<String>,
    pub is_cluster_binding: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacSubject {
    pub name: String,
    pub kind: SubjectKind,
    pub namespace: Option<String>,
    pub roles: Vec<String>,
    pub effective_permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubjectKind {
    User,
    Group,
    ServiceAccount,
}

impl RbacVisualizer {
    pub fn new() -> Self {
        Self {
            roles: Vec::new(),
            bindings: Vec::new(),
            subjects: Vec::new(),
        }
    }

    pub fn add_role(&mut self, role: RbacRole) {
        self.roles.push(role);
    }
    pub fn add_binding(&mut self, binding: RoleBinding) {
        self.bindings.push(binding);
    }
    pub fn add_subject(&mut self, subject: RbacSubject) {
        self.subjects.push(subject);
    }

    pub fn get_role(&self, name: &str) -> Option<&RbacRole> {
        self.roles.iter().find(|r| r.name == name)
    }

    pub fn roles_for_subject(&self, subject: &str) -> Vec<&RbacRole> {
        let role_names: Vec<&str> = self
            .bindings
            .iter()
            .filter(|b| b.subjects.iter().any(|s| s == subject))
            .map(|b| b.role_ref.as_str())
            .collect();
        self.roles
            .iter()
            .filter(|r| role_names.contains(&r.name.as_str()))
            .collect()
    }

    pub fn subjects_with_role(&self, role: &str) -> Vec<&str> {
        self.bindings
            .iter()
            .filter(|b| b.role_ref == role)
            .flat_map(|b| b.subjects.iter().map(|s| s.as_str()))
            .collect()
    }

    pub fn permission_matrix(&self) -> HashMap<String, Vec<String>> {
        let mut matrix = HashMap::new();
        for subject in &self.subjects {
            let perms: Vec<String> = self
                .roles_for_subject(&subject.name)
                .iter()
                .flat_map(|r| r.rules.iter())
                .flat_map(|rule| {
                    rule.verbs
                        .iter()
                        .flat_map(|v| rule.resources.iter().map(move |r| format!("{}/{}", v, r)))
                })
                .collect();
            matrix.insert(subject.name.clone(), perms);
        }
        matrix
    }

    pub fn overprivileged_subjects(&self) -> Vec<&RbacSubject> {
        self.subjects
            .iter()
            .filter(|s| {
                self.roles_for_subject(&s.name).iter().any(|r| {
                    r.rules
                        .iter()
                        .any(|rule| rule.verbs.contains(&"*".to_string()))
                })
            })
            .collect()
    }
}

impl Default for RbacVisualizer {
    fn default() -> Self {
        Self::new()
    }
}
