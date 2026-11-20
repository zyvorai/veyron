// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Plan webhooks: tell people when a plan needs them, so a second approver learns about it
//! without opening the console.
//!
//! `VEYRON_AI_PLAN_WEBHOOK_URL` is any HTTP(S) endpoint that accepts JSON. The body carries
//! both `text` (Slack, Mattermost, Teams-style) and `content` (Discord) with a one-line
//! summary, plus the structured `plan`. `VEYRON_AI_PLAN_WEBHOOK_EVENTS` limits which events
//! are sent (comma list of `created`, `approved`, `executed`, `failed`, `cancelled`; default all).
//! Delivery is best effort in the background: it never delays or fails a plan operation, and
//! the URL (which may embed a token) is never logged.

use serde_json::{Value, json};

use super::plan::Plan;

pub const ALL_EVENTS: [&str; 5] = ["created", "approved", "executed", "failed", "cancelled"];

/// `raw` trimmed, if it is a plain http(s) URL with a host.
pub fn valid_webhook_url(raw: &str) -> Option<String> {
    let u = raw.trim();
    let lower = u.to_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))?;
    (!rest.is_empty() && !rest.starts_with('/')).then(|| u.to_string())
}

/// The configured endpoint, only if it is a plain http(s) URL.
pub fn webhook_url() -> Option<String> {
    valid_webhook_url(&std::env::var("VEYRON_AI_PLAN_WEBHOOK_URL").ok()?)
}

/// Whether `event` passes the `VEYRON_AI_PLAN_WEBHOOK_EVENTS` filter (`filter` empty = all).
pub fn event_enabled(event: &str, filter: &str) -> bool {
    let wanted: Vec<String> = filter
        .split(',')
        .map(|w| w.trim().to_lowercase())
        .filter(|w| !w.is_empty())
        .collect();
    wanted.is_empty() || wanted.iter().any(|w| w == event)
}

fn first_step(p: &Plan) -> String {
    let first = p
        .steps
        .first()
        .map(|s| s.preview.clone())
        .unwrap_or_else(|| "no steps".into());
    match p.steps.len() {
        0 | 1 => first,
        n => format!("{first} (+{} more)", n - 1),
    }
}

/// One line a person can read in a chat channel.
pub fn message(event: &str, p: &Plan, actor: &str) -> String {
    let what = first_step(p);
    match event {
        "created" if p.needs_approval => format!(
            "{} drafted a plan that needs a second approver: {what} [{}]",
            p.created_by, p.id
        ),
        "created" => format!("{} drafted a plan: {what} [{}]", p.created_by, p.id),
        "approved" => format!(
            "{actor} approved {}'s plan: {what} [{}]",
            p.created_by, p.id
        ),
        "executed" => format!("{actor} ran plan: {what} [{}]", p.id),
        "failed" => {
            let ok = p
                .steps
                .iter()
                .filter(|s| s.result.as_ref().is_some_and(|r| r.ok))
                .count();
            format!(
                "plan failed ({ok}/{} steps succeeded): {what} [{}]",
                p.steps.len(),
                p.id
            )
        }
        "cancelled" => format!(
            "{actor} cancelled {}'s plan: {what} [{}]",
            p.created_by, p.id
        ),
        other => format!("plan {other}: {what} [{}]", p.id),
    }
}

pub fn payload(event: &str, p: &Plan, actor: &str) -> Value {
    let text = message(event, p, actor);
    json!({
        "event": format!("plan.{event}"),
        "text": text,
        "content": text,
        "actor": actor,
        "source": "veyron-copilot",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "plan": {
            "id": p.id,
            "status": p.status,
            "created_by": p.created_by,
            "min_role": p.min_role,
            "steps": p.steps.len(),
            "preview": p.steps.iter().take(5).map(|s| s.preview.clone()).collect::<Vec<_>>(),
            "needs_approval": p.needs_approval,
            "approved_by": p.approved_by,
            "confirm_count": p.confirm_count,
            "expires_at": p.expires_at,
            "prompt": p.prompt,
        },
    })
}

