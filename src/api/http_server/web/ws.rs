// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
//
// WebSocket endpoints: metrics stream, VNC and serial console proxies.

use super::*;

pub(super) async fn metrics_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| metrics_stream(socket, state))
}

pub(super) async fn metrics_stream(mut socket: WebSocket, state: SharedState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

    loop {
        interval.tick().await;

        // Clone client outside the lock to avoid holding it during K8s calls
        let client = {
            let s = state.read().await;
            s.client().clone()
        };

        let metrics = {
            let vms = client.list_all_vms().await.unwrap_or_default();
            let nodes = client.list_nodes().await.unwrap_or_default();

            let total_vms = vms.len() as u32;
            let running_vms = vms
                .iter()
                .filter(|vm| {
                    vm.status
                        .as_ref()
                        .and_then(|s| s.printable_status.as_deref())
                        .map(|s| s == "Running")
                        .unwrap_or(false)
                })
                .count() as u32;

            let mut total_cpu: u32 = 0;
            let mut total_mem: u64 = 0;
            for node in &nodes {
                if let Some(cap) = node.status.as_ref().and_then(|s| s.capacity.as_ref()) {
                    if let Some(cpu) = cap.get("cpu") {
                        total_cpu += cpu.0.parse::<u32>().unwrap_or(0);
                    }
                    if let Some(mem) = cap.get("memory") {
                        total_mem += crate::utils::parse_memory_bytes(&mem.0);
                    }
                }
            }

            serde_json::json!({
                "type": "cluster_metrics",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "total_vms": total_vms,
                "running_vms": running_vms,
                "total_cpu_cores": total_cpu,
                "total_memory_bytes": total_mem,
                "node_count": nodes.len(),
            })
        };

        if socket
            .send(Message::Text(metrics.to_string()))
            .await
            .is_err()
        {
            break; // Client disconnected
        }
    }
}

// ── Security headers middleware ─────────────────────────────

pub(super) async fn vnc_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    #[cfg(feature = "kairon")]
    if crate::api::vm_backend::is_kairon() {
        let client = { state.read().await.kube_client.clone() };
        return match kairon_relay_upstream(&client, &ns, &name, "console").await {
            Ok(upstream) => ws
                .protocols(["binary"])
                .on_upgrade(move |socket| {
                    kubevirt_subresource_ws_proxy(socket, ns, name, upstream, "VNC")
                })
                .into_response(),
            Err(resp) => resp.into_response(),
        };
    }
    let running = {
        let s = state.read().await;
        s.kube_client.is_running(&ns, &name).await
    };

    // Check VM is running
    match running {
        Ok(true) => {}
        Ok(false) => {
            return (StatusCode::BAD_REQUEST, "VM is not running").into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to check VM status",
            )
                .into_response();
        }
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    let vmi_name = match client.resolve_vmi_name_for_console(&ns, &name).await {
        Ok(n) => n,
        Err(e) => {
            log::warn!("VNC: could not resolve VMI for {}/{}: {}", ns, name, e);
            return (StatusCode::NOT_FOUND, format!("No VMI for VM {name}: {e}")).into_response();
        }
    };

    log::info!(
        "VNC WebSocket upgrade requested for {}/{} (VMI {})",
        ns,
        name,
        vmi_name
    );
    ws.protocols(["binary"])
        .on_upgrade(move |socket| {
            kubevirt_subresource_ws_proxy(socket, ns, vmi_name, Upstream::KubeVirt("vnc"), "VNC")
        })
        .into_response()
}

