// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Named network/storage mapping profiles for imports, stored as ConfigMaps.
//!
//! A profile translates names from the source platform to Kairon: a source network (a vSphere
//! port group, a Hyper-V switch) becomes a `tap` or `macvtap` attachment, and a source
//! datastore becomes a storage pool label. An import request names a profile with
//! `"mapping": "dc1"`; Veyron resolves `source.network` and `source.datastore` through it.
//!
//! Each profile is one ConfigMap `import-mapping-<name>` labelled
//! `veyron.io/type=import-mapping` in Veyron's namespace. Everything except the
//! `KubeMappingStore` is pure and runs without a cluster.

use std::collections::BTreeMap;
use std::future::Future;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, DeleteParams, ListParams, PostParams};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::import_lifecycle::ApiErr;
use super::imports::valid_name;
use crate::api::http_server::web::SharedState;
use crate::kairon::Machine;

pub const MAPPING_TYPE_LABEL: &str = "veyron.io/type";
pub const MAPPING_TYPE: &str = "import-mapping";
pub const MAPPING_ANNOTATION: &str = "veyron.io/import-mapping";
pub const SOURCE_NETWORK_ANNOTATION: &str = "veyron.io/import-source-network";
pub const SOURCE_DATASTORE_ANNOTATION: &str = "veyron.io/import-source-datastore";
pub const STORAGE_POOL_ANNOTATION: &str = "veyron.io/import-storage-pool";
const MAX_ENTRIES: usize = 256;

/// Where a source network lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTarget {
    /// `tap`, `macvtap` or `user`.
    pub mode: String,
    /// Host bridge (`tap`) or parent interface (`macvtap`). Required for both.
    #[serde(default)]
    pub bridge: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mapping {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Source network name -> target.
    #[serde(default)]
    pub networks: BTreeMap<String, NetworkTarget>,
    /// Source datastore name -> storage pool.
    #[serde(default)]
    pub datastores: BTreeMap<String, String>,
    /// Used when the request's `source.network` is absent or not listed.
    #[serde(default)]
    pub default_network: Option<NetworkTarget>,
    #[serde(default)]
    pub default_pool: Option<String>,
}

pub fn configmap_name(name: &str) -> String {
    format!("import-mapping-{name}")
}

impl Mapping {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_name(&self.name) || self.name.len() > 40 {
            return Err(format!(
                "mapping name {:?} must be a DNS-1123 label of at most 40 characters",
                self.name
            ));
        }
        if self.networks.len() > MAX_ENTRIES || self.datastores.len() > MAX_ENTRIES {
            return Err(format!("at most {MAX_ENTRIES} networks and datastores"));
        }
        let check_target = |label: &str, t: &NetworkTarget| -> Result<(), String> {
            match t.mode.as_str() {
                "tap" | "macvtap" => {
                    if t.bridge.as_deref().map(str::trim).unwrap_or("").is_empty() {
                        return Err(format!("{label}: mode {} requires a bridge", t.mode));
                    }
                }
                "user" => {}
                other => {
                    return Err(format!(
                        "{label}: mode {other:?} must be tap, macvtap or user"
                    ));
                }
            }
            Ok(())
        };
        for (src, t) in &self.networks {
            if src.trim().is_empty() {
                return Err("source network names must not be empty".into());
            }
            check_target(&format!("network {src:?}"), t)?;
        }
        if let Some(t) = &self.default_network {
            check_target("default_network", t)?;
        }
        for (src, pool) in &self.datastores {
            if src.trim().is_empty() || pool.trim().is_empty() {
                return Err("datastore names and pools must not be empty".into());
            }
        }
        Ok(())
    }

    pub fn to_configmap(&self, namespace: &str) -> ConfigMap {
        let mut data = BTreeMap::new();
        data.insert("name".to_string(), self.name.clone());
        data.insert(
            "networks".into(),
            serde_json::to_string(&self.networks).unwrap_or_else(|_| "{}".into()),
        );
        data.insert(
            "datastores".into(),
            serde_json::to_string(&self.datastores).unwrap_or_else(|_| "{}".into()),
        );
        if let Some(d) = &self.description {
            data.insert("description".into(), d.clone());
        }
        if let Some(n) = &self.default_network {
            data.insert(
                "default_network".into(),
                serde_json::to_string(n).unwrap_or_default(),
            );
        }
        if let Some(p) = &self.default_pool {
            data.insert("default_pool".into(), p.clone());
        }
        ConfigMap {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(configmap_name(&self.name)),
                namespace: Some(namespace.to_string()),
                labels: Some(BTreeMap::from([(
                    MAPPING_TYPE_LABEL.to_string(),
                    MAPPING_TYPE.to_string(),
                )])),
                ..Default::default()
            },
            data: Some(data),
            ..Default::default()
        }
    }

    pub fn from_configmap(cm: &ConfigMap) -> Result<Self, String> {
        let data = cm.data.as_ref().ok_or("ConfigMap has no data")?;
        let name = data
            .get("name")
            .cloned()
            .or_else(|| {
                cm.metadata
                    .name
                    .as_deref()
                    .and_then(|n| n.strip_prefix("import-mapping-"))
                    .map(str::to_string)
            })
            .ok_or("ConfigMap has no mapping name")?;
        let networks = match data.get("networks") {
            Some(s) => serde_json::from_str(s).map_err(|e| format!("networks: {e}"))?,
            None => BTreeMap::new(),
        };
        let datastores = match data.get("datastores") {
            Some(s) => serde_json::from_str(s).map_err(|e| format!("datastores: {e}"))?,
            None => BTreeMap::new(),
        };
        let default_network = match data.get("default_network") {
            Some(s) if !s.is_empty() => {
                Some(serde_json::from_str(s).map_err(|e| format!("default_network: {e}"))?)
            }
            _ => None,
        };
        Ok(Self {
            name,
            description: data.get("description").cloned(),
            networks,
            datastores,
            default_network,
            default_pool: data.get("default_pool").cloned(),
        })
    }
}

