//! Embedded Vite build of `web/dashboard-next` (served at `/dashboard-next/`).

use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/api/web/dashboard-next/"]
struct DashboardNextAssets;

pub async fn dashboard_next_redirect() -> Redirect {
    Redirect::permanent("/dashboard-next/")
}

pub async fn root_redirect() -> Redirect {
    Redirect::permanent("/dashboard-next/")
}

pub async fn dashboard_next_index() -> impl IntoResponse {
    serve_embedded("index.html")
}

pub async fn dashboard_next_path(Path(path): Path<String>) -> impl IntoResponse {
    let key = path.trim_start_matches('/');
    if key.is_empty() {
        return serve_embedded("index.html");
    }
    match DashboardNextAssets::get(key) {
        Some(_) => serve_embedded(key),
        None => serve_embedded("index.html"),
    }
}

fn serve_embedded(path: &str) -> Response {
    match DashboardNextAssets::get(path) {
        Some(file) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, content_type(path))],
            file.data.into_owned(),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            format!("dashboard-next asset not found: {path}"),
        )
            .into_response(),
    }
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json; charset=utf-8",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use http::StatusCode;

    #[test]
    fn content_type_maps_known_extensions() {
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type("app.js"), "application/javascript; charset=utf-8");
        assert_eq!(content_type("styles.css"), "text/css; charset=utf-8");
        assert_eq!(content_type("data.bin"), "application/octet-stream");
    }

    #[tokio::test]
    async fn root_and_dashboard_next_redirect_to_operator_ui() {
        for redirect in [root_redirect().await, dashboard_next_redirect().await] {
            let resp = redirect.into_response();
            assert_eq!(resp.status(), StatusCode::PERMANENT_REDIRECT);
            assert_eq!(
                resp.headers()
                    .get(http::header::LOCATION)
                    .and_then(|v| v.to_str().ok()),
                Some("/dashboard-next/")
            );
        }
    }

    #[tokio::test]
    async fn embedded_index_is_react_shell() {
        let resp = dashboard_next_index().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body");
        let html = String::from_utf8(body.to_vec()).expect("utf-8 html");
        assert!(html.contains("id=\"root\""));
        assert!(html.contains("/dashboard-next/assets/"));
    }

    #[tokio::test]
    async fn unknown_asset_path_falls_back_to_spa_shell() {
        let resp = dashboard_next_path(Path("does-not-exist.js".into()))
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body");
        let html = String::from_utf8(body.to_vec()).expect("utf-8 html");
        assert!(html.contains("id=\"root\""));
    }
}