pub(super) async fn serial_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    #[cfg(feature = "kairon")]
    if crate::api::vm_backend::is_kairon() {
        let client = { state.read().await.kube_client.clone() };
        return match kairon_relay_upstream(&client, &ns, &name, "text-console").await {
            Ok(upstream) => ws
                .protocols(["binary"])
                .on_upgrade(move |socket| {
                    kubevirt_subresource_ws_proxy(socket, ns, name, upstream, "serial console")
                })
                .into_response(),
            Err(resp) => resp.into_response(),
        };
    }
    let running = {
        let s = state.read().await;
        s.kube_client.is_running(&ns, &name).await
    };

    match running {
        Ok(true) => {}
        Ok(false) => {
            return (StatusCode::BAD_REQUEST, "VM is not running").into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to check VM status",
            )
                .into_response();
        }
    }

    let client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    let vmi_name = match client.resolve_vmi_name_for_console(&ns, &name).await {
        Ok(n) => n,
        Err(e) => {
            log::warn!(
                "Serial console: could not resolve VMI for {}/{}: {}",
                ns,
                name,
                e
            );
            return (StatusCode::NOT_FOUND, format!("No VMI for VM {name}: {e}")).into_response();
        }
    };

    log::info!(
        "Serial console WebSocket upgrade requested for {}/{} (VMI {})",
        ns,
        name,
        vmi_name
    );
    ws.protocols(["binary"])
        .on_upgrade(move |socket| {
            kubevirt_subresource_ws_proxy(
                socket,
                ns,
                vmi_name,
                Upstream::KubeVirt("console"),
                "serial console",
            )
        })
        .into_response()
}

/// Where a console WebSocket is proxied to.
pub(super) enum Upstream {
    /// KubeVirt VMI subresource (`vnc`, `console`) through the Kubernetes API.
    KubeVirt(&'static str),
    /// kairon-node relay: full `ws(s)://` URL and its bearer token.
    #[cfg_attr(not(feature = "kairon"), allow(dead_code))]
    Relay { url: String, token: String },
}

/// Resolve a running Machine's console on its kairon-node relay
/// (`console` = VNC, `text-console` = serial).
#[cfg(feature = "kairon")]
pub(super) async fn kairon_relay_upstream(
    client: &crate::kube::KubeClient,
    ns: &str,
    name: &str,
    kind: &str,
) -> Result<Upstream, (StatusCode, String)> {
    let m = kube::Api::<crate::kairon::Machine>::namespaced(client.client(), ns)
        .get(name)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("VM {name}: {e}")))?;
    if !m.is_running() {
        return Err((StatusCode::BAD_REQUEST, "VM is not running".into()));
    }
    let target = crate::kairon::relay::RelayTarget::for_machine(&client.client(), &m)
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("{e:#}")))?;
    Ok(Upstream::Relay {
        url: target.websocket_url(kind),
        token: target.token.clone(),
    })
}

