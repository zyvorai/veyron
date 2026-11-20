// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Default `spec.lifecycle` for new VMs: a per-tenant (namespace annotation) or
//! cluster-wide (env) idle action and maximum lifetime, so lab and CI namespaces
//! don't accumulate forgotten VMs. A lifecycle set explicitly on the VM always wins.

use std::collections::BTreeMap;

use super::LifecycleSpec;

pub const ANN_IDLE_SECS: &str = "veyron.io/default-idle-seconds";
pub const ANN_IDLE_ACTION: &str = "veyron.io/default-idle-action";
pub const ANN_MAX_LIFETIME_SECS: &str = "veyron.io/default-max-lifetime-seconds";

const IDLE_ACTIONS: &[&str] = &["Pause", "Hibernate", "Halt", "Stop", "Delete"];

fn seconds(v: Option<&str>) -> Option<u64> {
    v.and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|n| *n >= 60)
}

/// Namespace annotation first, then the environment (`VEYRON_VM_DEFAULT_*`).
/// Returns `None` when neither names an idle timeout or a lifetime. An idle timeout
/// without an action pauses, which is reversible; `Delete` only applies when asked for.
pub fn lifecycle_defaults(
    annotations: &BTreeMap<String, String>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Option<LifecycleSpec> {
    let pick = |ann: &str, var: &str| -> Option<String> {
        annotations.get(ann).cloned().or_else(|| env(var))
    };
    let idle = seconds(pick(ANN_IDLE_SECS, "VEYRON_VM_DEFAULT_IDLE_SECS").as_deref());
    let max =
        seconds(pick(ANN_MAX_LIFETIME_SECS, "VEYRON_VM_DEFAULT_MAX_LIFETIME_SECS").as_deref());
    if idle.is_none() && max.is_none() {
        return None;
    }
    let action = pick(ANN_IDLE_ACTION, "VEYRON_VM_DEFAULT_IDLE_ACTION")
        .filter(|a| IDLE_ACTIONS.contains(&a.as_str()))
        .unwrap_or_else(|| "Pause".into());
    Some(LifecycleSpec {
        idle_timeout_seconds: idle,
        idle_action: idle.map(|_| action),
        idle_cpu_millicores: None,
        max_lifetime_seconds: max,
        wake_on_connect: idle.map(|_| true),
        wake_timeout_seconds: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn nothing_configured_means_no_lifecycle() {
        assert!(lifecycle_defaults(&BTreeMap::new(), &no_env).is_none());
    }

    #[test]
    fn namespace_annotation_beats_env() {
        let ann = BTreeMap::from([(ANN_IDLE_SECS.to_string(), "600".to_string())]);
        let env = |k: &str| (k == "VEYRON_VM_DEFAULT_IDLE_SECS").then(|| "3600".to_string());
        let l = lifecycle_defaults(&ann, &env).unwrap();
        assert_eq!(l.idle_timeout_seconds, Some(600));
        assert_eq!(l.idle_action.as_deref(), Some("Pause"));
        assert_eq!(l.wake_on_connect, Some(true));
    }

    #[test]
    fn lifetime_only_has_no_idle_action() {
        let env =
            |k: &str| (k == "VEYRON_VM_DEFAULT_MAX_LIFETIME_SECS").then(|| "86400".to_string());
        let l = lifecycle_defaults(&BTreeMap::new(), &env).unwrap();
        assert_eq!(l.max_lifetime_seconds, Some(86400));
        assert!(l.idle_timeout_seconds.is_none() && l.idle_action.is_none());
    }

    #[test]
    fn rejects_short_values_and_unknown_actions() {
        let ann = BTreeMap::from([
            (ANN_IDLE_SECS.to_string(), "5".to_string()),
            (ANN_MAX_LIFETIME_SECS.to_string(), "7200".to_string()),
            (ANN_IDLE_ACTION.to_string(), "Explode".to_string()),
        ]);
        let l = lifecycle_defaults(&ann, &no_env).unwrap();
        assert!(l.idle_timeout_seconds.is_none());
        assert_eq!(l.max_lifetime_seconds, Some(7200));
        let ann = BTreeMap::from([
            (ANN_IDLE_SECS.to_string(), "300".to_string()),
            (ANN_IDLE_ACTION.to_string(), "Explode".to_string()),
        ]);
        assert_eq!(
            lifecycle_defaults(&ann, &no_env)
                .unwrap()
                .idle_action
                .as_deref(),
            Some("Pause")
        );
    }
}