/// The result of resolving a request through a profile.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolved {
    pub mapping: String,
    pub source_network: Option<String>,
    pub network: Option<NetworkTarget>,
    pub source_datastore: Option<String>,
    pub pool: Option<String>,
}

/// Resolve the source network/datastore through `m`. A name the profile does not list falls
/// back to its default; with no default the import is refused rather than silently landing
/// on Kairon's user-mode network.
pub fn resolve(
    m: &Mapping,
    source_network: Option<&str>,
    source_datastore: Option<&str>,
) -> Result<Resolved, String> {
    let mut out = Resolved {
        mapping: m.name.clone(),
        source_network: source_network.map(str::to_string),
        source_datastore: source_datastore.map(str::to_string),
        ..Default::default()
    };
    match source_network {
        Some(n) => {
            out.network = Some(
                m.networks
                    .get(n)
                    .or(m.default_network.as_ref())
                    .cloned()
                    .ok_or_else(|| {
                        format!("source network {n:?} is not in mapping {:?} and it has no default_network", m.name)
                    })?,
            );
        }
        None => out.network = m.default_network.clone(),
    }
    match source_datastore {
        Some(d) => {
            out.pool = Some(
                m.datastores
                    .get(d)
                    .or(m.default_pool.as_ref())
                    .cloned()
                    .ok_or_else(|| {
                        format!("source datastore {d:?} is not in mapping {:?} and it has no default_pool", m.name)
                    })?,
            );
        }
        None => out.pool = m.default_pool.clone(),
    }
    Ok(out)
}

/// Apply a resolution to an already built import Machine.
pub fn apply_mapping(machine: &mut Machine, r: &Resolved) {
    if let Some(t) = &r.network {
        let net = machine.spec.network.get_or_insert_with(Default::default);
        net.mode = Some(t.mode.clone());
        net.bridge = t.bridge.clone();
    }
    let ann = machine
        .metadata
        .annotations
        .get_or_insert_with(Default::default);
    ann.insert(MAPPING_ANNOTATION.into(), r.mapping.clone());
    if let Some(n) = &r.source_network {
        ann.insert(SOURCE_NETWORK_ANNOTATION.into(), n.clone());
    }
    if let Some(d) = &r.source_datastore {
        ann.insert(SOURCE_DATASTORE_ANNOTATION.into(), d.clone());
    }
    if let Some(p) = &r.pool {
        ann.insert(STORAGE_POOL_ANNOTATION.into(), p.clone());
    }
}

/// Validation issues for `POST /imports/validate` when the request names a mapping.
pub fn mapping_issues(
    requested: &str,
    found: Option<&Mapping>,
    source_network: Option<&str>,
    source_datastore: Option<&str>,
) -> Vec<super::import_validate::Issue> {
    let mk = |code: &'static str, message: String| super::import_validate::Issue {
        field: "mapping",
        code,
        message,
    };
    match found {
        None => vec![mk(
            "MAPPING_NOT_FOUND",
            format!("mapping {requested:?} does not exist"),
        )],
        Some(m) => match resolve(m, source_network, source_datastore) {
            Ok(_) => vec![],
            Err(e) => vec![mk("UNMAPPED_SOURCE", e)],
        },
    }
}

