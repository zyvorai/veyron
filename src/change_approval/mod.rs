// Change Approval - Multi-stage approval workflows for critical operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeApprovalManager {
    pub requests: Vec<ChangeRequest>,
    pub policies: Vec<ApprovalPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRequest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub change_type: ChangeType,
    pub target_resource: String,
    pub target_namespace: String,
    pub requested_by: String,
    pub status: ApprovalStatus,
    pub approvals: Vec<Approval>,
    pub pre_checks: Vec<CheckResult>,
    pub post_checks: Vec<CheckResult>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub rollback_plan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    VmCreate,
    VmDelete,
    VmMigrate,
    VmScale,
    ConfigChange,
    SecurityUpdate,
    NetworkChange,
    StorageChange,
    ClusterWide,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    InProgress,
    Completed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub approver: String,
    pub decision: ApprovalDecision,
    pub comment: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Approved,
    Rejected,
    NeedsInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPolicy {
    pub name: String,
    pub change_types: Vec<ChangeType>,
    pub required_approvers: usize,
    pub auto_approve_for_roles: Vec<String>,
    pub expiry_hours: u64,
}

impl ChangeApprovalManager {
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
            policies: Vec::new(),
        }
    }

    pub fn submit_request(&mut self, request: ChangeRequest) -> String {
        let id = request.id.clone();
        self.requests.push(request);
        id
    }

    pub fn approve(&mut self, request_id: &str, approver: &str, comment: &str) -> bool {
        if let Some(req) = self
            .requests
            .iter_mut()
            .find(|r| r.id == request_id && r.status == ApprovalStatus::Pending)
        {
            req.approvals.push(Approval {
                approver: approver.to_string(),
                decision: ApprovalDecision::Approved,
                comment: comment.to_string(),
                timestamp: Utc::now(),
            });
            let required = self
                .policies
                .iter()
                .find(|p| {
                    p.change_types.iter().any(|ct| {
                        std::mem::discriminant(ct) == std::mem::discriminant(&req.change_type)
                    })
                })
                .map(|p| p.required_approvers)
                .unwrap_or(1);
            let approved_count = req
                .approvals
                .iter()
                .filter(|a| matches!(a.decision, ApprovalDecision::Approved))
                .count();
            if approved_count >= required {
                req.status = ApprovalStatus::Approved;
                req.resolved_at = Some(Utc::now());
            }
            true
        } else {
            false
        }
    }

    pub fn reject(&mut self, request_id: &str, approver: &str, comment: &str) -> bool {
        if let Some(req) = self
            .requests
            .iter_mut()
            .find(|r| r.id == request_id && r.status == ApprovalStatus::Pending)
        {
            req.approvals.push(Approval {
                approver: approver.to_string(),
                decision: ApprovalDecision::Rejected,
                comment: comment.to_string(),
                timestamp: Utc::now(),
            });
            req.status = ApprovalStatus::Rejected;
            req.resolved_at = Some(Utc::now());
            true
        } else {
            false
        }
    }

    pub fn pending_requests(&self) -> Vec<&ChangeRequest> {
        self.requests
            .iter()
            .filter(|r| r.status == ApprovalStatus::Pending)
            .collect()
    }

    pub fn get_request(&self, id: &str) -> Option<&ChangeRequest> {
        self.requests.iter().find(|r| r.id == id)
    }
}

impl Default for ChangeApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}
