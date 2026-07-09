// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Atlas storage control-plane client.
//!
//! Atlas (`../atlas`, the Zyvor storage control plane) fronts Ceph (RBD/CephFS/RGW),
//! NFS and ZFS behind stable REST/gRPC APIs. Veyron calls Atlas to drive **Ceph-backed**
//! VM disk snapshots, clones, restores and off-cluster backups (RBD `export-diff` → RGW/S3)
//! instead of re-implementing Ceph itself. This is the additive HTTP surface: it never
//! replaces Veyron's native KubeVirt `VolumeSnapshot` flow — it sits alongside it.
//!
//! Configuration mirrors the PacketWolf integration:
//! - `VEYRON_ATLAS_URL`   — Atlas gateway base URL, e.g. `http://atlas-gateway.zyvor:5110`
//! - `VEYRON_ATLAS_TOKEN` — optional HS256 JWT bearer (required when `ATLAS_AUTH_REQUIRED=1`)
//! - `VEYRON_ATLAS_TENANT`— optional tenant id used for created volumes/backups (default `global`)
//!
//! The wire contract is Atlas REST v1 (`/api/atlas/v1`); see `../atlas/docs/API.md`.

#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};

/// Product identifier recorded in Atlas ownership bindings (`owner.product`).
pub const ATLAS_PRODUCT: &str = "veyron";

#[cfg(feature = "web")]
pub fn atlas_base_url() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_ATLAS_URL")
}

#[cfg(feature = "web")]
pub fn atlas_token() -> Option<String> {
    crate::api::integrations::env_var("VEYRON_ATLAS_TOKEN")
}

#[cfg(feature = "web")]
pub fn atlas_tenant() -> String {
    crate::api::integrations::env_var("VEYRON_ATLAS_TENANT").unwrap_or_else(|| "global".to_string())
}

#[cfg(feature = "web")]
pub fn atlas_configured() -> bool {
    atlas_base_url().is_some()
}

/// Reject obviously-unsafe base URLs. Atlas is an **admin-configured** in-cluster backend
/// (typically a ClusterIP service, or `127.0.0.1:5110` in dev), not an attacker-controlled
/// proxy target, so — unlike the PacketWolf proxy validator — loopback is allowed. We still
/// block non-http(s) schemes and the cloud metadata endpoint to keep SSRF surface minimal.
#[cfg(feature = "web")]
pub fn validate_atlas_url(url: &str) -> Result<(), String> {
    let lower = url.trim().to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(format!("unsafe Atlas URL scheme: {url}"));
    }
    let host_part = lower
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .split('@')
        .next_back()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    if host_part == "169.254.169.254" || host_part.starts_with("metadata.") {
        return Err("Atlas URL points at a cloud metadata endpoint".to_string());
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// Wire DTOs — a pragmatic subset of Atlas REST v1 (see ../atlas/docs/API.md).
// Only the fields Veyron consumes are typed; the rest is ignored.
// ----------------------------------------------------------------------------

/// An Atlas-inventoried storage volume. For Ceph-backed VM disks the `(kubernetes_namespace,
/// pvc_name)` pair is what Veyron matches a KubeVirt VM's PVC against.
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasVolume {
    pub id: String,
    #[serde(default)]
    pub cluster_id: Option<String>,
    #[serde(default)]
    pub pool_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub backend_native_id: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub used_bytes: Option<u64>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub health: Option<String>,
    #[serde(default)]
    pub kubernetes_namespace: Option<String>,
    #[serde(default)]
    pub pvc_name: Option<String>,
    #[serde(default)]
    pub storage_class_name: Option<String>,
}

/// Atlas async job (state machine: pending → queued → running → verifying → succeeded|failed).
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasJob {
    pub id: String,
    pub state: String,
    #[serde(default)]
    pub progress_percent: Option<u32>,
    #[serde(default, alias = "job_type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default, alias = "result")]
    pub resource: Option<serde_json::Value>,
}

/// 202-Accepted envelope returned by Atlas write endpoints.
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasJobAck {
    pub job_id: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub resource: Option<serde_json::Value>,
    #[serde(default)]
    pub links: Option<serde_json::Value>,
}

#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AtlasCapabilities {
    #[serde(default)]
    pub block: bool,
    #[serde(default)]
    pub file: bool,
    #[serde(default)]
    pub object: bool,
    #[serde(default)]
    pub snapshots: bool,
    #[serde(default)]
    pub clone: bool,
    #[serde(default)]
    pub expansion: bool,
    #[serde(default)]
    pub replication: bool,
}

#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasBackend {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub backend_type: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub capabilities: Option<AtlasCapabilities>,
}

// ----------------------------------------------------------------------------
// Client
// ----------------------------------------------------------------------------

/// Thin typed client over Atlas REST v1. Cheap to construct (`AtlasClient::from_env`);
/// resolves config from `VEYRON_ATLAS_*` on each build.
#[cfg(feature = "web")]
#[derive(Clone)]
pub struct AtlasClient {
    base: String,
    token: Option<String>,
}

