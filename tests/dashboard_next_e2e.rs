// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Live-cluster smoke test for `/dashboard-next/` and classic dashboard API pages.
//!
//! Run manually when a cluster is available:
//!   VMROGUE_E2E_URL=https://node:30151 VMROGUE_API_KEY=secret \
//!     cargo test --test dashboard_next_e2e -- --ignored --nocapture

#[test]
#[ignore = "requires VMROGUE_E2E_URL and VMROGUE_API_KEY against a live cluster"]
fn dashboard_and_operator_api_smoke() {
    let base = std::env::var("VMROGUE_E2E_URL").expect("VMROGUE_E2E_URL");
    let key = std::env::var("VMROGUE_API_KEY").expect("VMROGUE_API_KEY");
    let client = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("http client");
    let no_redirect = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("http client (no redirect)");

    let paths = [
        "/api/v1/health",
        "/api/v1/vms?namespace=all",
        "/api/v1/compliance/status?namespace=all",
        "/api/v1/heatmap/resources",
        "/api/v1/dashboards?namespace=all",
        "/api/v1/network-policies?namespace=all",
        "/api/v1/clusters",
        "/api/v1/cilium/status",
        "/api/v1/autoscaler/policies?namespace=all",
        "/api/v1/images/catalog?namespace=all",
        "/dashboard-next/",
    ];

    for path in paths {
        let url = format!("{}{}", base.trim_end_matches('/'), path);
        let mut req = client.get(&url);
        if path.starts_with("/api/") {
            req = req.header("X-API-Key", &key);
        }
        let res = req.send().unwrap_or_else(|e| panic!("GET {url}: {e}"));
        assert!(
            res.status().is_success(),
            "GET {} returned {}",
            url,
            res.status()
        );
    }

    let root = no_redirect
        .get(format!("{}/", base.trim_end_matches('/')))
        .send()
        .expect("GET /");
    assert!(
        root.status().is_redirection(),
        "GET / should redirect, got {}",
        root.status()
    );
    let loc = root
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        loc.contains("dashboard-next"),
        "GET / should redirect to dashboard-next, got Location: {loc}"
    );
}
