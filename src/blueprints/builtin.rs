// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Built-in Blueprint Definitions
// These are the default multi-VM blueprints shipped with VMRogue

use super::{Blueprint, VMSpec};
use std::collections::HashMap;

/// Get all built-in blueprints
pub fn builtin_blueprints() -> HashMap<String, Blueprint> {
    let mut blueprints = HashMap::new();

    // LAMP Stack Blueprint
    blueprints.insert(
        "lamp".to_string(),
        Blueprint {
            name: "lamp".to_string(),
            description: "LAMP Stack (Linux + Apache + MySQL + PHP)".to_string(),
            vms: vec![
                VMSpec {
                    name: "mysql-db".to_string(),
                    template: "ubuntu".to_string(),
                    profile: Some("database".to_string()),
                    cpu: None,
                    memory: None,
                    disk_size: Some("100Gi".to_string()),
                    depends_on: vec![],
                    labels: HashMap::from([
                        ("tier".to_string(), "database".to_string()),
                        ("app".to_string(), "lamp".to_string()),
                    ]),
                },
                VMSpec {
                    name: "web-server".to_string(),
                    template: "ubuntu".to_string(),
                    profile: Some("web".to_string()),
                    cpu: None,
                    memory: None,
                    disk_size: None,
                    depends_on: vec!["mysql-db".to_string()],
                    labels: HashMap::from([
                        ("tier".to_string(), "frontend".to_string()),
                        ("app".to_string(), "lamp".to_string()),
                    ]),
                },
            ],
            tags: vec![
                "web".to_string(),
                "database".to_string(),
                "classic".to_string(),
            ],
        },
    );

    // Kubernetes Cluster Blueprint
    blueprints.insert(
        "k8s-cluster".to_string(),
        Blueprint {
            name: "k8s-cluster".to_string(),
            description: "Kubernetes cluster (1 control plane + 2 workers)".to_string(),
            vms: vec![
                VMSpec {
                    name: "k8s-control".to_string(),
                    template: "ubuntu-22.04".to_string(),
                    profile: Some("prod".to_string()),
                    cpu: Some(4),
                    memory: Some("8Gi".to_string()),
                    disk_size: Some("50Gi".to_string()),
                    depends_on: vec![],
                    labels: HashMap::from([
                        ("role".to_string(), "control-plane".to_string()),
                        ("cluster".to_string(), "k8s".to_string()),
                    ]),
                },
                VMSpec {
                    name: "k8s-worker-1".to_string(),
                    template: "ubuntu-22.04".to_string(),
                    profile: Some("prod".to_string()),
                    cpu: Some(4),
                    memory: Some("16Gi".to_string()),
                    disk_size: Some("100Gi".to_string()),
                    depends_on: vec!["k8s-control".to_string()],
                    labels: HashMap::from([
                        ("role".to_string(), "worker".to_string()),
                        ("cluster".to_string(), "k8s".to_string()),
                    ]),
                },
                VMSpec {
                    name: "k8s-worker-2".to_string(),
                    template: "ubuntu-22.04".to_string(),
                    profile: Some("prod".to_string()),
                    cpu: Some(4),
                    memory: Some("16Gi".to_string()),
                    disk_size: Some("100Gi".to_string()),
                    depends_on: vec!["k8s-control".to_string()],
                    labels: HashMap::from([
                        ("role".to_string(), "worker".to_string()),
                        ("cluster".to_string(), "k8s".to_string()),
                    ]),
                },
            ],
            tags: vec![
                "kubernetes".to_string(),
                "cluster".to_string(),
                "container".to_string(),
            ],
        },
    );

    // 3-Tier Web App Blueprint
    blueprints.insert(
        "3tier".to_string(),
        Blueprint {
            name: "3tier".to_string(),
            description: "3-tier web application (Web + App + Database)".to_string(),
            vms: vec![
                VMSpec {
                    name: "postgres-db".to_string(),
                    template: "almalinux".to_string(),
                    profile: Some("database".to_string()),
                    cpu: None,
                    memory: Some("16Gi".to_string()),
                    disk_size: Some("200Gi".to_string()),
                    depends_on: vec![],
                    labels: HashMap::from([
                        ("tier".to_string(), "database".to_string()),
                        ("app".to_string(), "3tier".to_string()),
                    ]),
                },
                VMSpec {
                    name: "app-server".to_string(),
                    template: "ubuntu".to_string(),
                    profile: Some("prod".to_string()),
                    cpu: Some(4),
                    memory: Some("8Gi".to_string()),
                    disk_size: Some("40Gi".to_string()),
                    depends_on: vec!["postgres-db".to_string()],
                    labels: HashMap::from([
                        ("tier".to_string(), "application".to_string()),
                        ("app".to_string(), "3tier".to_string()),
                    ]),
                },
                VMSpec {
                    name: "nginx-lb".to_string(),
                    template: "alpine".to_string(),
                    profile: Some("web".to_string()),
                    cpu: Some(2),
                    memory: Some("4Gi".to_string()),
                    disk_size: Some("20Gi".to_string()),
                    depends_on: vec!["app-server".to_string()],
                    labels: HashMap::from([
                        ("tier".to_string(), "frontend".to_string()),
                        ("app".to_string(), "3tier".to_string()),
                    ]),
                },
            ],
            tags: vec![
                "web".to_string(),
                "microservices".to_string(),
                "production".to_string(),
            ],
        },
    );

    // Development Stack Blueprint
    blueprints.insert(
        "dev-stack".to_string(),
        Blueprint {
            name: "dev-stack".to_string(),
            description: "Development stack (Code server + DB + Redis)".to_string(),
            vms: vec![
                VMSpec {
                    name: "dev-db".to_string(),
                    template: "ubuntu".to_string(),
                    profile: Some("test".to_string()),
                    cpu: Some(2),
                    memory: Some("4Gi".to_string()),
                    disk_size: Some("20Gi".to_string()),
                    depends_on: vec![],
                    labels: HashMap::from([
                        ("env".to_string(), "development".to_string()),
                        ("type".to_string(), "database".to_string()),
                    ]),
                },
                VMSpec {
                    name: "dev-cache".to_string(),
                    template: "alpine".to_string(),
                    profile: Some("minimal".to_string()),
                    cpu: Some(1),
                    memory: Some("2Gi".to_string()),
                    disk_size: Some("10Gi".to_string()),
                    depends_on: vec![],
                    labels: HashMap::from([
                        ("env".to_string(), "development".to_string()),
                        ("type".to_string(), "cache".to_string()),
                    ]),
                },
                VMSpec {
                    name: "dev-workspace".to_string(),
                    template: "ubuntu".to_string(),
                    profile: Some("dev".to_string()),
                    cpu: Some(4),
                    memory: Some("8Gi".to_string()),
                    disk_size: Some("40Gi".to_string()),
                    depends_on: vec!["dev-db".to_string(), "dev-cache".to_string()],
                    labels: HashMap::from([
                        ("env".to_string(), "development".to_string()),
                        ("type".to_string(), "workspace".to_string()),
                    ]),
                },
            ],
            tags: vec!["development".to_string(), "testing".to_string()],
        },
    );

    // CI/CD Pipeline Blueprint
    blueprints.insert(
        "cicd".to_string(),
        Blueprint {
            name: "cicd".to_string(),
            description: "CI/CD Pipeline (Jenkins + GitLab + Artifact Registry)".to_string(),
            vms: vec![
                VMSpec {
                    name: "gitlab-server".to_string(),
                    template: "ubuntu-22.04".to_string(),
                    profile: Some("prod".to_string()),
                    cpu: Some(4),
                    memory: Some("16Gi".to_string()),
                    disk_size: Some("100Gi".to_string()),
                    depends_on: vec![],
                    labels: HashMap::from([
                        ("component".to_string(), "scm".to_string()),
                        ("pipeline".to_string(), "cicd".to_string()),
                    ]),
                },
                VMSpec {
                    name: "jenkins-server".to_string(),
                    template: "ubuntu-22.04".to_string(),
                    profile: Some("prod".to_string()),
                    cpu: Some(4),
                    memory: Some("8Gi".to_string()),
                    disk_size: Some("80Gi".to_string()),
                    depends_on: vec!["gitlab-server".to_string()],
                    labels: HashMap::from([
                        ("component".to_string(), "ci".to_string()),
                        ("pipeline".to_string(), "cicd".to_string()),
                    ]),
                },
                VMSpec {
                    name: "artifact-registry".to_string(),
                    template: "almalinux".to_string(),
                    profile: Some("prod".to_string()),
                    cpu: Some(2),
                    memory: Some("8Gi".to_string()),
                    disk_size: Some("200Gi".to_string()),
                    depends_on: vec![],
                    labels: HashMap::from([
                        ("component".to_string(), "registry".to_string()),
                        ("pipeline".to_string(), "cicd".to_string()),
                    ]),
                },
            ],
            tags: vec![
                "cicd".to_string(),
                "automation".to_string(),
                "devops".to_string(),
            ],
        },
    );

    blueprints
}
