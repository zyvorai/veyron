// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

use super::feature_context::VmrogueFeatureContext;

/// One entry in the incident timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentEvent {
    pub id: String,
    pub timestamp: String,
    pub kind: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub vm_name: Option<String>,
    pub namespace: String,
    pub resolved: bool,
}

/// Full incident timeline response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentTimeline {
    pub vmrogue_context: VmrogueFeatureContext,
    pub total_incidents: u32,
    pub open_incidents: u32,
    pub resolved_last_24h: u32,
    pub critical: u32,
    pub warning: u32,
    pub events: Vec<IncidentEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IncidentQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/incidents/timeline", get(get_incident_timeline))
        .with_state(state)
}

/// Build an incident timeline by correlating K8s Warning events and SLO status.
#[cfg(feature = "web")]
async fn get_incident_timeline(
    State(state): State<SharedState>,
    Query(query): Query<IncidentQuery>,
) -> Json<IncidentTimeline> {
    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());

    let mut incidents: Vec<IncidentEvent> = Vec::new();
    let now = chrono::Utc::now();
    let mut ctx = VmrogueFeatureContext::incidents();

    if let Some(am_url) = crate::api::integrations::env_var("VMROGUE_ALERTMANAGER_URL") {
        match crate::api::integrations::fetch_alertmanager_alerts(&am_url).await {
            Ok(alerts) if !alerts.is_empty() => {
                ctx = VmrogueFeatureContext::incidents_alertmanager();
                for (i, alert) in alerts.iter().enumerate() {
                    let labels = alert.labels.as_ref();
                    let ns = labels
                        .and_then(|l| l.get("namespace"))
                        .cloned()
                        .unwrap_or_else(|| namespace.clone());
                    if namespace != "all" && ns != namespace {
                        continue;
                    }
                    let title = labels
                        .and_then(|l| l.get("alertname"))
                        .cloned()
                        .unwrap_or_else(|| "Alert".into());
                    let desc = alert
                        .annotations
                        .as_ref()
                        .and_then(|a| a.get("description"))
                        .or_else(|| alert.annotations.as_ref().and_then(|a| a.get("summary")))
                        .cloned()
                        .unwrap_or_default();
                    let state = alert
                        .status
                        .as_ref()
                        .and_then(|st| st.state.as_deref())
                        .unwrap_or("active");
                    incidents.push(IncidentEvent {
                        id: format!("am-{i}-{title}"),
                        timestamp: now.to_rfc3339(),
                        kind: "alertmanager".to_string(),
                        severity: labels
                            .and_then(|l| l.get("severity"))
                            .cloned()
                            .unwrap_or_else(|| "warning".into()),
                        title,
                        description: desc,
                        vm_name: labels.and_then(|l| l.get("vm")).cloned(),
                        namespace: ns,
                        resolved: state == "suppressed",
                    });
                }
            }
            Ok(_) => {}
            Err(e) => {
                log::warn!("Alertmanager fetch failed: {}", e);
            }
        }
    }

    if incidents.is_empty() {
        let events = if namespace == "all" {
            s.client().list_all_events().await.unwrap_or_default()
        } else {
            s.client().list_events(&namespace).await.unwrap_or_default()
        };

        for event in &events {
            if event.type_.as_deref() != Some("Warning") {
                continue;
            }

            let reason = event.reason.as_deref().unwrap_or("Unknown");
            let msg = event.message.as_deref().unwrap_or("");
            let obj = event.involved_object.name.as_deref().unwrap_or("unknown");
            let obj_kind = event.involved_object.kind.as_deref().unwrap_or("Object");
            let ev_ns = event
                .metadata
                .namespace
                .as_deref()
                .unwrap_or(namespace.as_str());

            let ts = event
                .last_timestamp
                .as_ref()
                .or(event.first_timestamp.as_ref())
                .or(event.metadata.creation_timestamp.as_ref())
                .map(|t| t.0)
                .unwrap_or(now);

            let age_hours = (now - ts).num_hours();

            // Map K8s warning reason to severity
            let severity = match reason {
                "OOMKilling" | "BackOff" | "Failed" | "FailedMount" | "FailedScheduling" => {
                    "critical"
                }
                _ => "warning",
            };

            // VM-related events get the vm_name extracted from the object name
            let vm_name = if obj_kind == "VirtualMachineInstance" || obj_kind == "VirtualMachine" {
                Some(obj.to_string())
            } else if obj.starts_with("virt-launcher-") {
                Some(obj.trim_start_matches("virt-launcher-").to_string())
            } else {
                None
            };

            // Consider events older than 1h as resolved (simplified heuristic)
            let resolved = age_hours > 1;

            let id = format!("{}-{}-{}", ev_ns, obj, ts.timestamp());

            incidents.push(IncidentEvent {
                id,
                timestamp: ts.to_rfc3339(),
                kind: "k8s-event".to_string(),
                severity: severity.to_string(),
                title: format!("{}/{}: {}", obj_kind, obj, reason),
                description: msg.to_string(),
                vm_name,
                namespace: ev_ns.to_string(),
                resolved,
            });
        }
    } // incidents.is_empty() fallback

    // Check SLO status and add a synthetic SLO-breach incident if needed
    let vms = if namespace == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&namespace).await.unwrap_or_default()
    };
    if !vms.is_empty() {
        let total = vms.len() as f64;
        let running = vms
            .iter()
            .filter(|vm| vm.status.as_ref().and_then(|s| s.ready).unwrap_or(false))
            .count() as f64;
        let availability = running / total * 100.0;
        if availability < 99.9 {
            incidents.push(IncidentEvent {
                id: format!("slo-breach-{}", namespace),
                timestamp: now.to_rfc3339(),
                kind: "slo-breach".to_string(),
                severity: if availability < 99.0 {
                    "critical"
                } else {
                    "warning"
                }
                .to_string(),
                title: format!(
                    "SLO breach — VM availability {:.1}% < 99.9% target",
                    availability
                ),
                description: format!(
                    "{} of {} VMs running in namespace {}",
                    running as u32, total as u32, namespace
                ),
                vm_name: None,
                namespace: namespace.clone(),
                resolved: false,
            });
        }
    }

    // Sort by timestamp descending
    incidents.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let total = incidents.len() as u32;
    let open = incidents.iter().filter(|e| !e.resolved).count() as u32;
    let resolved_24h = incidents.iter().filter(|e| e.resolved).count() as u32;
    let critical = incidents
        .iter()
        .filter(|e| e.severity == "critical")
        .count() as u32;
    let warning = incidents.iter().filter(|e| e.severity == "warning").count() as u32;

    Json(IncidentTimeline {
        vmrogue_context: ctx,
        total_incidents: total,
        open_incidents: open,
        resolved_last_24h: resolved_24h,
        critical,
        warning,
        events: incidents,
    })
}
