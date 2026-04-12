use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Application runtime
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Runtime {
    Container,
    VM,
    Serverless,
    Native,
}

/// Edge application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeApplication {
    pub id: String,
    pub name: String,
    pub runtime: Runtime,
    pub version: String,
    pub image: String,
    pub replicas_desired: u32,
    pub replicas_running: u32,
    pub auto_scale: bool,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub created_at: DateTime<Utc>,
}

impl EdgeApplication {
    pub fn new(
        name: impl Into<String>,
        runtime: Runtime,
        version: impl Into<String>,
        image: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "app-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            runtime,
            version: version.into(),
            image: image.into(),
            replicas_desired: 1,
            replicas_running: 0,
            auto_scale: false,
            min_replicas: 1,
            max_replicas: 10,
            created_at: Utc::now(),
        }
    }

    pub fn enable_auto_scale(mut self, min: u32, max: u32) -> Self {
        self.auto_scale = true;
        self.min_replicas = min;
        self.max_replicas = max;
        self
    }

    pub fn set_replicas(&mut self, desired: u32) {
        self.replicas_desired = desired;
    }

    pub fn is_healthy(&self) -> bool {
        self.replicas_running >= self.replicas_desired
    }

    pub fn is_degraded(&self) -> bool {
        self.replicas_running > 0 && self.replicas_running < self.replicas_desired
    }

    pub fn is_down(&self) -> bool {
        self.replicas_running == 0
    }
}

/// Application manager
pub struct ApplicationManager {
    applications: HashMap<String, EdgeApplication>,
}

impl ApplicationManager {
    pub fn new() -> Self {
        Self {
            applications: HashMap::new(),
        }
    }

    pub fn add_application(&mut self, app: EdgeApplication) -> String {
        let id = app.id.clone();
        self.applications.insert(id.clone(), app);
        id
    }

    pub fn get_application(&self, id: &str) -> Option<&EdgeApplication> {
        self.applications.get(id)
    }

    pub fn get_application_mut(&mut self, id: &str) -> Option<&mut EdgeApplication> {
        self.applications.get_mut(id)
    }

    pub fn application_count(&self) -> usize {
        self.applications.len()
    }

    pub fn applications_by_runtime(&self, runtime: &Runtime) -> Vec<&EdgeApplication> {
        self.applications
            .values()
            .filter(|a| &a.runtime == runtime)
            .collect()
    }

    pub fn healthy_applications(&self) -> Vec<&EdgeApplication> {
        self.applications
            .values()
            .filter(|a| a.is_healthy())
            .collect()
    }

    pub fn degraded_applications(&self) -> Vec<&EdgeApplication> {
        self.applications
            .values()
            .filter(|a| a.is_degraded())
            .collect()
    }

    pub fn down_applications(&self) -> Vec<&EdgeApplication> {
        self.applications.values().filter(|a| a.is_down()).collect()
    }

    pub fn auto_scaled_applications(&self) -> Vec<&EdgeApplication> {
        self.applications
            .values()
            .filter(|a| a.auto_scale)
            .collect()
    }
}

impl Default for ApplicationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_application() {
        let app = EdgeApplication::new("web-app", Runtime::Container, "1.0.0", "nginx:latest");