fn err(status: StatusCode, code: &'static str, message: impl Into<String>) -> ApiErr {
    ApiErr {
        status,
        code,
        message: message.into(),
    }
}

/// Storage for profiles, so handlers run without a cluster.
pub trait MappingStore: Send + Sync {
    fn get(&self, name: &str) -> impl Future<Output = Result<Option<Mapping>, String>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<Mapping>, String>> + Send;
    /// `create_only`: fail with `Ok(false)` if it already exists.
    fn put(
        &self,
        m: &Mapping,
        create_only: bool,
    ) -> impl Future<Output = Result<bool, String>> + Send;
    /// `Ok(false)` when there was nothing to delete.
    fn delete(&self, name: &str) -> impl Future<Output = Result<bool, String>> + Send;
}

pub struct KubeMappingStore {
    pub client: kube::Client,
    pub namespace: String,
}

impl KubeMappingStore {
    fn api(&self) -> Api<ConfigMap> {
        Api::namespaced(self.client.clone(), &self.namespace)
    }
}

impl MappingStore for KubeMappingStore {
    async fn get(&self, name: &str) -> Result<Option<Mapping>, String> {
        match self
            .api()
            .get_opt(&configmap_name(name))
            .await
            .map_err(|e| e.to_string())?
        {
            Some(cm) => Mapping::from_configmap(&cm).map(Some),
            None => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<Mapping>, String> {
        let list = self
            .api()
            .list(&ListParams::default().labels(&format!("{MAPPING_TYPE_LABEL}={MAPPING_TYPE}")))
            .await
            .map_err(|e| e.to_string())?;
        // A hand-edited ConfigMap that no longer parses is skipped, not fatal.
        Ok(list
            .items
            .iter()
            .filter_map(|cm| Mapping::from_configmap(cm).ok())
            .collect())
    }

    async fn put(&self, m: &Mapping, create_only: bool) -> Result<bool, String> {
        let cm = m.to_configmap(&self.namespace);
        let api = self.api();
        if create_only {
            return match api.create(&PostParams::default(), &cm).await {
                Ok(_) => Ok(true),
                Err(kube::Error::Api(e)) if e.code == 409 => Ok(false),
                Err(e) => Err(e.to_string()),
            };
        }
        api.replace(&configmap_name(&m.name), &PostParams::default(), &cm)
            .await
            .map(|_| true)
            .map_err(|e| e.to_string())
    }

    async fn delete(&self, name: &str) -> Result<bool, String> {
        match self
            .api()
            .delete(&configmap_name(name), &DeleteParams::default())
            .await
        {
            Ok(_) => Ok(true),
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(false),
            Err(e) => Err(e.to_string()),
        }
    }
}

pub async fn create_mapping<S: MappingStore>(store: &S, m: Mapping) -> Result<Mapping, ApiErr> {
    m.validate()
        .map_err(|e| err(StatusCode::BAD_REQUEST, "INVALID_MAPPING", e))?;
    match store.put(&m, true).await {
        Ok(true) => Ok(m),
        Ok(false) => Err(err(
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
            format!("mapping {:?} already exists; use PUT to replace it", m.name),
        )),
        Err(e) => Err(err(StatusCode::INTERNAL_SERVER_ERROR, "STORE_FAILED", e)),
    }
}

pub async fn replace_mapping<S: MappingStore>(
    store: &S,
    name: &str,
    mut m: Mapping,
) -> Result<Mapping, ApiErr> {
    m.name = name.to_string();
    m.validate()
        .map_err(|e| err(StatusCode::BAD_REQUEST, "INVALID_MAPPING", e))?;
    let exists = store
        .get(name)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "STORE_FAILED", e))?
        .is_some();
    if !exists {
        return Err(err(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("mapping {name:?} not found"),
        ));
    }
    store
        .put(&m, false)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "STORE_FAILED", e))?;
    Ok(m)
}

pub async fn get_mapping<S: MappingStore>(store: &S, name: &str) -> Result<Mapping, ApiErr> {
    store
        .get(name)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "STORE_FAILED", e))?
        .ok_or_else(|| {
            err(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                format!("mapping {name:?} not found"),
            )
        })
}

pub async fn delete_mapping<S: MappingStore>(store: &S, name: &str) -> Result<(), ApiErr> {
    match store.delete(name).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(err(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("mapping {name:?} not found"),
        )),
        Err(e) => Err(err(StatusCode::INTERNAL_SERVER_ERROR, "STORE_FAILED", e)),
    }
}

