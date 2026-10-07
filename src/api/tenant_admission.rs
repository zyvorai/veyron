// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
//! Reserve VM capacity with ConfigMap resourceVersion CAS across API replicas.
//! Ambiguous creates retain reservations until inventory observes the VM or an administrator reviews them.
use crate::api::vm_backend::selected;
use crate::{
    config::VMConfig,
    enterprise::planning::{Capacity, quantity},
    kube::KubeClient,
};
use anyhow::{Result, anyhow, bail};
use k8s_openapi::api::core::v1::{ConfigMap, Namespace, ResourceQuota};
#[cfg(feature = "kairon")]
use kube::api::ListParams;
use kube::{Api, api::PostParams};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
const LEDGER: &str = "veyron-vm-admission";
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Ledger {
    pending: BTreeMap<String, Capacity>,
}
fn add(total: &mut Capacity, r: &Capacity) -> Result<()> {
    total.cpu_millis = total
        .cpu_millis
        .checked_add(r.cpu_millis)
        .ok_or_else(|| anyhow!("CPU overflow"))?;
    total.memory_bytes = total
        .memory_bytes
        .checked_add(r.memory_bytes)
        .ok_or_else(|| anyhow!("memory overflow"))?;
    total.gpu = total
        .gpu
        .checked_add(r.gpu)
        .ok_or_else(|| anyhow!("GPU overflow"))?;
    Ok(())
}
fn number(v: &Value, path: &str, default: u64) -> Result<u64> {
    match v.pointer(path) {
        None => Ok(default),
        Some(v) => v
            .as_u64()
            .ok_or_else(|| anyhow!("invalid resource field {path}")),
    }
}
fn q(v: &Value, path: &str, cpu: bool) -> Result<u64> {
    let s = v
        .pointer(path)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("resource sizing unresolved at {path}"))?;
    quantity(s, cpu).map_err(|e| anyhow!(e))
}
fn object_size(v: &Value, kairon: bool) -> Result<Capacity> {
    if kairon {
        let cpu = q(
            v,
            if v.pointer("/spec/resources/maxCpu").is_some() {
                "/spec/resources/maxCpu"
            } else {
                "/spec/resources/cpu"
            },
            true,
        )?;
        let mem = q(
            v,
            if v.pointer("/spec/resources/maxMemory").is_some() {
                "/spec/resources/maxMemory"
            } else {
                "/spec/resources/memory"
            },
            false,
        )?;
        Ok(Capacity {
            cpu_millis: cpu,
            memory_bytes: mem,
            gpu: v
                .pointer("/spec/deviceClaims")
                .and_then(Value::as_array)
                .map_or(0, |a| a.len() as u64),
        })
    } else {
        if v.pointer("/spec/instancetype").is_some() {
            bail!("cannot safely account unresolved instancetype VM");
        }
        let base = "/spec/template/spec/domain";
        let cpu = if v.pointer(&format!("{base}/cpu")).is_some() {
            let cores = number(v, &format!("{base}/cpu/cores"), 1)?;
            let sockets = number(
                v,
                &format!("{base}/cpu/maxSockets"),
                number(v, &format!("{base}/cpu/sockets"), 1)?,
            )?;
            cores
                .checked_mul(sockets)
                .and_then(|n| n.checked_mul(number(v, &format!("{base}/cpu/threads"), 1).ok()?))
                .and_then(|n| n.checked_mul(1000))
                .ok_or_else(|| anyhow!("CPU overflow"))?
        } else {
            q(v, &format!("{base}/resources/requests/cpu"), true)?
        };
        let mem = [
            "/memory/maxGuest",
            "/memory/guest",
            "/resources/requests/memory",
        ]
        .iter()
        .find_map(|s| {
            let p = format!("{base}{s}");
            v.pointer(&p).map(|_| p)
        })
        .ok_or_else(|| anyhow!("memory unresolved"))?;
        if v.pointer(&format!("{base}/devices/hostDevices"))
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
        {
            bail!("host device accounting requires administrator review");
        }
        Ok(Capacity {
            cpu_millis: cpu,
            memory_bytes: q(v, &mem, false)?,
            gpu: v
                .pointer(&format!("{base}/devices/gpus"))
                .and_then(Value::as_array)
                .map_or(0, |a| a.len() as u64),
        })
    }
}
fn config_size(c: &VMConfig) -> Result<Capacity> {
    if c.instancetype.is_some() || !c.host_devices.is_empty() {
        bail!("managed tenant quotas require explicit sizing and no unclassified host devices");
    }
    let cpu = u64::from(c.cpu.cores)
        .checked_mul(u64::from(c.cpu.sockets))
        .and_then(|n| n.checked_mul(u64::from(c.cpu.threads)))
        .and_then(|n| n.checked_mul(1000))
        .ok_or_else(|| anyhow!("CPU overflow"))?;
    let mem = quantity(
        c.memory.max_guest.as_deref().unwrap_or(&c.memory.size),
        false,
    )
    .map_err(|e| anyhow!(e))?;
    let mem = mem.max(quantity(&c.memory.size, false).map_err(|e| anyhow!(e))?);
    if cpu == 0 || mem == 0 {
        bail!("positive CPU and memory required");
    }
    Ok(Capacity {
        cpu_millis: cpu,
        memory_bytes: mem,
        gpu: c.gpus.len() as u64,
    })
}
fn fits(total: &Capacity, count: u64, limits: &Capacity, max_vms: u64) -> bool {
    limits.fits(total) && count <= max_vms
}

