#[cfg(feature = "web")]
use axum::{Json, Router, extract::{Path, State}, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// VMI (VirtualMachineInstance) response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmiResponse {
    pub name: String,
    pub namespace: String,
    pub phase: String,
    pub node_name: Option<String>,
    pub ip_address: Option<String>,
    pub cpu_cores: u32,
    pub memory: String,
    pub created_at: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/vmis", get(list_vmis))
        .route("/vmis/{name}", get(get_vmi))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_vmis(State(state): State<SharedState>) -> Json<Vec<VmiResponse>> {
    let s = state.read().await;
    let vmis = s.client().list_vmis(&s.namespace).await.unwrap_or_default();

    let results: Vec<VmiResponse> = vmis
        .iter()
        .map(|vmi| {
            let meta = &vmi.metadata;
            let status = vmi.status.as_ref();

            let phase = status
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("Unknown")
                .to_string();

            let node_name = status.and_then(|s| s.node_name.clone());

            let ip_address = status
                .and_then(|s| s.interfaces.first())
                .and_then(|i| i.ip_address.clone());

            VmiResponse {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                phase,
                node_name,
                ip_address,
                cpu_cores: vmi.spec.extra.get("domain")
                    .and_then(|d| d.get("cpu"))
                    .and_then(|c| c.get("cores"))
                    .and_then(|v| v.as_u64())
                    .map(|c| c as u32)
                    .unwrap_or(0),
                memory: vmi.spec.extra.get("domain")
                    .and_then(|d| d.get("resources"))
                    .and_then(|r| r.get("requests"))
                    .and_then(|req| req.get("memory"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| vmi.spec.extra.get("domain")
                        .and_then(|d| d.get("memory"))
                        .and_then(|m| m.get("guest"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()))
                    .unwrap_or_default(),
                created_at: meta
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            }
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn get_vmi(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Json<Option<VmiResponse>> {
    let s = state.read().await;
    match s.client().get_vmi(&s.namespace, &name).await {
        Ok(vmi) => {
            let meta = &vmi.metadata;
            let status = vmi.status.as_ref();

            Json(Some(VmiResponse {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                phase: status
                    .and_then(|s| s.phase.as_deref())
                    .unwrap_or("Unknown")
                    .to_string(),
                node_name: status.and_then(|s| s.node_name.clone()),
                ip_address: status
                    .and_then(|s| s.interfaces.first())
                    .and_then(|i| i.ip_address.clone()),
                cpu_cores: vmi.spec.extra.get("domain")
                    .and_then(|d| d.get("cpu"))
                    .and_then(|c| c.get("cores"))
                    .and_then(|v| v.as_u64())
                    .map(|c| c as u32)
                    .unwrap_or(0),
                memory: vmi.spec.extra.get("domain")
                    .and_then(|d| d.get("resources"))
                    .and_then(|r| r.get("requests"))
                    .and_then(|req| req.get("memory"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| vmi.spec.extra.get("domain")
                        .and_then(|d| d.get("memory"))
                        .and_then(|m| m.get("guest"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()))
                    .unwrap_or_default(),
                created_at: meta
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            }))
        }
        Err(_) => Json(None),
    }
}
