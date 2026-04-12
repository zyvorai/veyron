use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Saved configuration template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ConfigCategory,
    pub config_data: String,
    pub format: ConfigFormat,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub version: u32,
    pub usage_count: u64,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

/// Configuration category
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigCategory {
    Development,
    Testing,
    Staging,
    Production,
    Database,
    WebServer,
    CICDRunner,
    MachineLearning,
    Custom(String),
}

impl std::fmt::Display for ConfigCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigCategory::Development => write!(f, "development"),
            ConfigCategory::Testing => write!(f, "testing"),
            ConfigCategory::Staging => write!(f, "staging"),
            ConfigCategory::Production => write!(f, "production"),
            ConfigCategory::Database => write!(f, "database"),
            ConfigCategory::WebServer => write!(f, "web-server"),
            ConfigCategory::CICDRunner => write!(f, "cicd-runner"),
            ConfigCategory::MachineLearning => write!(f, "machine-learning"),
            ConfigCategory::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl ConfigCategory {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "development" | "dev" => ConfigCategory::Development,
            "testing" | "test" => ConfigCategory::Testing,
            "staging" => ConfigCategory::Staging,
            "production" | "prod" => ConfigCategory::Production,
            "database" | "db" => ConfigCategory::Database,
            "web-server" | "web" => ConfigCategory::WebServer,
            "cicd-runner" | "cicd" => ConfigCategory::CICDRunner,
            "machine-learning" | "ml" => ConfigCategory::MachineLearning,
            other => ConfigCategory::Custom(other.to_string()),
        }
    }
}

/// Config format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigFormat {
    Yaml,
    Json,
    Toml,
}

impl std::fmt::Display for ConfigFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigFormat::Yaml => write!(f, "yaml"),
            ConfigFormat::Json => write!(f, "json"),
            ConfigFormat::Toml => write!(f, "toml"),
        }
    }
}

impl ConfigTemplate {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        config_data: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "tpl-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            description: description.into(),
            category: ConfigCategory::Development,
            config_data: config_data.into(),
            format: ConfigFormat::Yaml,
            tags: Vec::new(),
            author: None,
            version: 1,
            usage_count: 0,
            created_at: Utc::now(),
            last_used: None,
        }
    }

    pub fn with_category(mut self, category: ConfigCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_format(mut self, format: ConfigFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag_str = tag.into();
        if !self.tags.contains(&tag_str) {
            self.tags.push(tag_str);
        }
    }

    pub fn remove_tag(&mut self, tag: &str) -> bool {
        let len_before = self.tags.len();
        self.tags.retain(|t| t != tag);
        self.tags.len() < len_before
    }

    pub fn record_usage(&mut self) {
        self.usage_count += 1;
        self.last_used = Some(Utc::now());
    }

    pub fn increment_version(&mut self) {
        self.version += 1;
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }
}

/// Configuration template manager
pub struct ConfigTemplateManager {
    templates: HashMap<String, ConfigTemplate>,
}

impl ConfigTemplateManager {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    pub fn save_template(&mut self, template: ConfigTemplate) -> String {
        let id = template.id.clone();
        self.templates.insert(id.clone(), template);
        id
    }

    pub fn get_template(&self, id: &str) -> Option<&ConfigTemplate> {
        self.templates.get(id)
    }

    pub fn get_template_mut(&mut self, id: &str) -> Option<&mut ConfigTemplate> {
        self.templates.get_mut(id)
    }

    pub fn delete_template(&mut self, id: &str) -> bool {
        self.templates.remove(id).is_some()
    }

    pub fn list_templates(&self) -> Vec<&ConfigTemplate> {
        let mut templates: Vec<&ConfigTemplate> = self.templates.values().collect();
        templates.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        templates
    }

    pub fn find_by_name(&self, name: &str) -> Option<&ConfigTemplate> {
        self.templates.values().find(|t| t.name == name)
    }

