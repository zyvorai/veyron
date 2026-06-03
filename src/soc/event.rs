// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Normalized security event (ECS-friendly).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: String,
    pub timestamp: String,
    pub severity: String,
    pub category: String,
    pub source: String,
    pub actor: String,
    pub target: String,
    pub namespace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vm_name: Option<String>,
    pub action: String,
    pub outcome: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
}

impl SecurityEvent {
    pub fn new(
        category: impl Into<String>,
        severity: impl Into<String>,
        source: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let ts = Utc::now();
        let id = format!("soc-{}-{:x}", ts.timestamp_millis(), rand_id());
        Self {
            id,
            timestamp: ts.to_rfc3339(),
            severity: severity.into(),
            category: category.into(),
            source: source.into(),
            actor: "system".into(),
            target: String::new(),
            namespace: String::new(),
            vm_name: None,
            action: String::new(),
            outcome: "unknown".into(),
            message: message.into(),
            labels: BTreeMap::new(),
        }
    }

    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = actor.into();
        self
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }

    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = ns.into();
        self
    }

    pub fn with_vm(mut self, name: impl Into<String>) -> Self {
        self.vm_name = Some(name.into());
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = action.into();
        self
    }

    pub fn with_outcome(mut self, outcome: impl Into<String>) -> Self {
        self.outcome = outcome.into();
        self
    }

    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

fn rand_id() -> u32 {
    use rand::Rng;
    rand::thread_rng().r#gen()
}

/// Map to Elasticsearch ECS-shaped document for bulk index.
pub fn to_ecs_doc(ev: &SecurityEvent) -> serde_json::Value {
    let host = ev.vm_name.as_deref().unwrap_or(&ev.target);
    serde_json::json!({
        "@timestamp": ev.timestamp,
        "event": {
            "id": ev.id,
            "category": [ev.category],
            "kind": "event",
            "outcome": ev.outcome,
            "severity": severity_to_ecs(&ev.severity),
        },
        "message": ev.message,
        "user": { "name": ev.actor },
        "host": { "name": host },
        "cloud": {
            "instance": { "name": ev.vm_name },
            "provider": "kubevirt"
        },
        "kubernetes": {
            "namespace": ev.namespace,
        },
        "vmrogue": {
            "source": ev.source,
            "action": ev.action,
            "labels": ev.labels,
        }
    })
}

fn severity_to_ecs(s: &str) -> i64 {
    match s.to_lowercase().as_str() {
        "critical" => 90,
        "high" => 70,
        "medium" => 50,
        "low" => 30,
        _ => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecs_doc_has_timestamp() {
        let ev = SecurityEvent::new("audit", "high", "vmrogue", "test");
        let doc = to_ecs_doc(&ev);
        assert!(doc.get("@timestamp").is_some());
    }
}