/// HMAC-SHA256 (RFC 2104) built on `sha2`, so signing needs no extra dependency.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(k.map(|b| b ^ 0x36));
    inner.update(msg);
    let mut outer = Sha256::new();
    outer.update(k.map(|b| b ^ 0x5c));
    outer.update(inner.finalize());
    outer.finalize().into()
}

/// `sha256=<hex>` over `"<timestamp>.<body>"`, the value of `X-Veyron-Signature`. Signing the
/// timestamp with the body lets a receiver reject replays of an old delivery.
pub fn signature(secret: &str, timestamp: &str, body: &[u8]) -> String {
    let mut msg = Vec::with_capacity(timestamp.len() + 1 + body.len());
    msg.extend_from_slice(timestamp.as_bytes());
    msg.push(b'.');
    msg.extend_from_slice(body);
    format!(
        "sha256={}",
        hex::encode(hmac_sha256(secret.as_bytes(), &msg))
    )
}

/// The secret configured for signing, if any (`VEYRON_AI_PLAN_WEBHOOK_SECRET`).
pub fn webhook_secret() -> Option<String> {
    std::env::var("VEYRON_AI_PLAN_WEBHOOK_SECRET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// POST `body` to `url`; one retry after a short pause on a network error or 5xx.
#[cfg(feature = "web")]
pub(crate) async fn deliver(url: &str, body: &Value) -> bool {
    deliver_signed(url, body, webhook_secret().as_deref()).await
}

/// [`deliver`] with an explicit signing secret (`None` sends no signature headers).
#[cfg(feature = "web")]
pub(crate) async fn deliver_signed(url: &str, body: &Value, secret: Option<&str>) -> bool {
    let client = crate::soc::http_client();
    // Serialize once so the signature covers exactly the bytes that are sent.
    let bytes = serde_json::to_vec(body).unwrap_or_default();
    for attempt in 0..2 {
        let mut req = client
            .post(url)
            .header("content-type", "application/json")
            .body(bytes.clone());
        if let Some(secret) = secret {
            let ts = chrono::Utc::now().timestamp().to_string();
            req = req
                .header("X-Veyron-Signature", signature(secret, &ts, &bytes))
                .header("X-Veyron-Timestamp", ts);
        }
        match req.send().await {
            Ok(r) if r.status().is_success() => return true,
            Ok(r) if r.status().is_client_error() => {
                log::warn!("plan webhook rejected with HTTP {}", r.status());
                return false;
            }
            Ok(r) => log::warn!("plan webhook HTTP {}", r.status()),
            Err(e) => log::warn!("plan webhook failed: {}", e.without_url()),
        }
        if attempt == 0 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    false
}

/// Fire-and-forget: send `event` for `plan` if a webhook is configured and the event is enabled.
#[cfg(feature = "web")]
pub fn emit(event: &'static str, plan: &Plan, actor: &str) {
    let Some(url) = webhook_url() else { return };
    let filter = std::env::var("VEYRON_AI_PLAN_WEBHOOK_EVENTS").unwrap_or_default();
    if !event_enabled(event, &filter) {
        return;
    }
    let body = payload(event, plan, actor);
    tokio::spawn(async move {
        deliver(&url, &body).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::plan::{StepResult, build_steps, new_plan};

    fn plan(names: &[&str], action: &str) -> Plan {
        let n: Vec<String> = names.iter().map(|s| s.to_string()).collect();
        new_plan(
            "do it",
            "alice",
            build_steps("vm", action, "prod", &n).unwrap(),
            600,
        )
    }

    #[test]
    fn the_url_must_be_plain_http_or_https() {
        assert_eq!(
            valid_webhook_url("https://hooks.example/abc").as_deref(),
            Some("https://hooks.example/abc")
        );
        assert_eq!(
            valid_webhook_url("http://10.0.0.5:8080/x").as_deref(),
            Some("http://10.0.0.5:8080/x")
        );
        assert_eq!(
            valid_webhook_url("  https://hooks.example/abc \n").as_deref(),
            Some("https://hooks.example/abc")
        );
        assert_eq!(
            valid_webhook_url("HTTPS://Hooks.Example/A").as_deref(),
            Some("HTTPS://Hooks.Example/A")
        );
        for bad in [
            "ftp://hooks.example",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://",
            "http:///x",
            "hooks.example",
            "",
        ] {
            assert!(valid_webhook_url(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn the_event_filter_defaults_to_everything() {
        assert!(event_enabled("created", ""));
        assert!(event_enabled("failed", "  "));
        assert!(event_enabled("created", "Created, executed"));
        assert!(!event_enabled("approved", "created,executed"));
        for e in ALL_EVENTS {
            assert!(event_enabled(e, ""));
        }
    }

    #[test]
    fn messages_read_like_a_sentence_for_every_event() {
        let mut p = plan(&["db-1", "db-2", "db-3"], "delete");
        p.needs_approval = true;
        let created = message("created", &p, "alice");
        assert!(
            created.contains("alice drafted a plan that needs a second approver"),
            "{created}"
        );
        assert!(
            created.contains("delete VM prod/db-1 (+2 more)"),
            "{created}"
        );
        assert!(created.contains(&p.id), "{created}");
        p.needs_approval = false;
        assert!(message("created", &p, "alice").starts_with("alice drafted a plan: "));
        assert!(message("approved", &p, "bob").starts_with("bob approved alice's plan: "));
        assert!(message("executed", &p, "alice").starts_with("alice ran plan: "));
        assert!(message("cancelled", &p, "alice").contains("cancelled alice's plan"));
        p.steps[0].result = Some(StepResult {
            ok: true,
            error: None,
        });
        assert!(message("failed", &p, "alice").contains("(1/3 steps succeeded)"));
        let one = plan(&["web-1"], "restart");
        assert!(
            message("executed", &one, "alice").contains("restart VM prod/web-1 ["),
            "single step has no '+ more'"
        );
    }

    #[test]
    fn the_payload_carries_both_chat_keys_and_the_structured_plan() {
        let mut p = plan(&["db-1"], "delete");
        p.needs_approval = true;
        p.approved_by = Some("bob".into());
        let v = payload("approved", &p, "bob");
        assert_eq!(v["event"], "plan.approved");
        assert_eq!(v["source"], "veyron-copilot");
        assert_eq!(
            v["text"], v["content"],
            "Slack-style text and Discord-style content match"
        );
        assert_eq!(v["actor"], "bob");
        assert_eq!(v["plan"]["id"], p.id);
        assert_eq!(v["plan"]["needs_approval"], true);
        assert_eq!(v["plan"]["approved_by"], "bob");
        assert_eq!(v["plan"]["steps"], 1);
        assert_eq!(v["plan"]["preview"][0], "delete VM prod/db-1");
        assert!(v["timestamp"].as_str().is_some_and(|t| t.contains('T')));
    }

    #[cfg(feature = "web")]
    #[tokio::test]
    async fn delivery_posts_json_and_retries_a_server_error_once() {
        use axum::{Json, Router, http::StatusCode, routing::post};
        use std::sync::{Arc, Mutex};

        let seen: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let app = Router::new().route(
            "/hook",
            post(move |Json(body): Json<Value>| {
                let sink = sink.clone();
                async move {
                    let mut g = sink.lock().unwrap();
                    g.push(body);
                    // first attempt fails with 503, the retry succeeds
                    if g.len() == 1 {
                        StatusCode::SERVICE_UNAVAILABLE
                    } else {
                        StatusCode::OK
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let p = plan(&["web-1"], "restart");
        let body = payload("created", &p, "alice");
        assert!(deliver(&format!("http://127.0.0.1:{port}/hook"), &body).await);
        let got = seen.lock().unwrap().clone();
        assert_eq!(got.len(), 2, "one retry after the 503");
        assert_eq!(got[0]["plan"]["id"], p.id);
        assert_eq!(got[1]["event"], "plan.created");
    }

    #[cfg(feature = "web")]
    #[tokio::test]
    async fn a_rejected_webhook_is_not_retried_and_never_panics() {
        use axum::{Router, http::StatusCode, routing::post};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        let app = Router::new().route(
            "/hook",
            post(move || {
                let h = h.clone();
                async move {
                    h.fetch_add(1, Ordering::SeqCst);
                    StatusCode::FORBIDDEN
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let body = payload("created", &plan(&["web-1"], "restart"), "alice");
        assert!(!deliver(&format!("http://127.0.0.1:{port}/hook"), &body).await);
        assert_eq!(hits.load(Ordering::SeqCst), 1, "a 4xx is final");
        // an unreachable endpoint also just returns false
        assert!(!deliver("http://127.0.0.1:1/hook", &body).await);
    }

    #[test]
    fn hmac_matches_the_rfc_4231_vectors() {
        // Test case 1
        assert_eq!(
            hex::encode(hmac_sha256(&[0x0b; 20], b"Hi There")),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        // Test case 2
        assert_eq!(
            hex::encode(hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        // Test case 6: a key longer than the block size is hashed first
        assert_eq!(
            hex::encode(hmac_sha256(
                &[0xaa; 131],
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            )),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn the_signature_covers_the_timestamp_and_the_body() {
        let a = signature("s3cret", "1700000000", br#"{"a":1}"#);
        assert!(a.starts_with("sha256="), "{a}");
        assert_eq!(a.len(), "sha256=".len() + 64);
        assert_eq!(a, signature("s3cret", "1700000000", br#"{"a":1}"#));
        assert_ne!(
            a,
            signature("s3cret", "1700000001", br#"{"a":1}"#),
            "timestamp"
        );
        assert_ne!(a, signature("s3cret", "1700000000", br#"{"a":2}"#), "body");
        assert_ne!(a, signature("other", "1700000000", br#"{"a":1}"#), "secret");
    }

    #[cfg(feature = "web")]
    #[tokio::test]
    async fn signed_delivery_lets_the_receiver_verify_the_exact_bytes() {
        use axum::{Router, body::Bytes, http::HeaderMap, routing::post};
        use std::sync::{Arc, Mutex};

        type Seen = (Option<String>, Option<String>, Vec<u8>);
        let got: Arc<Mutex<Vec<Seen>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = got.clone();
        let app = Router::new().route(
            "/hook",
            post(move |headers: HeaderMap, body: Bytes| {
                let sink = sink.clone();
                async move {
                    let h = |n: &str| {
                        headers
                            .get(n)
                            .and_then(|v| v.to_str().ok())
                            .map(String::from)
                    };
                    sink.lock().unwrap().push((
                        h("x-veyron-signature"),
                        h("x-veyron-timestamp"),
                        body.to_vec(),
                    ));
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        let url = format!("http://127.0.0.1:{port}/hook");
        let body = payload("created", &plan(&["web-1"], "restart"), "alice");

        assert!(deliver_signed(&url, &body, Some("s3cret")).await);
        assert!(deliver_signed(&url, &body, None).await);
        let g = got.lock().unwrap();
        let (sig, ts, raw) = &g[0];
        let (sig, ts) = (sig.clone().unwrap(), ts.clone().unwrap());
        assert_eq!(
            sig,
            signature("s3cret", &ts, raw),
            "receiver recomputes the same value"
        );
        assert_eq!(
            serde_json::from_slice::<Value>(raw).unwrap()["event"],
            "plan.created"
        );
        assert!(ts.parse::<i64>().is_ok(), "{ts}");
        assert!(
            g[1].0.is_none() && g[1].1.is_none(),
            "no secret, no signature headers"
        );
    }
}