fn reply(r: Result<(StatusCode, Value), ApiErr>) -> (StatusCode, Json<Value>) {
    match r {
        Ok((s, data)) => (s, Json(json!({"success": true, "data": data}))),
        Err(e) => (
            e.status,
            Json(json!({"success": false, "error": {"code": e.code, "message": e.message}})),
        ),
    }
}

async fn store_for(state: &SharedState) -> Result<KubeMappingStore, ApiErr> {
    if !crate::api::vm_backend::is_kairon() {
        return Err(err(
            StatusCode::NOT_IMPLEMENTED,
            "KAIRON_REQUIRED",
            "VM imports land on Kairon Machines; unset VEYRON_VM_BACKEND or set it to kairon",
        ));
    }
    let s = state.read().await;
    Ok(KubeMappingStore {
        client: s.client().client(),
        namespace: s.namespace.clone(),
    })
}

async fn list_route(State(state): State<SharedState>) -> (StatusCode, Json<Value>) {
    reply(
        async {
            let store = store_for(&state).await?;
            let items = store
                .list()
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, "STORE_FAILED", e))?;
            Ok::<_, ApiErr>((StatusCode::OK, json!(items)))
        }
        .await,
    )
}

async fn create_route(
    State(state): State<SharedState>,
    Json(m): Json<Mapping>,
) -> (StatusCode, Json<Value>) {
    reply(
        async {
            let store = store_for(&state).await?;
            let m = create_mapping(&store, m).await?;
            Ok::<_, ApiErr>((StatusCode::CREATED, json!(m)))
        }
        .await,
    )
}

async fn get_route(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Value>) {
    reply(
        async {
            let store = store_for(&state).await?;
            Ok::<_, ApiErr>((StatusCode::OK, json!(get_mapping(&store, &name).await?)))
        }
        .await,
    )
}

async fn put_route(
    State(state): State<SharedState>,
    Path(name): Path<String>,
    Json(m): Json<Mapping>,
) -> (StatusCode, Json<Value>) {
    reply(
        async {
            let store = store_for(&state).await?;
            Ok::<_, ApiErr>((
                StatusCode::OK,
                json!(replace_mapping(&store, &name, m).await?),
            ))
        }
        .await,
    )
}

