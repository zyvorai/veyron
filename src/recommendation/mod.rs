// Recommendation Engine - Resource optimization and cost saving recommendations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationEngine {
    pub recommendations: Vec<Recommendation>,
    pub config: RecommendationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub category: RecommendationCategory,
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub impact: Impact,
    pub effort: Effort,
    pub potential_savings: f64,
    pub resource_name: String,
    pub namespace: String,
    pub created_at: DateTime<Utc>,
    pub status: RecommendationStatus,
    pub actions: Vec<RecommendedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationCategory {
    ResourceOptimization, CostSaving, PerformanceTuning, SecurityHardening,
    HighAvailability, Compliance, BestPractice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Priority { Critical, High, Medium, Low, Info }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Impact { High, Medium, Low }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Effort { Minimal, Low, Medium, High }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationStatus { New, Acknowledged, InProgress, Applied, Dismissed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedAction {
    pub description: String,
    pub command: Option<String>,
    pub automated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationConfig {
    pub enabled: bool,
    pub min_priority: Priority,
    pub categories: Vec<RecommendationCategory>,
    pub auto_refresh_secs: u64,
}

impl Default for RecommendationConfig {
    fn default() -> Self {
        Self {
            enabled: true, min_priority: Priority::Low,
            categories: vec![RecommendationCategory::ResourceOptimization, RecommendationCategory::CostSaving,
                RecommendationCategory::PerformanceTuning, RecommendationCategory::SecurityHardening],
            auto_refresh_secs: 3600,
        }
    }
}

impl RecommendationEngine {
    pub fn new() -> Self {
        Self { recommendations: Vec::new(), config: RecommendationConfig::default() }
    }

    pub fn add(&mut self, rec: Recommendation) {
        self.recommendations.push(rec);
    }

    pub fn by_category(&self, cat: &RecommendationCategory) -> Vec<&Recommendation> {
        self.recommendations.iter().filter(|r| r.category == *cat).collect()
    }

    pub fn by_priority(&self, min: &Priority) -> Vec<&Recommendation> {
        self.recommendations.iter().filter(|r| r.priority <= *min).collect()
    }

    pub fn pending(&self) -> Vec<&Recommendation> {
        self.recommendations.iter().filter(|r| matches!(r.status, RecommendationStatus::New)).collect()
    }

    pub fn total_potential_savings(&self) -> f64 {
        self.recommendations.iter().map(|r| r.potential_savings).sum()
    }

    pub fn dismiss(&mut self, id: &str) {
        if let Some(r) = self.recommendations.iter_mut().find(|r| r.id == id) {
            r.status = RecommendationStatus::Dismissed;
        }
    }

    pub fn apply(&mut self, id: &str) {
        if let Some(r) = self.recommendations.iter_mut().find(|r| r.id == id) {
            r.status = RecommendationStatus::Applied;
        }
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self { Self::new() }
}