/// Proxy a browser WebSocket to a VM console: a KubeVirt VMI subresource or a
/// kairon-node relay.
pub(super) async fn kubevirt_subresource_ws_proxy(
    mut client_ws: WebSocket,
    ns: String,
    vmi_name: String,
    upstream: Upstream,
    label: &'static str,
) {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::protocol::CloseFrame as TsCloseFrame;
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode as TsCloseCode;

    fn truncate_close_reason(mut s: String) -> String {
        const MAX: usize = 123;
        if s.len() <= MAX {
            return s;
        }
        let mut end = MAX;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
        s
    }

    fn ax_close_to_ts(
        frame: Option<axum::extract::ws::CloseFrame>,
    ) -> Option<TsCloseFrame<'static>> {
        frame.map(|f| TsCloseFrame {
            code: TsCloseCode::from(f.code),
            reason: Cow::Owned(truncate_close_reason(f.reason.into_owned())),
        })
    }

    fn ts_close_to_ax(
        frame: Option<TsCloseFrame>,
    ) -> Option<axum::extract::ws::CloseFrame<'static>> {
        frame.map(|f| axum::extract::ws::CloseFrame {
            code: u16::from(f.code),
            reason: Cow::Owned(truncate_close_reason(f.reason.into_owned())),
        })
    }

    let (ws_url, bearer, verify_tls) = match upstream {
        Upstream::Relay { url, token } => {
            log::info!(
                "{} proxy connecting to kairon-node relay for {}/{}",
                label,
                ns,
                vmi_name
            );
            (url, Some(token), !crate::kairon::relay::tls_insecure())
        }
        Upstream::KubeVirt(subpath) => {
            let config = match kube::Config::infer().await {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to infer kube config: {}", e);
                    let _ = client_ws
                        .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                            code: 1011,
                            reason: "Failed to get cluster config".into(),
                        })))
                        .await;
                    return;
                }
            };
            let api_url = config
                .cluster_url
                .to_string()
                .trim_end_matches('/')
                .to_string();
            let path = format!(
                "/apis/subresources.kubevirt.io/v1/namespaces/{}/virtualmachineinstances/{}/{}",
                ns, vmi_name, subpath
            );
            log::info!("{} proxy connecting to K8s API: {}", label, path);
            let url = api_url
                .replace("https://", "wss://")
                .replace("http://", "ws://")
                + &path;
            let token =
                std::fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/token")
                    .ok()
                    .map(|t| t.trim().to_string());
            (url, token, false)
        }
    };

    let tls_connector = if verify_tls {
        None
    } else {
        let tls_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(AcceptAllVerifier))
            .with_no_client_auth();
        let connector = tokio_tungstenite::Connector::Rustls(std::sync::Arc::new(tls_config));
        Some(connector)
    };

    let mut request =
        tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(
            ws_url.as_str(),
        )
        .unwrap_or_else(|_| {
            tokio_tungstenite::tungstenite::handshake::client::Request::get(&ws_url)
                .body(())
                .unwrap_or_else(|_| {
                    // ws_url is malformed; build a safe no-op request so we fail at connect
                    tokio_tungstenite::tungstenite::handshake::client::Request::get(
                        "ws://localhost/",
                    )
                    .body(())
                    .expect("localhost ws request is always valid")
                })
        });

    if let Some(token) = bearer {
        if let Ok(header_val) = format!("Bearer {token}").parse() {
            request.headers_mut().insert("Authorization", header_val);
        }
    }

    let k8s_ws =
        match tokio_tungstenite::connect_async_tls_with_config(request, None, false, tls_connector)
            .await
        {
            Ok((ws, _)) => ws,
            Err(e) => {
                log::error!("Failed to connect to KubeVirt {} API: {}", label, e);
                let reason = format!("Failed to connect to {}", label);
                let _ = client_ws
                    .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code: 1011,
                        reason: reason.into(),
                    })))
                    .await;
                return;
            }
        };

    log::info!(
        "{} proxy connected to K8s API for namespace {} VMI {}",
        label,
        ns,
        vmi_name
    );

    let (mut k8s_sink, mut k8s_stream) = k8s_ws.split();

    // Apiserver / ingress often drop idle WebSockets; answer Ping and send periodic keepalives.
    let mut keepalive = tokio::time::interval(std::time::Duration::from_secs(20));
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut sent_client_close = false;

    loop {
        tokio::select! {
            msg = client_ws.recv() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        if k8s_sink.send(tokio_tungstenite::tungstenite::Message::Binary(data.to_vec())).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        if k8s_sink.send(tokio_tungstenite::tungstenite::Message::Text(text.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(v))) => {
                        if client_ws.send(Message::Pong(v)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(frame))) => {
                        let ts = ax_close_to_ts(frame);
                        let _ = k8s_sink.send(tokio_tungstenite::tungstenite::Message::Close(ts)).await;
                        break;
                    }
                    Some(Err(_)) | None => break,
                }
            }
            msg = k8s_stream.next() => {
                match msg {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(data))) => {
                        if client_ws.send(Message::Binary(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                        if client_ws.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(v))) => {
                        if k8s_sink.send(tokio_tungstenite::tungstenite::Message::Pong(v)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Pong(_))) => {}
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Frame(_))) => {}
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(frame))) => {
                        match &frame {
                            Some(cf) => log::info!(
                                "{} upstream close for {}/{}: code={} reason={}",
                                label,
                                ns,
                                vmi_name,
                                u16::from(cf.code),
                                cf.reason
                            ),
                            None => log::info!(
                                "{} upstream close for {}/{} (no frame)",
                                label,
                                ns,
                                vmi_name
                            ),
                        }
                        let ax = ts_close_to_ax(frame);
                        if client_ws.send(Message::Close(ax)).await.is_ok() {
                            sent_client_close = true;
                        }
                        break;
                    }
                    Some(Err(_)) | None => break,
                }
            }
            _ = keepalive.tick() => {
                if client_ws.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
                if k8s_sink
                    .send(tokio_tungstenite::tungstenite::Message::Ping(Vec::new()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    }

    if !sent_client_close {
        let _ = client_ws.send(Message::Close(None)).await;
    }
    log::info!("{} proxy session ended for {}/{}", label, ns, vmi_name);
}

// ── VM Security Posture ──────────────────────────────────────
