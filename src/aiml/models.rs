// ML Model Management - Model registry and versioning

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::MLFramework;

/// ML model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLModel {
    pub id: String,
    pub name: String,
    pub version: String,
    pub framework: MLFramework,
    pub model_type: ModelType,
    pub size_mb: u64,
    pub accuracy: Option<f64>,
    pub storage_path: String,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl MLModel {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        framework: MLFramework,
        model_type: ModelType,
    ) -> Self {
        let name_str = name.into();
        let version_str = version.into();
        let id = format!(
            "model-{}-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            version_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            version: version_str,
            framework,
            model_type,
            size_mb: 0,
            accuracy: None,
            storage_path: String::new(),
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_size(mut self, size_mb: u64) -> Self {
        self.size_mb = size_mb;
        self
    }

    pub fn with_accuracy(mut self, accuracy: f64) -> Self {
        self.accuracy = Some(accuracy);
        self
    }

    pub fn with_storage_path(mut self, path: impl Into<String>) -> Self {
        self.storage_path = path.into();
        self
    }

    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    pub fn full_name(&self) -> String {
        format!("{}:{}", self.name, self.version)
    }
}

/// Model type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelType {
    Classification,
    Regression,
    ObjectDetection,
    Segmentation,
    NLP,
    GenerativeAI,
    Reinforcement,
}

/// Model registry
pub struct ModelRegistry {
    models: HashMap<String, MLModel>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn register_model(&mut self, model: MLModel) -> String {
        let id = model.id.clone();
        self.models.insert(id.clone(), model);
        id
    }

    pub fn get_model(&self, id: &str) -> Option<&MLModel> {
        self.models.get(id)
    }

    pub fn get_model_mut(&mut self, id: &str) -> Option<&mut MLModel> {
        self.models.get_mut(id)
    }

    pub fn remove_model(&mut self, id: &str) -> bool {
        self.models.remove(id).is_some()
    }

    pub fn list_models(&self) -> Vec<&MLModel> {
        self.models.values().collect()
    }

    pub fn by_name(&self, name: &str) -> Vec<&MLModel> {
        self.models.values().filter(|m| m.name == name).collect()
    }

    pub fn by_framework(&self, framework: MLFramework) -> Vec<&MLModel> {
        self.models
            .values()
            .filter(|m| m.framework == framework)
            .collect()
    }

    pub fn by_type(&self, model_type: ModelType) -> Vec<&MLModel> {
        self.models
            .values()
            .filter(|m| m.model_type == model_type)
            .collect()
    }

    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    pub fn total_size_mb(&self) -> u64 {
        self.models.values().map(|m| m.size_mb).sum()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ml_model() {
        let model = MLModel::new(
            "resnet50",
            "v1.0",
            MLFramework::PyTorch,
            ModelType::Classification,
        )
        .with_size(98)
        .with_accuracy(0.92)
        .with_storage_path("/models/resnet50-v1.0.pt");

        assert_eq!(model.name, "resnet50");
        assert_eq!(model.version, "v1.0");
        assert_eq!(model.framework, MLFramework::PyTorch);
        assert_eq!(model.size_mb, 98);
        assert_eq!(model.accuracy, Some(0.92));
    }

    #[test]
    fn test_model_metadata() {
        let mut model = MLModel::new("bert", "base", MLFramework::TensorFlow, ModelType::NLP);

        model.add_metadata("dataset", "wikipedia");
        model.add_metadata("epochs", "10");

        assert_eq!(
            model.metadata.get("dataset"),
            Some(&"wikipedia".to_string())
        );
        assert_eq!(model.metadata.get("epochs"), Some(&"10".to_string()));
    }

    #[test]
    fn test_model_full_name() {
        let model = MLModel::new("gpt", "3.5", MLFramework::PyTorch, ModelType::GenerativeAI);

        assert_eq!(model.full_name(), "gpt:3.5");
    }

    #[test]
    fn test_model_registry() {
        let mut registry = ModelRegistry::new();

        let model = MLModel::new(
            "test",
            "v1",
            MLFramework::PyTorch,
            ModelType::Classification,
        );
        let id = registry.register_model(model);

        assert_eq!(registry.model_count(), 1);
        assert!(registry.get_model(&id).is_some());
    }

    #[test]
    fn test_registry_by_name() {
        let mut registry = ModelRegistry::new();

        registry.register_model(MLModel::new(
            "resnet",
            "v1",
            MLFramework::PyTorch,
            ModelType::Classification,
        ));
        registry.register_model(MLModel::new(
            "resnet",
            "v2",
            MLFramework::PyTorch,
            ModelType::Classification,
        ));
        registry.register_model(MLModel::new(
            "bert",
            "v1",
            MLFramework::TensorFlow,
            ModelType::NLP,
        ));

        let resnet_models = registry.by_name("resnet");
        assert_eq!(resnet_models.len(), 2);
    }

    #[test]
    fn test_registry_by_framework() {
        let mut registry = ModelRegistry::new();

        registry.register_model(MLModel::new(
            "model1",
            "v1",
            MLFramework::PyTorch,
            ModelType::Classification,
        ));
        registry.register_model(MLModel::new(
            "model2",
            "v1",
            MLFramework::TensorFlow,
            ModelType::Regression,
        ));
        registry.register_model(MLModel::new(
            "model3",
            "v1",
            MLFramework::PyTorch,
            ModelType::NLP,
        ));

        let pytorch_models = registry.by_framework(MLFramework::PyTorch);
        assert_eq!(pytorch_models.len(), 2);
    }

    #[test]
    fn test_registry_by_type() {
        let mut registry = ModelRegistry::new();

        registry.register_model(MLModel::new(
            "model1",
            "v1",
            MLFramework::PyTorch,
            ModelType::Classification,
        ));
        registry.register_model(MLModel::new(
            "model2",
            "v1",
            MLFramework::TensorFlow,
            ModelType::Classification,
        ));
        registry.register_model(MLModel::new(
            "model3",
            "v1",
            MLFramework::JAX,
            ModelType::Regression,
        ));

        let classification = registry.by_type(ModelType::Classification);
        assert_eq!(classification.len(), 2);
    }

    #[test]
    fn test_registry_total_size() {
        let mut registry = ModelRegistry::new();

        registry.register_model(
            MLModel::new(
                "model1",
                "v1",
                MLFramework::PyTorch,
                ModelType::Classification,
            )
            .with_size(100),
        );
        registry.register_model(
            MLModel::new(
                "model2",
                "v1",
                MLFramework::TensorFlow,
                ModelType::Regression,
            )
            .with_size(50),
        );

        assert_eq!(registry.total_size_mb(), 150);
    }

    #[test]
    fn test_registry_remove_model() {
        let mut registry = ModelRegistry::new();

        let model = MLModel::new(
            "test",
            "v1",
            MLFramework::PyTorch,
            ModelType::Classification,
        );
        let id = registry.register_model(model);

        assert!(registry.remove_model(&id));
        assert_eq!(registry.model_count(), 0);
    }

    #[test]
    fn test_model_type() {
        assert_eq!(ModelType::Classification, ModelType::Classification);
        assert_ne!(ModelType::Classification, ModelType::Regression);
    }
}
