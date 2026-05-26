// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// AI Troubleshooting - Diagnostic wizard with root cause analysis

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::generate_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TroubleshootSession {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: SessionStatus,
    pub symptoms: Vec<Symptom>,
    pub root_causes: Vec<RootCause>,
    pub remediations: Vec<Remediation>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    InProgress,
    DiagnosisComplete,
    Resolved,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symptom {
    pub description: String,
    pub severity: IssueSeverity,
    pub category: SymptomCategory,
    pub detected_at: DateTime<Utc>,
    pub metrics: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymptomCategory {
    Performance,
    Connectivity,
    Storage,
    Configuration,
    Resource,
    Security,
    Scheduling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCause {
    pub description: String,
    pub confidence: DiagnosisConfidence,
    pub evidence: Vec<String>,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiagnosisConfidence {
    High,
    Medium,
    Low,
    Uncertain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remediation {
    pub action: String,
    pub description: String,
    pub risk_level: RemediationRisk,
    pub estimated_duration: String,
    pub prerequisites: Vec<String>,
    pub rollback_plan: String,
    pub automated: bool,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemediationRisk {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl TroubleshootSession {
    pub fn new(title: &str, description: &str) -> Self {
        Self {
            id: generate_id("ts", title),
            title: title.to_string(),
            description: description.to_string(),
            status: SessionStatus::InProgress,
            symptoms: Vec::new(),
            root_causes: Vec::new(),
            remediations: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn add_symptom(&mut self, symptom: Symptom) {
        self.symptoms.push(symptom);
        self.updated_at = Utc::now();
    }

    pub fn diagnose(&mut self) {
        self.root_causes.clear();
        self.remediations.clear();

        for symptom in &self.symptoms {
            match symptom.category {
                SymptomCategory::Performance => {
                    self.root_causes.push(RootCause {
                        description: "Insufficient CPU/Memory resources allocated".to_string(),
                        confidence: DiagnosisConfidence::High,
                        evidence: vec!["High resource utilization detected".to_string()],
                        category: "Resource".to_string(),
                    });
                    self.remediations.push(Remediation {
                        action: "Scale up VM resources".to_string(),
                        description: "Increase CPU cores and memory allocation".to_string(),
                        risk_level: RemediationRisk::Low,
                        estimated_duration: "5 minutes".to_string(),
                        prerequisites: vec!["Available cluster resources".to_string()],
                        rollback_plan: "Revert resource allocation to previous values".to_string(),
                        automated: true,
                        commands: vec![
                            "vmrogue resources scale --cpu +2 --memory +2Gi".to_string(),
                        ],
                    });
                }
                SymptomCategory::Connectivity => {
                    self.root_causes.push(RootCause {
                        description: "Network policy blocking traffic".to_string(),
                        confidence: DiagnosisConfidence::Medium,
                        evidence: vec!["Connection timeouts observed".to_string()],
                        category: "Network".to_string(),
                    });
                    self.remediations.push(Remediation {
                        action: "Check network policies".to_string(),
                        description: "Review and update network policies".to_string(),
                        risk_level: RemediationRisk::Medium,
                        estimated_duration: "10 minutes".to_string(),
                        prerequisites: Vec::new(),
                        rollback_plan: "Restore previous network policy".to_string(),
                        automated: false,
                        commands: vec!["vmrogue network-policies".to_string()],
                    });
                }
                SymptomCategory::Storage => {
                    self.root_causes.push(RootCause {
                        description: "Disk space exhaustion or I/O bottleneck".to_string(),
                        confidence: DiagnosisConfidence::High,
                        evidence: vec!["Storage metrics indicate high utilization".to_string()],
                        category: "Storage".to_string(),
                    });
                    self.remediations.push(Remediation {
                        action: "Expand disk or clean up storage".to_string(),
                        description: "Expand PVC or remove unnecessary data".to_string(),
                        risk_level: RemediationRisk::Low,
                        estimated_duration: "15 minutes".to_string(),
                        prerequisites: vec!["Storage class supports expansion".to_string()],
                        rollback_plan: "Snapshot before expansion".to_string(),
                        automated: true,
                        commands: vec!["vmrogue disk-expand --vm <name> --size +10Gi".to_string()],
                    });
                }
                _ => {
                    self.root_causes.push(RootCause {
                        description: format!(
                            "Issue in {:?} category requires investigation",
                            symptom.category
                        ),
                        confidence: DiagnosisConfidence::Low,
                        evidence: vec![symptom.description.clone()],
                        category: format!("{:?}", symptom.category),
                    });
                }
            }
        }

        self.status = SessionStatus::DiagnosisComplete;
        self.updated_at = Utc::now();
    }

    pub fn resolve(&mut self) {
        self.status = SessionStatus::Resolved;
        self.updated_at = Utc::now();
    }

    pub fn highest_severity(&self) -> Option<&IssueSeverity> {
        self.symptoms
            .iter()
            .map(|s| &s.severity)
            .min_by_key(|s| match s {
                IssueSeverity::Critical => 0,
                IssueSeverity::High => 1,
                IssueSeverity::Medium => 2,
                IssueSeverity::Low => 3,
                IssueSeverity::Info => 4,
            })
    }
}
