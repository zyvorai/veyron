// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
#![cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
};
use k8s_openapi::api::core::v1::ConfigMap;
use serde_json::{Value, json};
use std::future::IntoFuture;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::{Barrier, Mutex};
use veyron::{VMConfigBuilder, api::tenant_admission, kube::KubeClient};
#[derive(Clone)]
struct Mock {
    cms: Arc<Mutex<BTreeMap<String, ConfigMap>>>,
    first_reads: Arc<AtomicUsize>,
    barrier: Arc<Barrier>,
}
fn status(code: StatusCode, reason: &str) -> Response {
    (code,Json(json!({"apiVersion":"v1","kind":"Status","status":"Failure","reason":reason,"message":reason,"code":code.as_u16()}))).into_response()
}
async fn serve(State(mock): State<Mock>, req: Request) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    if method == Method::GET && path == "/api/v1/namespaces/tenant-test" {
        return Json(json!({"apiVersion":"v1","kind":"Namespace","metadata":{"name":"tenant-test","labels":{"veyron.io/tenant":"test","veyron.io/vm-backend":veyron::api::vm_backend::selected().as_str()}}})).into_response();
    }
    if method == Method::GET && path.ends_with("/resourcequotas/tenant-quota") {
        return Json(json!({"apiVersion":"v1","kind":"ResourceQuota","metadata":{"name":"tenant-quota"},"spec":{"hard":{"limits.cpu":"10","limits.memory":"64Gi","requests.nvidia.com/gpu":"0","count/virtualmachines.kubevirt.io":"1","count/machines.kairon.zyvor.dev":"1"}}})).into_response();
    }
    if method == Method::GET && path.ends_with("/virtualmachines") {
        return Json(json!({"apiVersion":"kubevirt.io/v1","kind":"VirtualMachineList","metadata":{},"items":[]})).into_response();
    }
    if method == Method::GET && path.ends_with("/machines") {
        return Json(json!({"apiVersion":"kairon.zyvor.dev/v1alpha1","kind":"MachineList","metadata":{},"items":[]})).into_response();
    }
    if path.contains("/configmaps") {
        if method == Method::GET {
            let name = path.rsplit('/').next().unwrap();
            let found = mock.cms.lock().await.get(name).cloned();
            // Force the initial callers to observe the same empty ledger before either writes.
            if mock.first_reads.fetch_add(1, Ordering::SeqCst) < 2 {
                mock.barrier.wait().await;
            }
            return match found {
                Some(cm) => Json(cm).into_response(),
                None => status(StatusCode::NOT_FOUND, "NotFound"),
            };
        }
        if matches!(method, Method::POST | Method::PUT) {
            let bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024)
                .await
                .unwrap();
            let mut cm: ConfigMap = serde_json::from_slice(&bytes).unwrap();
            let name = cm.metadata.name.clone().unwrap();
            let mut store = mock.cms.lock().await;
            if method == Method::POST && store.contains_key(&name) {
                return status(StatusCode::CONFLICT, "AlreadyExists");
            }
            if method == Method::PUT
                && store.get(&name).map(|v| &v.metadata.resource_version)
                    != Some(&cm.metadata.resource_version)
            {
                return status(StatusCode::CONFLICT, "Conflict");
            }
            let next = store
                .get(&name)
                .and_then(|c| c.metadata.resource_version.as_ref())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
                + 1;
            cm.metadata.resource_version = Some(next.to_string());
            store.insert(name, cm.clone());
            return Json(cm).into_response();
        }
    }
    status(StatusCode::NOT_FOUND, "unexpected mock route")
}
#[tokio::test]
async fn concurrent_creates_cannot_exceed_one_vm_quota() {
    let mock = Mock {
        cms: Arc::new(Mutex::new(BTreeMap::new())),
        first_reads: Arc::new(AtomicUsize::new(0)),
        barrier: Arc::new(Barrier::new(2)),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(
        axum::serve(
            listener,
            Router::new().fallback(serve).with_state(mock.clone()),
        )
        .into_future(),
    );
    let cfg = kube::Config::new(url.parse().unwrap());
    let client = KubeClient::from_client(kube::Client::try_from(cfg).unwrap());
    let a = VMConfigBuilder::new("a")
        .namespace("tenant-test")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .build();
    let b = VMConfigBuilder::new("b")
        .namespace("tenant-test")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .build();
    let results = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            tenant_admission::reserve(&client, &a),
            tenant_admission::reserve(&client, &b)
        )
    })
    .await
    .unwrap();
    assert_ne!(
        results.0.is_ok(),
        results.1.is_ok(),
        "exactly one create must reserve quota"
    );
    let store = mock.cms.lock().await;
    let raw = store["veyron-vm-admission"].data.as_ref().unwrap()["ledger.json"].clone();
    let ledger: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(ledger["pending"].as_object().unwrap().len(), 1);
    server.abort();
}
