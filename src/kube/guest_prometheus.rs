// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Optional Prometheus enrichment for guest filesystem views (PVC kubelet stats + guest exporters).

use std::collections::HashMap;

use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client, api::ListParams};

use super::guest_filesystem::GuestFilesystemMount;
use super::types::VirtualMachine;

/// PVC and optional in-guest exporter mounts from Prometheus.
pub async fn prometheus_storage_mounts_for_vm(
    client: &Client,
    prom_base: &str,
    namespace: &str,
    vm_name: &str,
    vm: &VirtualMachine,
) -> (Vec<GuestFilesystemMount>, Vec<String>) {
    let mut sources = Vec::new();
    let mut mounts = Vec::new();

    let pvc_mounts = prometheus_pvc_mounts(prom_base, namespace, vm).await;
    if !pvc_mounts.is_empty() {
        sources.push("prometheus-kubelet-pvc".into());
        mounts.extend(pvc_mounts);
    }

    if let Some(pod) = virt_launcher_pod_name(client, namespace, vm_name).await {
        let guest = prometheus_guest_exporter_mounts(prom_base, namespace, &pod).await;
        if !guest.is_empty() {
            sources.push("prometheus-guest-exporter".into());
            mounts.extend(guest);
        }
    }

    (mounts, sources)
}

async fn prometheus_pvc_mounts(
    prom_base: &str,
    namespace: &str,
    vm: &VirtualMachine,
) -> Vec<GuestFilesystemMount> {
    let pvcs = vm_bound_pvcs(vm);
    if pvcs.is_empty() {
        return Vec::new();
    }

    let used_q = format!("kubelet_volume_stats_used_bytes{{namespace=\"{namespace}\"}}");
    let cap_q = format!("kubelet_volume_stats_capacity_bytes{{namespace=\"{namespace}\"}}");

    let used = crate::api::prometheus::instant_query_vector(prom_base, &used_q)
        .await
        .unwrap_or_default();
    let cap = crate::api::prometheus::instant_query_vector(prom_base, &cap_q)
        .await
        .unwrap_or_default();

    let used_map = index_pvc_bytes(used);
    let cap_map = index_pvc_bytes(cap);

    let mut mounts = Vec::new();
    for (vol_name, pvc_name) in pvcs {
        let key = (namespace.to_string(), pvc_name.clone());
        let used_bytes = used_map.get(&key).copied().unwrap_or(0);
        let size_bytes = cap_map.get(&key).copied().unwrap_or(0);
        let avail_bytes = size_bytes.saturating_sub(used_bytes);
        let use_percent = if size_bytes > 0 {
            (used_bytes as f64 / size_bytes as f64) * 100.0
        } else {
            0.0
        };
        mounts.push(GuestFilesystemMount {
            mount: format!("pvc:{pvc_name} ({vol_name})"),
            filesystem: Some("kubernetes-pvc".into()),
            size_bytes,
            used_bytes,
            avail_bytes,
            use_percent,
        });
    }
    mounts
}

async fn prometheus_guest_exporter_mounts(
    prom_base: &str,
    namespace: &str,
    pod: &str,
) -> Vec<GuestFilesystemMount> {
    let mut mounts = Vec::new();
    let pod_filter = format!("{{namespace=\"{namespace}\",pod=\"{pod}\"}}");

    // Linux node_exporter-style (if scraped from the guest network namespace / sidecar).
    let linux_size_q = format!("node_filesystem_size_bytes{pod_filter}");
    let linux_avail_q = format!("node_filesystem_avail_bytes{pod_filter}");
    mounts.extend(merge_fs_pair(prom_base, &linux_size_q, &linux_avail_q, "node_exporter").await);

    // Windows perf counters (guest exporter / windows_exporter).
    let win_size_q = format!("windows_logical_disk_size_bytes{pod_filter}");
    let win_free_q = format!("windows_logical_disk_free_bytes{pod_filter}");
    mounts.extend(merge_fs_pair(prom_base, &win_size_q, &win_free_q, "windows_exporter").await);

    mounts
}

async fn merge_fs_pair(
    prom_base: &str,
    size_query: &str,
    avail_query: &str,
    filesystem: &str,
) -> Vec<GuestFilesystemMount> {
    let sizes = crate::api::prometheus::instant_query_vector(prom_base, size_query)
        .await
        .unwrap_or_default();
    let avails = crate::api::prometheus::instant_query_vector(prom_base, avail_query)
        .await
        .unwrap_or_default();

    let mut avail_by_mount: HashMap<String, u64> = HashMap::new();
    for (metric, v) in avails {
        let mount = mount_label(&metric);
        if !mount.is_empty() {
            avail_by_mount.insert(mount, v.max(0.0) as u64);
        }
    }

    let mut out = Vec::new();
    for (metric, size_v) in sizes {
        let mount = mount_label(&metric);
        if mount.is_empty() {
            continue;
        }
        let size_bytes = size_v.max(0.0) as u64;
        let avail_bytes = avail_by_mount.get(&mount).copied().unwrap_or(0);
        let used_bytes = size_bytes.saturating_sub(avail_bytes);
        let use_percent = if size_bytes > 0 {
            (used_bytes as f64 / size_bytes as f64) * 100.0
        } else {
            0.0
        };
        out.push(GuestFilesystemMount {
            mount,
            filesystem: Some(filesystem.into()),
            size_bytes,
            used_bytes,
            avail_bytes,
            use_percent,
        });
    }
    out
}

fn mount_label(metric: &HashMap<String, String>) -> String {
    metric
        .get("mountpoint")
        .or_else(|| metric.get("mount"))
        .or_else(|| metric.get("volume"))
        .or_else(|| metric.get("device"))
        .cloned()
        .unwrap_or_default()
}

fn index_pvc_bytes(samples: Vec<(HashMap<String, String>, f64)>) -> HashMap<(String, String), u64> {
    crate::api::prometheus::index_pvc_used_bytes(samples)
}

fn vm_bound_pvcs(vm: &VirtualMachine) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(vols) = vm.spec.template.spec.volumes.as_ref() {
        for vol in vols {
            let vol_name = vol.name.clone();
            if let Some(pvc) = vol.persistent_volume_claim.as_ref() {
                out.push((vol_name, pvc.claim_name.clone()));
            } else if let Some(dv) = vol.data_volume.as_ref() {
                out.push((vol_name, dv.name.clone()));
            }
        }
    }
    out
}

async fn virt_launcher_pod_name(client: &Client, namespace: &str, vm_name: &str) -> Option<String> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let lp = ListParams::default().labels(&format!("{}={vm_name}", crate::kube::VM_NAME_LABEL));
    let list = pods.list(&lp).await.ok()?;
    list.items.into_iter().find_map(|p| p.metadata.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_label_prefers_mountpoint() {
        let mut m = HashMap::new();
        m.insert("mountpoint".into(), "/".into());
        assert_eq!(mount_label(&m), "/");
    }
}