    pub fn find_by_category(&self, category: &ConfigCategory) -> Vec<&ConfigTemplate> {
        self.templates
            .values()
            .filter(|t| &t.category == category)
            .collect()
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<&ConfigTemplate> {
        self.templates.values().filter(|t| t.has_tag(tag)).collect()
    }

    pub fn most_used(&self, limit: usize) -> Vec<&ConfigTemplate> {
        let mut templates: Vec<&ConfigTemplate> = self.templates.values().collect();
        templates.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        templates.truncate(limit);
        templates
    }

    pub fn recently_used(&self, limit: usize) -> Vec<&ConfigTemplate> {
        let mut templates: Vec<&ConfigTemplate> = self
            .templates
            .values()
            .filter(|t| t.last_used.is_some())
            .collect();
        templates.sort_by(|a, b| b.last_used.cmp(&a.last_used));
        templates.truncate(limit);
        templates
    }

    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    pub fn categories(&self) -> Vec<&ConfigCategory> {
        let mut cats: Vec<&ConfigCategory> = self.templates.values().map(|t| &t.category).collect();
        cats.dedup();
        cats
    }
}

impl Default for ConfigTemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_template_new() {
        let template =
            ConfigTemplate::new("test-template", "A test template", "cpu: 2\nmemory: 4Gi");
        assert_eq!(template.name, "test-template");
        assert_eq!(template.description, "A test template");
        assert_eq!(template.version, 1);
        assert_eq!(template.usage_count, 0);
        assert!(template.last_used.is_none());
    }

    #[test]
    fn test_config_template_with_category() {
        let template = ConfigTemplate::new("prod", "Production config", "data")
            .with_category(ConfigCategory::Production);
        assert_eq!(template.category, ConfigCategory::Production);
    }

    #[test]
    fn test_config_template_with_format() {
        let template = ConfigTemplate::new("test", "Test", "data").with_format(ConfigFormat::Json);
        assert_eq!(template.format, ConfigFormat::Json);
    }

    #[test]
    fn test_config_template_with_author() {
        let template = ConfigTemplate::new("test", "Test", "data").with_author("admin");
        assert_eq!(template.author, Some("admin".to_string()));
    }

    #[test]
    fn test_config_template_tags() {
        let mut template = ConfigTemplate::new("test", "Test", "data");
        template.add_tag("kubernetes");
        template.add_tag("production");
        template.add_tag("kubernetes"); // Duplicate

        assert_eq!(template.tag_count(), 2);
        assert!(template.has_tag("kubernetes"));
        assert!(template.has_tag("production"));
        assert!(!template.has_tag("staging"));
    }

    #[test]
    fn test_config_template_remove_tag() {
        let mut template = ConfigTemplate::new("test", "Test", "data");
        template.add_tag("tag1");
        template.add_tag("tag2");

        assert!(template.remove_tag("tag1"));
        assert!(!template.remove_tag("nonexistent"));
        assert_eq!(template.tag_count(), 1);
    }

    #[test]
    fn test_config_template_record_usage() {
        let mut template = ConfigTemplate::new("test", "Test", "data");
        assert_eq!(template.usage_count, 0);
        assert!(template.last_used.is_none());

        template.record_usage();
        assert_eq!(template.usage_count, 1);
        assert!(template.last_used.is_some());

        template.record_usage();
        assert_eq!(template.usage_count, 2);
    }

    #[test]
    fn test_config_template_increment_version() {
        let mut template = ConfigTemplate::new("test", "Test", "data");
        assert_eq!(template.version, 1);

        template.increment_version();
        assert_eq!(template.version, 2);
    }

    #[test]
    fn test_config_category_from_str() {
        assert_eq!(ConfigCategory::parse("dev"), ConfigCategory::Development);
        assert_eq!(
            ConfigCategory::parse("development"),
            ConfigCategory::Development
        );
        assert_eq!(ConfigCategory::parse("test"), ConfigCategory::Testing);
        assert_eq!(ConfigCategory::parse("prod"), ConfigCategory::Production);
        assert_eq!(ConfigCategory::parse("db"), ConfigCategory::Database);
        assert_eq!(ConfigCategory::parse("web"), ConfigCategory::WebServer);
        assert_eq!(ConfigCategory::parse("cicd"), ConfigCategory::CICDRunner);
        assert_eq!(ConfigCategory::parse("ml"), ConfigCategory::MachineLearning);
        assert_eq!(
            ConfigCategory::parse("custom"),
            ConfigCategory::Custom("custom".to_string())
        );
    }

