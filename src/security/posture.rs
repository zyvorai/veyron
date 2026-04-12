// Security Posture - Multi-category security assessments

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPosture {
    pub overall_score: u8,
    pub categories: Vec<PostureCategory>,
    pub findings: Vec<SecurityFinding>,
    pub assessed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostureCategory {
    pub name: String,
    pub category_type: PostureCategoryType,
    pub score: u8,
    pub max_score: u8,
    pub finding_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PostureCategoryType {
    NetworkPolicy, PodSecurity, RBAC, SecretManagement,
    ImageSecurity, RuntimeSecurity, Compliance, Configuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: FindingSeverity,
    pub category: PostureCategoryType,
    pub resource: String,
    pub namespace: String,
    pub remediation: String,
    pub cvss_score: Option<f64>,
    pub cve_id: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub status: FindingStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum FindingSeverity { Critical, High, Medium, Low, Info }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FindingStatus { Open, Acknowledged, InProgress, Resolved, FalsePositive }

impl SecurityPosture {
    pub fn new() -> Self {
        Self { overall_score: 0, categories: Vec::new(), findings: Vec::new(), assessed_at: Utc::now() }
    }

    pub fn add_finding(&mut self, finding: SecurityFinding) { self.findings.push(finding); }
    pub fn critical_findings(&self) -> Vec<&SecurityFinding> { self.findings.iter().filter(|f| f.severity == FindingSeverity::Critical).collect() }
    pub fn open_findings(&self) -> Vec<&SecurityFinding> { self.findings.iter().filter(|f| f.status == FindingStatus::Open).collect() }
    pub fn by_category(&self, cat: &PostureCategoryType) -> Vec<&SecurityFinding> { self.findings.iter().filter(|f| f.category == *cat).collect() }

    pub fn calculate_score(&mut self) {
        if self.findings.is_empty() { self.overall_score = 100; return; }
        let total_impact: f64 = self.findings.iter().filter(|f| f.status == FindingStatus::Open).map(|f| match f.severity {
            FindingSeverity::Critical => 25.0, FindingSeverity::High => 15.0,
            FindingSeverity::Medium => 8.0, FindingSeverity::Low => 3.0, FindingSeverity::Info => 1.0,
        }).sum();
        self.overall_score = (100.0 - total_impact).clamp(0.0, 100.0) as u8;
    }
}

impl Default for SecurityPosture {
    fn default() -> Self { Self::new() }
}