pub async fn reserve(client: &KubeClient, config: &VMConfig) -> Result<()> {
    let ns = Api::<Namespace>::all(client.client())
        .get(&config.namespace)
        .await?;
    if !ns
        .metadata
        .labels
        .as_ref()
        .is_some_and(|l| l.contains_key("veyron.io/tenant"))
    {
        return Ok(());
    }
    if ns
        .metadata
        .labels
        .as_ref()
        .and_then(|l| l.get("veyron.io/vm-backend"))
        .map(String::as_str)
        != Some(selected().as_str())
    {
        bail!(
            "managed namespace backend label is missing or mismatched; inspect inventory before adopting"
        );
    }
    let quota = Api::<ResourceQuota>::namespaced(client.client(), &config.namespace)
        .get("tenant-quota")
        .await?;
    let hard = quota
        .spec
        .and_then(|s| s.hard)
        .ok_or_else(|| anyhow!("tenant quota has no limits"))?;
    let get = |key: &str, cpu: bool| -> Result<u64> {
        quantity(
            &hard
                .get(key)
                .ok_or_else(|| anyhow!("missing quota {key}"))?
                .0,
            cpu,
        )
        .map_err(|e| anyhow!(e))
    };
    let limits = Capacity {
        cpu_millis: get("limits.cpu", true)?,
        memory_bytes: get("limits.memory", false)?,
        gpu: get("requests.nvidia.com/gpu", false)?,
    };
    let count_key = if selected().as_str() == "kairon" {
        "count/machines.kairon.zyvor.dev"
    } else {
        "count/virtualmachines.kubevirt.io"
    };
    let max_vms = get(count_key, false)?;
    let request = config_size(config)?;
    let api = Api::<ConfigMap>::namespaced(client.client(), &config.namespace);
    for _ in 0..10 {
        let existing = api.get_opt(LEDGER).await?;
        let mut cm = existing.clone().unwrap_or_else(|| ConfigMap {
            metadata: kube::api::ObjectMeta {
                name: Some(LEDGER.into()),
                namespace: Some(config.namespace.clone()),
                labels: Some(BTreeMap::from([(
                    "veyron.io/type".into(),
                    "tenant-admission".into(),
                )])),
                ..Default::default()
            },
            ..Default::default()
        });
        let mut ledger: Ledger = match cm.data.as_ref().and_then(|d| d.get("ledger.json")) {
            Some(raw) => serde_json::from_str(raw)?,
            None if existing.is_none() => Ledger::default(),
            None => bail!("tenant admission ledger corrupt"),
        };
        let mut inventory = BTreeMap::new();
        if selected().as_str() == "kairon" {
            #[cfg(feature = "kairon")]
            for vm in Api::<crate::kairon::Machine>::namespaced(client.client(), &config.namespace)
                .list(&ListParams::default())
                .await?
                .items
            {
                let name = vm
                    .metadata
                    .name
                    .clone()
                    .ok_or_else(|| anyhow!("Machine missing name"))?;
                inventory.insert(name, object_size(&serde_json::to_value(vm)?, true)?);
            }
        } else {
            for vm in client.list_vms(&config.namespace).await? {
                let name = vm
                    .metadata
                    .name
                    .clone()
                    .ok_or_else(|| anyhow!("VM missing name"))?;
                inventory.insert(name, object_size(&serde_json::to_value(vm)?, false)?);
            }
        }
        ledger
            .pending
            .retain(|name, _| !inventory.contains_key(name));
        if inventory.contains_key(&config.name) || ledger.pending.contains_key(&config.name) {
            bail!("VM already exists or a pending create requires review");
        }
        let mut total = request.clone();
        for r in inventory.values().chain(ledger.pending.values()) {
            add(&mut total, r)?;
        }
        if !fits(
            &total,
            inventory.len() as u64 + ledger.pending.len() as u64 + 1,
            &limits,
            max_vms,
        ) {
            bail!("tenant VM/CPU/memory/GPU quota exceeded");
        }
        ledger.pending.insert(config.name.clone(), request.clone());
        cm.data
            .get_or_insert_with(BTreeMap::new)
            .insert("ledger.json".into(), serde_json::to_string(&ledger)?);
        let result = if existing.is_some() {
            api.replace(LEDGER, &PostParams::default(), &cm).await
        } else {
            api.create(&PostParams::default(), &cm).await
        };
        match result {
            Ok(_) => return Ok(()),
            Err(kube::Error::Api(e)) if e.code == 409 => continue,
            Err(e) => return Err(e.into()),
        }
    }
    bail!("tenant admission busy; retry later")
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn quota_checks_gpu_and_vm_count() {
        let r = Capacity {
            cpu_millis: 1000,
            memory_bytes: 1024,
            gpu: 1,
        };
        assert!(fits(&r, 1, &r, 1));
        assert!(!fits(&r, 2, &r, 1));
        assert!(!fits(
            &r,
            1,
            &Capacity {
                gpu: 0,
                ..r.clone()
            },
            1
        ));
    }
    #[test]
    fn kairon_accounts_maximum_hotplug_capacity() {
        let v = json!({"spec":{"resources":{"cpu":"2","maxCpu":"8","memory":"2Gi","maxMemory":"16Gi"}}});
        let size = object_size(&v, true).unwrap();
        assert_eq!(size.cpu_millis, 8000);
        assert_eq!(size.memory_bytes, 17179869184);
    }
    #[test]
    fn kubevirt_uses_full_topology() {
        let v = json!({"spec":{"template":{"spec":{"domain":{"cpu":{"cores":2,"sockets":2,"threads":2},"resources":{"requests":{"memory":"4Gi"}}}}}}});
        assert_eq!(object_size(&v, false).unwrap().cpu_millis, 8000);
    }
    #[test]
    fn unresolved_instancetype_fails_closed() {
        assert!(object_size(&json!({"spec":{"instancetype":{"name":"large"}}}), false).is_err());
    }
}