    #[test]
    fn test_config_category_display() {
        assert_eq!(ConfigCategory::Development.to_string(), "development");
        assert_eq!(ConfigCategory::Production.to_string(), "production");
        assert_eq!(ConfigCategory::Database.to_string(), "database");
        assert_eq!(
            ConfigCategory::Custom("mycat".to_string()).to_string(),
            "mycat"
        );
    }

    #[test]
    fn test_config_format_display() {
        assert_eq!(ConfigFormat::Yaml.to_string(), "yaml");
        assert_eq!(ConfigFormat::Json.to_string(), "json");
        assert_eq!(ConfigFormat::Toml.to_string(), "toml");
    }

    #[test]
    fn test_template_manager_new() {
        let manager = ConfigTemplateManager::new();
        assert_eq!(manager.template_count(), 0);
    }

    #[test]
    fn test_template_manager_save_and_get() {
        let mut manager = ConfigTemplateManager::new();
        let template = ConfigTemplate::new("test", "Test", "data");
        let id = manager.save_template(template);

        assert_eq!(manager.template_count(), 1);
        assert!(manager.get_template(&id).is_some());
    }

    #[test]
    fn test_template_manager_delete() {
        let mut manager = ConfigTemplateManager::new();
        let template = ConfigTemplate::new("test", "Test", "data");
        let id = manager.save_template(template);

        assert!(manager.delete_template(&id));
        assert!(!manager.delete_template(&id));
        assert_eq!(manager.template_count(), 0);
    }

    #[test]
    fn test_template_manager_find_by_name() {
        let mut manager = ConfigTemplateManager::new();
        manager.save_template(ConfigTemplate::new("alpha", "First", "data1"));
        manager.save_template(ConfigTemplate::new("beta", "Second", "data2"));

        assert!(manager.find_by_name("alpha").is_some());
        assert!(manager.find_by_name("gamma").is_none());
    }

    #[test]
    fn test_template_manager_find_by_category() {
        let mut manager = ConfigTemplateManager::new();
        manager.save_template(
            ConfigTemplate::new("t1", "Test", "data").with_category(ConfigCategory::Production),
        );
        manager.save_template(
            ConfigTemplate::new("t2", "Test", "data").with_category(ConfigCategory::Development),
        );
        manager.save_template(
            ConfigTemplate::new("t3", "Test", "data").with_category(ConfigCategory::Production),
        );

        let prod = manager.find_by_category(&ConfigCategory::Production);
        assert_eq!(prod.len(), 2);
    }

    #[test]
    fn test_template_manager_find_by_tag() {
        let mut manager = ConfigTemplateManager::new();

        let mut t1 = ConfigTemplate::new("t1", "Test", "data");
        t1.add_tag("kubernetes");

        let mut t2 = ConfigTemplate::new("t2", "Test", "data");
        t2.add_tag("docker");

        let mut t3 = ConfigTemplate::new("t3", "Test", "data");
        t3.add_tag("kubernetes");

        manager.save_template(t1);
        manager.save_template(t2);
        manager.save_template(t3);

        let k8s = manager.find_by_tag("kubernetes");
        assert_eq!(k8s.len(), 2);
    }

    #[test]
    fn test_template_manager_most_used() {
        let mut manager = ConfigTemplateManager::new();

        let mut t1 = ConfigTemplate::new("t1", "Low usage", "data");
        t1.usage_count = 5;

        let mut t2 = ConfigTemplate::new("t2", "High usage", "data");
        t2.usage_count = 100;

        let mut t3 = ConfigTemplate::new("t3", "Medium usage", "data");
        t3.usage_count = 50;

        manager.save_template(t1);
        manager.save_template(t2);
        manager.save_template(t3);

        let top = manager.most_used(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].usage_count, 100);
    }

    #[test]
    fn test_template_manager_list() {
        let mut manager = ConfigTemplateManager::new();
        manager.save_template(ConfigTemplate::new("t1", "First", "data1"));
        manager.save_template(ConfigTemplate::new("t2", "Second", "data2"));

        let list = manager.list_templates();
        assert_eq!(list.len(), 2);
    }
}