#[cfg(feature = "web")]
impl AtlasClient {
    /// Build a client from env, or `None` when `VEYRON_ATLAS_URL` is unset/invalid.
    pub fn from_env() -> Option<Self> {
        let base = atlas_base_url()?;
        validate_atlas_url(&base).ok()?;
        Some(Self {
            base: base.trim_end_matches('/').to_string(),
            token: atlas_token(),
        })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/atlas/v1{}", self.base, path)
    }

    async fn client() -> Result<reqwest::Client, String> {
        crate::api::integrations::http_client()
            .await
            .map_err(|e| format!("HTTP client unavailable: {e}"))
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.token.as_ref().filter(|t| !t.trim().is_empty()) {
            Some(t) => req.bearer_auth(t),
            None => req,
        }
    }

    /// `GET /health` — returns the raw JSON body on success, `None` when unreachable.
    pub async fn health(&self) -> Option<serde_json::Value> {
        let client = Self::client().await.ok()?;
        let req = self.auth(client.get(format!("{}/health", self.base)));
        match req.send().await {
            Ok(resp) if resp.status().is_success() => resp.json().await.ok(),
            _ => None,
        }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, String> {
        let client = Self::client().await?;
        let mut req = self.auth(client.get(self.url(path)));
        if !query.is_empty() {
            req = req.query(query);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("Atlas request failed: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Atlas returned {status}: {}",
                body.trim().chars().take(200).collect::<String>()
            ));
        }
        resp.json::<T>()
            .await
            .map_err(|e| format!("invalid Atlas JSON: {e}"))
    }

    async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, String> {
        let client = Self::client().await?;
        let req = self.auth(client.post(self.url(path)).json(body));
        let resp = req
            .send()
            .await
            .map_err(|e| format!("Atlas request failed: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Atlas returned {status}: {}",
                text.trim().chars().take(300).collect::<String>()
            ));
        }
        resp.json::<T>()
            .await
            .map_err(|e| format!("invalid Atlas JSON: {e}"))
    }

    async fn delete(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<serde_json::Value, String> {
        let client = Self::client().await?;
        let mut req = self.auth(client.delete(self.url(path)));
        if !query.is_empty() {
            req = req.query(query);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("Atlas request failed: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Atlas returned {status}: {}",
                text.trim().chars().take(300).collect::<String>()
            ));
        }
        Ok(resp.json().await.unwrap_or(serde_json::Value::Null))
    }

    // -- Backends / inventory --------------------------------------------------

    pub async fn list_backends(&self) -> Result<Vec<AtlasBackend>, String> {
        self.get_json("/backends", &[]).await
    }

    pub async fn list_volumes(
        &self,
        filters: &[(&str, String)],
    ) -> Result<Vec<AtlasVolume>, String> {
        self.get_json("/volumes", filters).await
    }

    /// `GET /buckets` — RGW/S3 buckets available as backup targets.
    pub async fn list_buckets(&self) -> Result<serde_json::Value, String> {
        self.get_json("/buckets", &[]).await
    }

    /// Find the Atlas volume backing a given Kubernetes PVC, matching on
    /// `(kubernetes_namespace, pvc_name)`. Returns `None` when Atlas has no such
    /// inventory row (e.g. the disk is not Ceph-backed, or discovery hasn't run).
    pub async fn find_volume_for_pvc(
        &self,
        namespace: &str,
        pvc_name: &str,
    ) -> Result<Option<AtlasVolume>, String> {
        let vols = self.list_volumes(&[("kind", "block".to_string())]).await?;
        Ok(vols.into_iter().find(|v| {
            v.pvc_name.as_deref() == Some(pvc_name)
                && v.kubernetes_namespace
                    .as_deref()
                    .map(|n| n == namespace)
                    .unwrap_or(true)
        }))
    }

    // -- Snapshots -------------------------------------------------------------

    pub async fn list_snapshots(&self) -> Result<serde_json::Value, String> {
        self.get_json("/snapshots", &[]).await
    }

    /// `POST /volumes/{id}/snapshots` — snapshot the volume's PVC (async job).
    pub async fn create_snapshot(
        &self,
        volume_id: &str,
        name: Option<&str>,
        snapshot_class: Option<&str>,
    ) -> Result<AtlasJobAck, String> {
        let mut body = serde_json::Map::new();
        if let Some(n) = name.filter(|s| !s.trim().is_empty()) {
            body.insert("name".into(), n.into());
        }
        if let Some(sc) = snapshot_class.filter(|s| !s.trim().is_empty()) {
            body.insert("snapshot_class".into(), sc.into());
        }
        self.post_json(
            &format!("/volumes/{volume_id}/snapshots"),
            &serde_json::Value::Object(body),
        )
        .await
    }

    /// `POST /snapshots/{id}/restore` — point-in-time copy of the source volume.
    pub async fn restore_snapshot(
        &self,
        snapshot_id: &str,
        body: &serde_json::Value,
    ) -> Result<AtlasJobAck, String> {
        self.post_json(&format!("/snapshots/{snapshot_id}/restore"), body)
            .await
    }

    /// `POST /snapshots/{id}/clone` — new independent volume from the snapshot.
    pub async fn clone_snapshot(
        &self,
        snapshot_id: &str,
        body: &serde_json::Value,
    ) -> Result<AtlasJobAck, String> {
        self.post_json(&format!("/snapshots/{snapshot_id}/clone"), body)
            .await
    }

    pub async fn delete_snapshot(
        &self,
        snapshot_id: &str,
        force: bool,
    ) -> Result<serde_json::Value, String> {
        let query: Vec<(&str, String)> = if force {
            vec![("force", "true".to_string())]
        } else {
            vec![]
        };
        self.delete(&format!("/snapshots/{snapshot_id}"), &query)
            .await
    }

    // -- Backups (RGW / S3) ----------------------------------------------------

    pub async fn list_backups(&self, volume_id: Option<&str>) -> Result<serde_json::Value, String> {
        let query: Vec<(&str, String)> = match volume_id {
            Some(v) => vec![("volume_id", v.to_string())],
            None => vec![],
        };
        self.get_json("/backups", &query).await
    }

    /// `POST /backup-jobs` — snapshot + write a backup to a bound RGW bucket.
    /// `mode` = `"data"` exports real RBD image data (`rbd export-diff`) to S3.
    pub async fn create_backup_job(&self, body: &serde_json::Value) -> Result<AtlasJobAck, String> {
        self.post_json("/backup-jobs", body).await
    }

    // -- Jobs ------------------------------------------------------------------

    pub async fn get_job(&self, job_id: &str) -> Result<AtlasJob, String> {
        self.get_json(&format!("/jobs/{job_id}"), &[]).await
    }
}

