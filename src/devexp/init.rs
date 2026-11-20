// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Project initialization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInit {
    pub project_name: String,
    pub project_type: ProjectType,
    pub directory: String,
    pub namespace: String,
    pub include_examples: bool,
    pub include_ci: bool,
    pub git_init: bool,
    pub created_at: String,
}

/// Type of project to initialize
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectType {
    Basic,
    Development,
    Production,
    Microservices,
    DataPipeline,
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::Basic => write!(f, "basic"),
            ProjectType::Development => write!(f, "development"),
            ProjectType::Production => write!(f, "production"),
            ProjectType::Microservices => write!(f, "microservices"),
            ProjectType::DataPipeline => write!(f, "data-pipeline"),
        }
    }
}

impl ProjectType {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "basic" => Some(ProjectType::Basic),
            "development" | "dev" => Some(ProjectType::Development),
            "production" | "prod" => Some(ProjectType::Production),
            "microservices" | "micro" => Some(ProjectType::Microservices),
            "data-pipeline" | "data" | "pipeline" => Some(ProjectType::DataPipeline),
            _ => None,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            ProjectType::Basic => "Basic VM setup with minimal configuration",
            ProjectType::Development => "Development environment with debug tools",
            ProjectType::Production => "Production-ready with HA and monitoring",
            ProjectType::Microservices => "Multi-VM microservices architecture",
            ProjectType::DataPipeline => "Data processing pipeline setup",
        }
    }
}

