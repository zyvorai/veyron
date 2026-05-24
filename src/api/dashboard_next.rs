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