async fn delete_route(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Value>) {
    reply(
        async {
            let store = store_for(&state).await?;
            delete_mapping(&store, &name).await?;
            Ok::<_, ApiErr>((StatusCode::OK, json!({"name": name, "deleted": true})))
        }
        .await,
    )
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/import-mappings", get(list_route).post(create_route))
        .route(
            "/import-mappings/:name",
            get(get_route).put(put_route).delete(delete_route),
        )
        .with_state(state)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::kairon::MachineSpec;
    use std::sync::Mutex;

    pub fn dc1() -> Mapping {
        Mapping {
            name: "dc1".into(),
            description: Some("DC1 vSphere".into()),
            networks: BTreeMap::from([
                (
                    "VM Network".to_string(),
                    NetworkTarget {
                        mode: "macvtap".into(),
                        bridge: Some("eth1".into()),
                    },
                ),
                (
                    "DMZ".to_string(),
                    NetworkTarget {
                        mode: "tap".into(),
                        bridge: Some("br-dmz".into()),
                    },
                ),
            ]),
            datastores: BTreeMap::from([("ds-ssd".to_string(), "fast".to_string())]),
            default_network: None,
            default_pool: Some("bulk".into()),
        }
    }

    #[derive(Default)]
    pub struct FakeMappings(pub Mutex<BTreeMap<String, Mapping>>);

    impl MappingStore for FakeMappings {
        async fn get(&self, name: &str) -> Result<Option<Mapping>, String> {
            Ok(self.0.lock().unwrap().get(name).cloned())
        }
        async fn list(&self) -> Result<Vec<Mapping>, String> {
            Ok(self.0.lock().unwrap().values().cloned().collect())
        }
        async fn put(&self, m: &Mapping, create_only: bool) -> Result<bool, String> {
            let mut g = self.0.lock().unwrap();
            if create_only && g.contains_key(&m.name) {
                return Ok(false);
            }
            g.insert(m.name.clone(), m.clone());
            Ok(true)
        }
        async fn delete(&self, name: &str) -> Result<bool, String> {
            Ok(self.0.lock().unwrap().remove(name).is_some())
        }
    }

    #[test]
    fn validates_profiles() {
        assert!(dc1().validate().is_ok());
        let mut m = dc1();
        m.name = "DC_1".into();
        assert!(m.validate().is_err());
        let mut m = dc1();
        m.networks.insert(
            "x".into(),
            NetworkTarget {
                mode: "tap".into(),
                bridge: None,
            },
        );
        assert!(m.validate().unwrap_err().contains("requires a bridge"));
        let mut m = dc1();
        m.networks.insert(
            "x".into(),
            NetworkTarget {
                mode: "vlan".into(),
                bridge: Some("b".into()),
            },
        );
        assert!(m.validate().is_err());
    }

    #[test]
    fn configmap_round_trip() {
        let m = dc1();
        let cm = m.to_configmap("veyron-system");
        assert_eq!(cm.metadata.name.as_deref(), Some("import-mapping-dc1"));
        assert_eq!(
            cm.metadata.labels.as_ref().unwrap()[MAPPING_TYPE_LABEL],
            MAPPING_TYPE
        );
        assert_eq!(Mapping::from_configmap(&cm).unwrap(), m);
        let mut bad = cm;
        bad.data
            .as_mut()
            .unwrap()
            .insert("networks".into(), "{".into());
        assert!(Mapping::from_configmap(&bad).is_err());
    }

    #[test]
    fn resolves_with_defaults_and_refuses_unmapped() {
        let m = dc1();
        let r = resolve(&m, Some("DMZ"), Some("ds-ssd")).unwrap();
        assert_eq!(r.network.unwrap().bridge.as_deref(), Some("br-dmz"));
        assert_eq!(r.pool.as_deref(), Some("fast"));
        // unknown datastore falls back to default_pool
        assert_eq!(
            resolve(&m, None, Some("ds-other")).unwrap().pool.as_deref(),
            Some("bulk")
        );
        // unknown network, no default -> error
        assert!(
            resolve(&m, Some("Lab"), None)
                .unwrap_err()
                .contains("default_network")
        );
        // nothing given: nothing applied
        let r = resolve(&m, None, None).unwrap();
        assert!(r.network.is_none());
    }

    #[test]
    fn applies_to_machine() {
        let mut machine = Machine::new("a", MachineSpec::default());
        let r = resolve(&dc1(), Some("VM Network"), Some("ds-ssd")).unwrap();
        apply_mapping(&mut machine, &r);
        let net = machine.spec.network.as_ref().unwrap();
        assert_eq!(net.mode.as_deref(), Some("macvtap"));
        assert_eq!(net.bridge.as_deref(), Some("eth1"));
        let ann = machine.metadata.annotations.unwrap();
        assert_eq!(ann[MAPPING_ANNOTATION], "dc1");
        assert_eq!(ann[SOURCE_NETWORK_ANNOTATION], "VM Network");
        assert_eq!(ann[STORAGE_POOL_ANNOTATION], "fast");
    }

    #[test]
    fn mapping_issues_codes() {
        assert_eq!(
            mapping_issues("x", None, None, None)[0].code,
            "MAPPING_NOT_FOUND"
        );
        let m = dc1();
        assert_eq!(
            mapping_issues("dc1", Some(&m), Some("Lab"), None)[0].code,
            "UNMAPPED_SOURCE"
        );
        assert!(mapping_issues("dc1", Some(&m), Some("DMZ"), None).is_empty());
    }

    #[tokio::test]
    async fn crud_flow() {
        let store = FakeMappings::default();
        create_mapping(&store, dc1()).await.unwrap();
        assert_eq!(
            create_mapping(&store, dc1()).await.unwrap_err().code,
            "ALREADY_EXISTS"
        );
        let mut bad = dc1();
        bad.name = "Bad Name".into();
        assert_eq!(
            create_mapping(&store, bad).await.unwrap_err().code,
            "INVALID_MAPPING"
        );
        assert_eq!(get_mapping(&store, "dc1").await.unwrap(), dc1());
        let mut changed = dc1();
        changed.default_pool = Some("slow".into());
        // the path name wins over the body name
        changed.name = "ignored".into();
        let out = replace_mapping(&store, "dc1", changed).await.unwrap();
        assert_eq!(out.name, "dc1");
        assert_eq!(
            get_mapping(&store, "dc1")
                .await
                .unwrap()
                .default_pool
                .as_deref(),
            Some("slow")
        );
        assert_eq!(
            replace_mapping(&store, "nope", dc1())
                .await
                .unwrap_err()
                .code,
            "NOT_FOUND"
        );
        assert_eq!(store.list().await.unwrap().len(), 1);
        delete_mapping(&store, "dc1").await.unwrap();
        assert_eq!(
            delete_mapping(&store, "dc1").await.unwrap_err().code,
            "NOT_FOUND"
        );
    }
}