impl ProjectInit {
    pub fn new(project_name: impl Into<String>, project_type: ProjectType) -> Self {
        let name = project_name.into();
        Self {
            directory: format!("./{}", name),
            project_name: name,
            project_type,
            namespace: "default".to_string(),
            include_examples: true,
            include_ci: false,
            git_init: true,
            created_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn with_directory(mut self, directory: impl Into<String>) -> Self {
        self.directory = directory.into();
        self
    }

    pub fn with_examples(mut self, include: bool) -> Self {
        self.include_examples = include;
        self
    }

    pub fn with_ci(mut self, include: bool) -> Self {
        self.include_ci = include;
        self
    }

    pub fn with_git(mut self, init: bool) -> Self {
        self.git_init = init;
        self
    }

    /// Generate the default VM configuration for this project type
    pub fn generate_default_config(&self) -> String {
        match self.project_type {
            ProjectType::Basic => format!(
                r#"# Veyron VM Configuration
# Project: {name}
# Type: Basic

name: {name}-vm
namespace: {ns}

cpu:
  cores: 2
  sockets: 1
  threads: 1

memory:
  size: 4Gi

disks:
  - name: rootdisk
    size: 20Gi
    boot_order: 1

networks:
  - name: default
    type: pod
"#,
                name = self.project_name,
                ns = self.namespace,
            ),
            ProjectType::Development => format!(
                r#"# Veyron VM Configuration
# Project: {name}
# Type: Development

name: {name}-dev
namespace: {ns}

cpu:
  cores: 4
  sockets: 1
  threads: 1

memory:
  size: 8Gi

disks:
  - name: rootdisk
    size: 40Gi
    boot_order: 1
  - name: datadisk
    size: 20Gi
    boot_order: 2

networks:
  - name: default
    type: pod

labels:
  environment: development
  managed-by: veyron
"#,
                name = self.project_name,
                ns = self.namespace,
            ),
            ProjectType::Production => format!(
                r#"# Veyron VM Configuration
# Project: {name}
# Type: Production

name: {name}-prod
namespace: {ns}

cpu:
  cores: 8
  sockets: 2
  threads: 1

memory:
  size: 16Gi

disks:
  - name: rootdisk
    size: 50Gi
    boot_order: 1
    storage_class: fast-ssd
  - name: datadisk
    size: 100Gi
    boot_order: 2
    storage_class: fast-ssd

networks:
  - name: default
    type: pod
  - name: internal
    type: multus

labels:
  environment: production
  managed-by: veyron
  ha-enabled: "true"
"#,
                name = self.project_name,
                ns = self.namespace,
            ),
            ProjectType::Microservices => format!(
                r#"# Veyron VM Configuration
# Project: {name}
# Type: Microservices

name: {name}-svc
namespace: {ns}

cpu:
  cores: 2
  sockets: 1
  threads: 1

memory:
  size: 4Gi

disks:
  - name: rootdisk
    size: 20Gi
    boot_order: 1

networks:
  - name: default
    type: pod
  - name: service-mesh
    type: multus

labels:
  environment: production
  managed-by: veyron
  service-mesh: enabled
"#,
                name = self.project_name,
                ns = self.namespace,
            ),
            ProjectType::DataPipeline => format!(
                r#"# Veyron VM Configuration
# Project: {name}
# Type: Data Pipeline

name: {name}-data
namespace: {ns}

cpu:
  cores: 8
  sockets: 1
  threads: 2

memory:
  size: 32Gi

disks:
  - name: rootdisk
    size: 50Gi
    boot_order: 1
  - name: datadisk
    size: 500Gi
    boot_order: 2
    storage_class: high-throughput

networks:
  - name: default
    type: pod

labels:
  environment: production
  managed-by: veyron
  workload-type: data-pipeline
"#,
                name = self.project_name,
                ns = self.namespace,
            ),
        }
    }

    /// Generate gitignore content for veyron project
    pub fn generate_gitignore(&self) -> String {
        r#"# Veyron project gitignore
*.secret
*.key
*.pem
.env
.env.*
kubeconfig
*.backup

# OS files
.DS_Store
Thumbs.db

# IDE files
.vscode/
.idea/
*.swp
*.swo
"#
        .to_string()
    }

    /// Get the list of files that would be created
    pub fn file_list(&self) -> Vec<String> {
        let mut files = vec![format!("{}/veyron.yaml", self.directory)];

        if self.git_init {
            files.push(format!("{}/.gitignore", self.directory));
        }

        if self.include_examples {
            files.push(format!("{}/examples/basic-vm.yaml", self.directory));
            files.push(format!("{}/examples/multi-vm.yaml", self.directory));
        }

        if self.include_ci {
            files.push(format!("{}/.github/workflows/veyron.yaml", self.directory));
        }

        files
    }

    pub fn file_count(&self) -> usize {
        self.file_list().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_init_new() {
        let init = ProjectInit::new("my-project", ProjectType::Basic);
        assert_eq!(init.project_name, "my-project");
        assert_eq!(init.project_type, ProjectType::Basic);
        assert_eq!(init.namespace, "default");
        assert!(init.include_examples);
        assert!(!init.include_ci);
        assert!(init.git_init);
    }

    #[test]
    fn test_project_init_with_namespace() {
        let init = ProjectInit::new("test", ProjectType::Production).with_namespace("production");
        assert_eq!(init.namespace, "production");
    }

    #[test]
    fn test_project_init_with_directory() {
        let init = ProjectInit::new("test", ProjectType::Basic).with_directory("/custom/path");
        assert_eq!(init.directory, "/custom/path");
    }

    #[test]
    fn test_project_init_with_examples() {
        let init = ProjectInit::new("test", ProjectType::Basic).with_examples(false);
        assert!(!init.include_examples);
    }

    #[test]
    fn test_project_init_with_ci() {
        let init = ProjectInit::new("test", ProjectType::Basic).with_ci(true);
        assert!(init.include_ci);
    }

    #[test]
    fn test_project_init_with_git() {
        let init = ProjectInit::new("test", ProjectType::Basic).with_git(false);
        assert!(!init.git_init);
    }

    #[test]
    fn test_project_type_from_str() {
        assert_eq!(ProjectType::parse("basic"), Some(ProjectType::Basic));
        assert_eq!(ProjectType::parse("dev"), Some(ProjectType::Development));
        assert_eq!(
            ProjectType::parse("development"),
            Some(ProjectType::Development)
        );
        assert_eq!(ProjectType::parse("prod"), Some(ProjectType::Production));
        assert_eq!(
            ProjectType::parse("production"),
            Some(ProjectType::Production)
        );
        assert_eq!(
            ProjectType::parse("micro"),
            Some(ProjectType::Microservices)
        );
        assert_eq!(ProjectType::parse("data"), Some(ProjectType::DataPipeline));
        assert_eq!(
            ProjectType::parse("pipeline"),
            Some(ProjectType::DataPipeline)
        );
        assert_eq!(ProjectType::parse("unknown"), None);
    }

    #[test]
    fn test_project_type_display() {
        assert_eq!(ProjectType::Basic.to_string(), "basic");
        assert_eq!(ProjectType::Development.to_string(), "development");
        assert_eq!(ProjectType::Production.to_string(), "production");
        assert_eq!(ProjectType::Microservices.to_string(), "microservices");
        assert_eq!(ProjectType::DataPipeline.to_string(), "data-pipeline");
    }

    #[test]
    fn test_project_type_description() {
        assert!(!ProjectType::Basic.description().is_empty());
        assert!(!ProjectType::Development.description().is_empty());
        assert!(!ProjectType::Production.description().is_empty());
        assert!(!ProjectType::Microservices.description().is_empty());
        assert!(!ProjectType::DataPipeline.description().is_empty());
    }

    #[test]
    fn test_generate_default_config_basic() {
        let init = ProjectInit::new("test-app", ProjectType::Basic);
        let config = init.generate_default_config();
        assert!(config.contains("test-app-vm"));
        assert!(config.contains("cores: 2"));
        assert!(config.contains("size: 4Gi"));
    }

    #[test]
    fn test_generate_default_config_development() {
        let init = ProjectInit::new("test-app", ProjectType::Development);
        let config = init.generate_default_config();
        assert!(config.contains("test-app-dev"));
        assert!(config.contains("cores: 4"));
        assert!(config.contains("environment: development"));
    }

    #[test]
    fn test_generate_default_config_production() {
        let init = ProjectInit::new("test-app", ProjectType::Production);
        let config = init.generate_default_config();
        assert!(config.contains("test-app-prod"));
        assert!(config.contains("cores: 8"));
        assert!(config.contains("ha-enabled"));
    }

    #[test]
    fn test_generate_default_config_microservices() {
        let init = ProjectInit::new("test-app", ProjectType::Microservices);
        let config = init.generate_default_config();
        assert!(config.contains("test-app-svc"));
        assert!(config.contains("service-mesh"));
    }

    #[test]
    fn test_generate_default_config_data_pipeline() {
        let init = ProjectInit::new("test-app", ProjectType::DataPipeline);
        let config = init.generate_default_config();
        assert!(config.contains("test-app-data"));
        assert!(config.contains("32Gi"));
        assert!(config.contains("workload-type: data-pipeline"));
    }

    #[test]
    fn test_generate_gitignore() {
        let init = ProjectInit::new("test", ProjectType::Basic);
        let gitignore = init.generate_gitignore();
        assert!(gitignore.contains("*.secret"));
        assert!(gitignore.contains("*.key"));
        assert!(gitignore.contains(".env"));
    }

    #[test]
    fn test_file_list_basic() {
        let init = ProjectInit::new("test", ProjectType::Basic);
        let files = init.file_list();
        assert!(files.iter().any(|f| f.contains("veyron.yaml")));
        assert!(files.iter().any(|f| f.contains(".gitignore")));
    }

    #[test]
    fn test_file_list_with_examples() {
        let init = ProjectInit::new("test", ProjectType::Basic).with_examples(true);
        let files = init.file_list();
        assert!(files.iter().any(|f| f.contains("examples/")));
    }

    #[test]
    fn test_file_list_without_git() {
        let init = ProjectInit::new("test", ProjectType::Basic).with_git(false);
        let files = init.file_list();
        assert!(!files.iter().any(|f| f.contains(".gitignore")));
    }

    #[test]
    fn test_file_list_with_ci() {
        let init = ProjectInit::new("test", ProjectType::Basic).with_ci(true);
        let files = init.file_list();
        assert!(files.iter().any(|f| f.contains(".github")));
    }

    #[test]
    fn test_file_count() {
        let init = ProjectInit::new("test", ProjectType::Basic);
        assert!(init.file_count() > 0);
    }
}