/// Live integration checks against a running Atlas gateway. These are **skipped**
/// unless `VEYRON_ATLAS_URL` is set (e.g. the fake gateway on `127.0.0.1:5110`),
/// so `cargo test` in CI without Atlas is a no-op. Run locally with:
/// `ATLAS_CEPH_DRIVER_MODE=fake cargo run -p atlas-gateway` then
/// `VEYRON_ATLAS_URL=http://127.0.0.1:5110 cargo test --features web atlas_live -- --nocapture`.
#[cfg(all(test, feature = "web"))]
mod live_tests {
    use super::*;

    fn client() -> Option<AtlasClient> {
        // Only run when explicitly pointed at a gateway.
        std::env::var("VEYRON_ATLAS_URL").ok()?;
        AtlasClient::from_env()
    }

    #[tokio::test]
    async fn atlas_live_health_and_inventory() {
        let Some(c) = client() else {
            eprintln!("VEYRON_ATLAS_URL unset — skipping live Atlas test");
            return;
        };
        assert!(c.health().await.is_some(), "gateway health probe failed");

        let backends = c.list_backends().await.expect("list backends");
        assert!(
            backends
                .iter()
                .any(|b| b.backend_type.as_deref() == Some("ceph")),
            "expected a ceph backend in {backends:?}"
        );

        let vols = c
            .list_volumes(&[("kind", "block".to_string())])
            .await
            .expect("list volumes");
        assert!(!vols.is_empty(), "expected at least one block volume");

        // Buckets + snapshots endpoints return JSON arrays (possibly empty).
        assert!(c.list_buckets().await.expect("list buckets").is_array());
        assert!(c.list_snapshots().await.expect("list snapshots").is_array());

        // The (namespace, pvc) matcher used to map a VM disk → Atlas volume.
        let sample = &vols[0];
        if let (Some(ns), Some(pvc)) =
            (sample.kubernetes_namespace.clone(), sample.pvc_name.clone())
        {
            let found = c
                .find_volume_for_pvc(&ns, &pvc)
                .await
                .expect("find_volume_for_pvc");
            assert_eq!(found.map(|v| v.id), Some(sample.id.clone()));
        }
    }

    #[tokio::test]
    async fn atlas_live_snapshot_ack_shape() {
        let Some(c) = client() else {
            return;
        };
        let vols = c
            .list_volumes(&[("kind", "block".to_string())])
            .await
            .expect("list volumes");
        let Some(vol) = vols.first() else { return };

        // Enqueues a job; in fake mode (no cluster) it later fails on the write path,
        // but the 202 ack + job lookup are what we assert here.
        let ack = c
            .create_snapshot(&vol.id, Some("veyron-live-test"), None)
            .await
            .expect("create_snapshot ack");
        assert!(!ack.job_id.is_empty(), "expected a job id in ack");

        let job = c.get_job(&ack.job_id).await.expect("get_job");
        assert_eq!(job.id, ack.job_id);
    }
}
