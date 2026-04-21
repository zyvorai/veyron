//! Snapshot schedules persisted as labeled ConfigMaps (`vmrogue.io/type=snapshot-schedule`).
//! Evaluated periodically by the web API while the process runs (UTC cron).

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::{
    Api, Client,
    api::{DeleteParams, ListParams, Patch, PatchParams, PostParams},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

/// Payload stored in ConfigMap `data["schedule.json"]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotScheduleRecord {
    pub vm_name: String,
    #[serde(default)]
    pub enabled: bool,
    /// Standard 5-field cron (UTC).
    pub cron: String,
    #[serde(default)]
    pub snapshot_prefix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run: Option<DateTime<Utc>>,
}

pub const SCHEDULE_CM_LABEL_TYPE: &str = "snapshot-schedule";
pub const SCHEDULE_CM_DATA_KEY: &str = "schedule.json";

fn schedule_json(data: &BTreeMap<String, String>) -> Result<SnapshotScheduleRecord> {
    let raw = data
        .get(SCHEDULE_CM_DATA_KEY)
        .context("missing schedule.json in ConfigMap")?;
    serde_json::from_str(raw).context("parse schedule.json")
}

fn store_schedule_json(rec: &SnapshotScheduleRecord) -> Result<BTreeMap<String, String>> {
    let mut data = BTreeMap::new();
    let s = serde_json::to_string_pretty(rec)?;
    data.insert(SCHEDULE_CM_DATA_KEY.to_string(), s);
    Ok(data)
}

/// List snapshot schedules across the cluster.
pub async fn list_schedule_configmaps(client: Client) -> Result<Vec<ConfigMap>> {
    let cms: Api<ConfigMap> = Api::all(client.clone());
    let lp = ListParams::default().labels("vmrogue.io/type=snapshot-schedule");
    Ok(cms.list(&lp).await?.items)
}

pub async fn delete_schedule_cm(client: Client, namespace: &str, name: &str) -> Result<()> {
    let cms: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    cms.delete(name, &DeleteParams::default()).await?;
    Ok(())
}

/// Create a snapshot schedule ConfigMap.
pub async fn upsert_schedule_cm(
    client: Client,
    namespace: &str,
    vm_name: &str,
    cron_expr: &str,
    snapshot_prefix: Option<&str>,
    enabled: bool,
) -> Result<String> {
    Schedule::from_str(cron_expr.trim()).context("invalid cron expression")?;

    let id: u64 = rand::random();
    let cm_name = format!("vmrogue-sschedule-{id:x}");

    let rec = SnapshotScheduleRecord {
        vm_name: vm_name.to_string(),
        enabled,
        cron: cron_expr.trim().to_string(),
        snapshot_prefix: snapshot_prefix.unwrap_or("sched").to_string(),
        last_run: None,
    };

    let data = store_schedule_json(&rec)?;

    let mut labels = BTreeMap::new();
    labels.insert(
        "vmrogue.io/type".to_string(),
        SCHEDULE_CM_LABEL_TYPE.to_string(),
    );
    labels.insert("vmrogue.io/vm".to_string(), vm_name.to_string());

    let cm = ConfigMap {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(cm_name.clone()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };

    let cms: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    cms.create(&PostParams::default(), &cm).await?;
    Ok(cm_name)
}

/// Periodically invoked from the HTTP server to create snapshots when cron matches (UTC).
pub async fn snapshot_schedule_tick(client: Client) -> Result<()> {
    let cms = list_schedule_configmaps(client.clone()).await?;

    let now = Utc::now();
    for cm in cms {
        let ns = cm.metadata.namespace.as_deref().unwrap_or("");
        let name = cm.metadata.name.as_deref().unwrap_or("");
        if ns.is_empty() || name.is_empty() {
            continue;
        }

        let data = match cm.data {
            Some(d) => d,
            None => continue,
        };

        let mut rec = match schedule_json(&data) {
            Ok(r) => r,
            Err(e) => {
                log::debug!("skip schedule CM {}: {}", name, e);
                continue;
            }
        };

        if !rec.enabled {
            continue;
        }

        let schedule = match Schedule::from_str(rec.cron.trim()) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("bad cron in {}: {}", name, e);
                continue;
            }
        };

        let anchor = rec.last_run.unwrap_or_else(|| {
            cm.metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.clone())
                .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap())
        });

        let upcoming = schedule.after(&anchor).next();
        let Some(next_time) = upcoming else {
            continue;
        };

        if next_time > now {
            continue;
        }

        if let Some(last) = rec.last_run {
            if (now - last).num_seconds() < 50 {
                continue;
            }
        }

        let mgr = crate::snapshots::SnapshotManager::from_client(client.clone(), ns);
        let snap_name = format!(
            "{}-{}-{}",
            rec.snapshot_prefix,
            rec.vm_name,
            now.format("%Y%m%d%H%M%S")
        );
        let cfg = crate::snapshots::SnapshotConfig::new(&rec.vm_name, &snap_name)
            .with_description("VMRogue scheduled snapshot");

        match mgr.create_snapshot(&cfg).await {
            Ok(_) => {
                rec.last_run = Some(now);
                let new_data = match store_schedule_json(&rec) {
                    Ok(d) => d,
                    Err(e) => {
                        log::warn!("serialize schedule {}: {}", name, e);
                        continue;
                    }
                };
                let patch = serde_json::json!({ "data": new_data });
                let cms_ns: Api<ConfigMap> = Api::namespaced(client.clone(), ns);
                let pp = PatchParams::default();
                if let Err(e) = cms_ns.patch(name, &pp, &Patch::Merge(patch)).await {
                    log::warn!("patch schedule CM {}: {}", name, e);
                } else {
                    log::info!(
                        "scheduled snapshot '{}' for VM {}/{}",
                        snap_name,
                        ns,
                        rec.vm_name
                    );
                }
            }
            Err(e) => log::warn!(
                "scheduled snapshot failed for VM {}/{}: {}",
                ns,
                rec.vm_name,
                e
            ),
        }
    }

    Ok(())
}