        assert_eq!(app.name, "web-app");
        assert_eq!(app.runtime, Runtime::Container);
        assert_eq!(app.version, "1.0.0");
        assert_eq!(app.image, "nginx:latest");
        assert!(!app.auto_scale);
    }

    #[test]
    fn test_app_enable_auto_scale() {
        let app = EdgeApplication::new("web", Runtime::Container, "1.0.0", "nginx:latest")
            .enable_auto_scale(2, 5);

        assert!(app.auto_scale);
        assert_eq!(app.min_replicas, 2);
        assert_eq!(app.max_replicas, 5);
    }

    #[test]
    fn test_app_set_replicas() {
        let mut app = EdgeApplication::new("web", Runtime::Container, "1.0.0", "nginx:latest");

        app.set_replicas(3);
        assert_eq!(app.replicas_desired, 3);
    }

    #[test]
    fn test_app_is_healthy() {
        let mut app = EdgeApplication::new("web", Runtime::Container, "1.0.0", "nginx:latest");
        app.set_replicas(2);
        app.replicas_running = 2;

        assert!(app.is_healthy());
    }

    #[test]
    fn test_app_is_degraded() {
        let mut app = EdgeApplication::new("web", Runtime::Container, "1.0.0", "nginx:latest");
        app.set_replicas(3);
        app.replicas_running = 1;

        assert!(app.is_degraded());
        assert!(!app.is_healthy());
    }

    #[test]
    fn test_app_is_down() {
        let mut app = EdgeApplication::new("web", Runtime::Container, "1.0.0", "nginx:latest");
        app.replicas_running = 0;

        assert!(app.is_down());
    }

    #[test]
    fn test_application_manager() {
        let mut manager = ApplicationManager::new();

        let app = EdgeApplication::new("web", Runtime::Container, "1.0.0", "nginx:latest");
        let id = manager.add_application(app);

        assert_eq!(manager.application_count(), 1);
        assert!(manager.get_application(&id).is_some());
    }

    #[test]
    fn test_manager_applications_by_runtime() {
        let mut manager = ApplicationManager::new();

        manager.add_application(EdgeApplication::new(
            "app1",
            Runtime::Container,
            "1.0.0",
            "img1",
        ));
        manager.add_application(EdgeApplication::new("app2", Runtime::VM, "1.0.0", "img2"));
        manager.add_application(EdgeApplication::new(
            "app3",
            Runtime::Container,
            "1.0.0",
            "img3",
        ));

        let containers = manager.applications_by_runtime(&Runtime::Container);
        assert_eq!(containers.len(), 2);
    }

    #[test]
    fn test_manager_healthy_applications() {
        let mut manager = ApplicationManager::new();

        let mut app1 = EdgeApplication::new("app1", Runtime::Container, "1.0.0", "img1");
        app1.set_replicas(2);
        app1.replicas_running = 2;

        let app2 = EdgeApplication::new("app2", Runtime::Container, "1.0.0", "img2");

        manager.add_application(app1);
        manager.add_application(app2);

        let healthy = manager.healthy_applications();
        assert_eq!(healthy.len(), 1);
    }

    #[test]
    fn test_manager_degraded_applications() {
        let mut manager = ApplicationManager::new();

        let mut app1 = EdgeApplication::new("app1", Runtime::Container, "1.0.0", "img1");
        app1.set_replicas(3);
        app1.replicas_running = 1;

        let mut app2 = EdgeApplication::new("app2", Runtime::Container, "1.0.0", "img2");
        app2.replicas_running = 0;

        manager.add_application(app1);
        manager.add_application(app2);

        let degraded = manager.degraded_applications();
        assert_eq!(degraded.len(), 1);
    }

    #[test]
    fn test_manager_down_applications() {
        let mut manager = ApplicationManager::new();

        let mut app1 = EdgeApplication::new("app1", Runtime::Container, "1.0.0", "img1");
        app1.replicas_running = 0;

        let mut app2 = EdgeApplication::new("app2", Runtime::Container, "1.0.0", "img2");
        app2.replicas_running = 1;

        manager.add_application(app1);
        manager.add_application(app2);

        let down = manager.down_applications();
        assert_eq!(down.len(), 1);
    }

    #[test]
    fn test_manager_auto_scaled_applications() {
        let mut manager = ApplicationManager::new();

        manager.add_application(
            EdgeApplication::new("app1", Runtime::Container, "1.0.0", "img1")
                .enable_auto_scale(2, 5),
        );
        manager.add_application(EdgeApplication::new(
            "app2",
            Runtime::Container,
            "1.0.0",
            "img2",
        ));

        let auto_scaled = manager.auto_scaled_applications();
        assert_eq!(auto_scaled.len(), 1);
    }

    #[test]
    fn test_runtime_equality() {
        assert_eq!(Runtime::Container, Runtime::Container);
        assert_ne!(Runtime::Container, Runtime::VM);
    }
}
