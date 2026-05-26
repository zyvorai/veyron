// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Leader election for the in-process snapshot scheduler using a `coordination.k8s.io` **Lease**
//! so only one API replica runs `snapshot_schedule_tick` (avoids duplicate `VirtualMachineSnapshot` CRs).
//!
//! - **RBAC:** the service account needs `leases` in the lease namespace (see Helm `ClusterRole`).
//! - **Dev / no RBAC:** set `VMROGUE_SCHEDULER_LEASE_DISABLED=1` to skip the lease (not safe with multiple replicas).
//! - **Namespace:** `VMROGUE_SCHEDULER_LEASE_NAMESPACE` (defaults to the API’s default namespace).

use anyhow::Result;
use chrono::{DateTime, Utc};
use k8s_openapi::api::coordination::v1::{Lease, LeaseSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{MicroTime, ObjectMeta};
use kube::Client;
use kube::api::{Api, PostParams};

const LEASE_NAME: &str = "vmrogue-snapshot-scheduler";
const DEFAULT_LEASE_SECS: i32 = 90;
const MAX_LEASE_RETRIES: u8 = 8;

/// Unique per process; used as `spec.holderIdentity`.
pub fn scheduler_holder_identity() -> String {
    let host = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "localhost".to_string());
    format!("{host}-{}", std::process::id())
}

/// Returns `true` if this instance should run the scheduler tick (holds or acquired the lease).
pub async fn acquire_snapshot_scheduler_leader(
    client: Client,
    namespace: &str,
    identity: &str,
) -> Result<bool> {
    if namespace.is_empty() {
        anyhow::bail!("lease namespace is empty");
    }

    let leases: Api<Lease> = Api::namespaced(client, namespace);
    let now: DateTime<Utc> = Utc::now();
    let pp = PostParams::default();

    for _ in 0..MAX_LEASE_RETRIES {
        match leases.get(LEASE_NAME).await {
            Ok(lease) => {
                let spec = lease.spec.as_ref();
                let holder = spec.and_then(|s| s.holder_identity.as_deref());
                let renew: Option<DateTime<Utc>> =
                    spec.and_then(|s| s.renew_time.as_ref()).map(|m| m.0);
                let dur_secs = i64::from(
                    spec.and_then(|s| s.lease_duration_seconds)
                        .unwrap_or(DEFAULT_LEASE_SECS),
                );

                let expired = renew
                    .map(|rt| now.signed_duration_since(rt) > chrono::Duration::seconds(dur_secs))
                    .unwrap_or(true);

                if holder == Some(identity) {
                    let mut new_lease = lease.clone();
                    {
                        let s = new_lease.spec.get_or_insert(LeaseSpec::default());
                        s.renew_time = Some(MicroTime(now));
                    }
                    match leases.replace(LEASE_NAME, &pp, &new_lease).await {
                        Ok(_) => return Ok(true),
                        Err(kube::Error::Api(ae)) if ae.code == 409 => continue,
                        Err(e) => return Err(e.into()),
                    }
                } else if expired {
                    let mut new_lease = lease.clone();
                    {
                        let s = new_lease.spec.get_or_insert(LeaseSpec::default());
                        s.holder_identity = Some(identity.to_string());
                        s.renew_time = Some(MicroTime(now));
                        s.lease_duration_seconds = Some(DEFAULT_LEASE_SECS);
                    }
                    match leases.replace(LEASE_NAME, &pp, &new_lease).await {
                        Ok(_) => return Ok(true),
                        Err(kube::Error::Api(ae)) if ae.code == 409 => continue,
                        Err(e) => return Err(e.into()),
                    }
                } else {
                    return Ok(false);
                }
            }
            Err(kube::Error::Api(ae)) if ae.code == 404 => {
                let new_lease = Lease {
                    metadata: ObjectMeta {
                        name: Some(LEASE_NAME.to_string()),
                        namespace: Some(namespace.to_string()),
                        ..Default::default()
                    },
                    spec: Some(LeaseSpec {
                        holder_identity: Some(identity.to_string()),
                        lease_duration_seconds: Some(DEFAULT_LEASE_SECS),
                        renew_time: Some(MicroTime(now)),
                        ..Default::default()
                    }),
                };
                match leases.create(&pp, &new_lease).await {
                    Ok(_) => return Ok(true),
                    Err(kube::Error::Api(ae)) if ae.code == 409 => continue,
                    Err(e) => return Err(e.into()),
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    Err(anyhow::anyhow!(
        "exhausted lease acquire retries for {LEASE_NAME}"
    ))
}
