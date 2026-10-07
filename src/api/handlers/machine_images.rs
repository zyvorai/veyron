// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Kairon image catalog (`MachineImage`) and Veyron's image upload store — the
//! Kairon-native replacement for CDI upload, DataVolumes and DataSources.
//!
//! Uploads stream into a content-addressed directory (`blobs/sha256/<hex>`) with
//! a name pointer per upload (`names/<name>.json`). Publishing turns an upload
//! (or any http(s) URL + digest) into a cluster-scoped `MachineImage`; nodes
//! download the blob from `/api/v1/image-store/blobs/<hex>`. That path is
//! unauthenticated because kairon-node sends no credentials: the 256-bit digest
//! is the capability, and every node verifies the bytes against it.

use std::path::{Path as FsPath, PathBuf};

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, put},
};
use futures_util::StreamExt;
use kube::api::{Api, DeleteParams, ListParams, PostParams};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::api::http_server::web::SharedState;
use crate::kairon::{CatalogImage, CatalogImageDefaults, CatalogImageSpec, ImageSource};

pub const KIND_DISK: &str = "disk";
pub const KIND_ISO: &str = "iso";
const STORE_FORMATS: [&str; 7] = ["iso", "raw", "qcow2", "vmdk", "vhd", "vhdx", "ova"];
const BLOB_PREFIX: &str = "/api/v1/image-store/blobs/";

pub(crate) type Reply = Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)>;

pub(crate) fn fail(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({"success": false, "error": {"code": code, "message": message.into()}})),
    )
}

pub(crate) fn ok(status: StatusCode, data: Value) -> Reply {
    Ok((status, Json(json!({"success": true, "data": data}))))
}

pub(crate) fn require_kairon() -> Result<(), (StatusCode, Json<Value>)> {
    if crate::api::vm_backend::is_kairon() {
        Ok(())
    } else {
        Err(fail(
            StatusCode::NOT_IMPLEMENTED,
            "KAIRON_REQUIRED",
            "the image catalog is Kairon's MachineImage; on KubeVirt use /api/v1/images/upload and /publish",
        ))
    }
}

/// Catalog and upload names: DNS-1123 subdomain-ish, so they work as CR names.
pub fn valid_image_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
        && name.starts_with(|c: char| c.is_ascii_alphanumeric())
        && name.ends_with(|c: char| c.is_ascii_alphanumeric())
}

fn valid_hex_digest(hex: &str) -> bool {
    hex.len() == 64
        && hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn normalize_digest(raw: &str) -> Option<String> {
    let hex = raw.trim().strip_prefix("sha256:").unwrap_or(raw.trim());
    let hex = hex.to_ascii_lowercase();
    valid_hex_digest(&hex).then(|| format!("sha256:{hex}"))
}

// ── Upload store ─────────────────────────────────────────────

pub fn store_dir() -> PathBuf {
    std::env::var("VEYRON_IMAGE_STORE_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/veyron/image-store"))
}

fn blob_path(dir: &FsPath, hex: &str) -> PathBuf {
    dir.join("blobs").join("sha256").join(hex)
}

fn name_path(dir: &FsPath, name: &str) -> PathBuf {
    dir.join("names").join(format!("{name}.json"))
}

/// Where nodes download blobs from: `VEYRON_IMAGE_STORE_PUBLIC_URL` (TLS
/// verified unless `VEYRON_IMAGE_STORE_INSECURE_TLS=1`), else the API's
/// NodePort with its self-signed certificate.
#[derive(Debug, Clone, Default)]
pub struct StoreLocation {
    pub public_url: Option<String>,
    pub public_insecure: bool,
    pub node_host: Option<String>,
    pub node_port: Option<String>,
}

impl StoreLocation {
    pub fn from_env() -> Self {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        Self {
            public_url: var("VEYRON_IMAGE_STORE_PUBLIC_URL"),
            public_insecure: var("VEYRON_IMAGE_STORE_INSECURE_TLS")
                .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            node_host: var("VEYRON_API_NODE_HOST"),
            node_port: var("VEYRON_API_NODE_PORT"),
        }
    }

    /// Blob URL and whether its certificate must be skipped.
    pub fn blob_url(&self, hex: &str) -> Option<(String, bool)> {
        if let Some(base) = &self.public_url {
            let base = base.trim().trim_end_matches('/');
            return Some((format!("{base}{BLOB_PREFIX}{hex}"), self.public_insecure));
        }
        let host = self.node_host.as_deref()?;
        let port = self.node_port.as_deref().unwrap_or("30151");
        Some((format!("https://{host}:{port}{BLOB_PREFIX}{hex}"), true))
    }
}

pub fn is_blob_path(path: &str) -> bool {
    path.strip_prefix(BLOB_PREFIX).is_some_and(valid_hex_digest)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredImage {
    pub name: String,
    pub digest: String,
    pub size: u64,
    pub format: String,
    pub uploaded_at: String,
}

impl StoredImage {
    fn hex(&self) -> &str {
        self.digest.trim_start_matches("sha256:")
    }

    fn to_json(&self) -> Value {
        let mut v = serde_json::to_value(self).unwrap_or_default();
        if let Some((url, _)) = StoreLocation::from_env().blob_url(self.hex()) {
            v["url"] = json!(url);
        }
        v
    }
}

pub(crate) async fn read_stored(dir: &FsPath, name: &str) -> Option<StoredImage> {
    let b = tokio::fs::read(name_path(dir, name)).await.ok()?;
    serde_json::from_slice(&b).ok()
}

async fn list_stored(dir: &FsPath) -> Vec<StoredImage> {
    let mut out = Vec::new();
    let Ok(mut rd) = tokio::fs::read_dir(dir.join("names")).await else {
        return out;
    };
    while let Ok(Some(e)) = rd.next_entry().await {
        let Some(name) = e
            .file_name()
            .to_str()
            .and_then(|n| n.strip_suffix(".json"))
            .map(str::to_string)
        else {
            continue;
        };
        if let Some(img) = read_stored(dir, &name).await {
            out.push(img);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

async fn write_atomic(path: &FsPath, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await
}

#[derive(Debug, Deserialize)]
struct UploadQuery {
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    replace: bool,
}

/// Format from the query, else the file extension in the name.
fn upload_format(q: Option<&str>, name: &str) -> Option<String> {
    let f = match q {
        Some(f) if !f.is_empty() => f.to_ascii_lowercase(),
        _ => name.rsplit('.').next().unwrap_or("").to_ascii_lowercase(),
    };
    let f = if f == "img" { "raw".to_string() } else { f };
    STORE_FORMATS.contains(&f.as_str()).then_some(f)
}

/// Why streaming into the store failed; `status` is what an upload returns.
#[derive(Debug)]
pub struct StoreError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl StoreError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    fn reply(self) -> (StatusCode, Json<Value>) {
        fail(self.status, self.code, self.message)
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Stream `body` into the store as upload `name`, hashing as it goes. `want`
/// rejects the blob when its digest differs.
pub async fn store_stream<S, B, E>(
    dir: &FsPath,
    name: &str,
    format: String,
    mut body: S,
    want: Option<String>,
    replace: bool,
) -> Result<StoredImage, StoreError>
where
    S: futures_util::Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    E: std::fmt::Display,
{
    if !replace && read_stored(dir, name).await.is_some() {
        return Err(StoreError::new(
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
            format!("image {name} already uploaded; add ?replace=true to overwrite"),
        ));
    }
    let io_err = |e: std::io::Error| {
        StoreError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "STORE_FAILED",
            format!("image store {}: {e}", dir.display()),
        )
    };
    let blobs = dir.join("blobs").join("sha256");
    tokio::fs::create_dir_all(&blobs).await.map_err(io_err)?;
    let tmp = blobs.join(format!(".upload-{:016x}", rand::random::<u64>()));
    let result = async {
        let mut file = tokio::fs::File::create(&tmp).await.map_err(io_err)?;
        let mut hasher = Sha256::new();
        let mut size: u64 = 0;
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| {
                StoreError::new(
                    StatusCode::BAD_REQUEST,
                    "UPLOAD_ABORTED",
                    format!("reading upload: {e}"),
                )
            })?;
            let chunk = chunk.as_ref();
            hasher.update(chunk);
            size += chunk.len() as u64;
            file.write_all(chunk).await.map_err(io_err)?;
        }
        file.flush().await.map_err(io_err)?;
        file.sync_all().await.map_err(io_err)?;
        Ok::<_, StoreError>((hex::encode(hasher.finalize()), size))
    }
    .await;
    let (hex, size) = match result {
        Ok(v) => v,
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(e);
        }
    };
    let digest = format!("sha256:{hex}");
    if size == 0 {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(StoreError::new(
            StatusCode::BAD_REQUEST,
            "EMPTY_UPLOAD",
            "upload body is empty",
        ));
    }
    if let Some(want) = want.filter(|w| *w != digest) {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(StoreError::new(
            StatusCode::BAD_REQUEST,
            "DIGEST_MISMATCH",
            format!("received {digest}, client sent {want}"),
        ));
    }
    tokio::fs::rename(&tmp, blob_path(dir, &hex))
        .await
        .map_err(io_err)?;
    let img = StoredImage {
        name: name.to_string(),
        digest,
        size,
        format,
        uploaded_at: chrono::Utc::now().to_rfc3339(),
    };
    write_atomic(
        &name_path(dir, name),
        &serde_json::to_vec(&img).unwrap_or_default(),
    )
    .await
    .map_err(io_err)?;
    Ok(img)
}

/// `PUT /api/v1/image-store/:name?format=iso` — stream the body into the store.
async fn upload(
    Path(name): Path<String>,
    Query(q): Query<UploadQuery>,
    headers: HeaderMap,
    body: Body,
) -> Reply {
    require_kairon()?;
    if !valid_image_name(&name) {
        return Err(fail(
            StatusCode::BAD_REQUEST,
            "INVALID_NAME",
            "name must be lowercase letters, digits, '-' or '.', up to 63 characters",
        ));
    }
    let Some(format) = upload_format(q.format.as_deref(), &name) else {
        return Err(fail(
            StatusCode::BAD_REQUEST,
            "INVALID_FORMAT",
            format!("format must be one of {}", STORE_FORMATS.join(", ")),
        ));
    };
    let want = match headers.get("x-image-digest").and_then(|v| v.to_str().ok()) {
        Some(raw) => Some(normalize_digest(raw).ok_or_else(|| {
            fail(
                StatusCode::BAD_REQUEST,
                "INVALID_DIGEST",
                "X-Image-Digest must be sha256:<64 hex>",
            )
        })?),
        None => None,
    };
    let img = store_stream(
        &store_dir(),
        &name,
        format,
        body.into_data_stream(),
        want,
        q.replace,
    )
    .await
    .map_err(StoreError::reply)?;
    ok(StatusCode::CREATED, img.to_json())
}

/// `GET /api/v1/image-store/blobs/:hex` — stream a blob to a node.
async fn download_blob(Path(hex): Path<String>) -> Response {
    if !valid_hex_digest(&hex) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok(file) = tokio::fs::File::open(blob_path(&store_dir(), &hex)).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let len = file.metadata().await.map(|m| m.len()).unwrap_or(0);
    let chunks = futures_util::stream::unfold(Some(file), |file| async move {
        let mut file = file?;
        let mut buf = vec![0u8; 1 << 20];
        match file.read(&mut buf).await {
            Ok(0) => None,
            Ok(n) => {
                buf.truncate(n);
                Some((Ok::<_, std::io::Error>(buf), Some(file)))
            }
            Err(e) => Some((Err(e), None)),
        }
    });
    (
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_LENGTH, len.to_string()),
        ],
        Body::from_stream(chunks),
    )
        .into_response()
}

/// `GET /api/v1/image-store`
async fn list_store() -> Reply {
    require_kairon()?;
    let items: Vec<Value> = list_stored(&store_dir())
        .await
        .iter()
        .map(StoredImage::to_json)
        .collect();
    ok(StatusCode::OK, json!(items))
}

/// `DELETE /api/v1/image-store/:name` — drop the name, and the blob once no
/// other upload or `MachineImage` points at it.
async fn delete_stored(State(state): State<SharedState>, Path(name): Path<String>) -> Reply {
    require_kairon()?;
    let dir = store_dir();
    let Some(img) = read_stored(&dir, &name).await else {
        return Err(fail(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("image {name} is not in the store"),
        ));
    };
    let client = state.read().await.client().client();
    let catalog = Api::<CatalogImage>::all(client)
        .list(&ListParams::default())
        .await
        .map_err(|e| {
            fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LIST_FAILED",
                e.to_string(),
            )
        })?;
    let users: Vec<String> = catalog
        .items
        .iter()
        .filter(|m| m.spec.digest == img.digest)
        .filter_map(|m| m.metadata.name.clone())
        .collect();
    if !users.is_empty() {
        return Err(fail(
            StatusCode::CONFLICT,
            "IN_USE",
            format!(
                "published as MachineImage {}; delete those first",
                users.join(", ")
            ),
        ));
    }
    let io_err = |e: std::io::Error| {
        fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "STORE_FAILED",
            e.to_string(),
        )
    };
    tokio::fs::remove_file(name_path(&dir, &name))
        .await
        .map_err(io_err)?;
    let shared = list_stored(&dir)
        .await
        .iter()
        .any(|o| o.digest == img.digest);
    if !shared {
        let _ = tokio::fs::remove_file(blob_path(&dir, img.hex())).await;
    }
    ok(
        StatusCode::OK,
        json!({"name": name, "blob_removed": !shared}),
    )
}

// ── Catalog (MachineImage) ───────────────────────────────────

pub async fn get_catalog_image(
    client: &kube::Client,
    name: &str,
) -> Result<Option<CatalogImage>, kube::Error> {
    Api::<CatalogImage>::all(client.clone()).get_opt(name).await
}

pub fn catalog_kind(m: &CatalogImage) -> &str {
    m.spec.kind.as_deref().unwrap_or(KIND_DISK)
}

pub(crate) fn catalog_json(m: &CatalogImage) -> Value {
    json!({
        "name": m.metadata.name,
        "display_name": m.spec.display_name,
        "family": m.spec.family,
        "version": m.spec.version,
        "kind": catalog_kind(m),
        "os": m.spec.os,
        "source": m.spec.source.http_url.clone().or_else(|| m.spec.source.oci.clone()),
        "format": m.spec.source.format,
        "digest": m.spec.digest,
        "defaults": m.spec.defaults.as_ref().map(|d| json!({
            "cpu": d.cpu,
            "memory": d.memory,
            "disk_size": d.disk_size,
        })),
        "insecure_skip_tls_verify": m.spec.source.insecure_skip_tls_verify,
        "deprecated": m.spec.deprecated,
        "created": m.metadata.creation_timestamp.as_ref().map(|t| t.0.to_rfc3339()),
    })
}

async fn list_catalog(State(state): State<SharedState>) -> Reply {
    require_kairon()?;
    let client = state.read().await.client().client();
    let list = Api::<CatalogImage>::all(client)
        .list(&ListParams::default())
        .await
        .map_err(|e| {
            fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LIST_FAILED",
                e.to_string(),
            )
        })?;
    let mut items: Vec<Value> = list.items.iter().map(catalog_json).collect();
    items.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    ok(StatusCode::OK, json!(items))
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PublishRequest {
    pub name: String,
    /// Name of an upload in the image store.
    #[serde(default)]
    pub from_store: Option<String>,
    /// Or an http(s) URL every node can reach, with its digest and format.
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub insecure_skip_tls_verify: bool,
    /// `disk` (default) or `iso`; an `.iso` upload defaults to `iso`.
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub cpu: Option<String>,
    #[serde(default)]
    pub memory: Option<String>,
    #[serde(default)]
    pub disk_size: Option<String>,
}

/// Build the `MachineImage` for a publish request. `stored` is the resolved
/// upload when `from_store` is set.
pub fn catalog_image_for(
    req: &PublishRequest,
    stored: Option<&StoredImage>,
    loc: &StoreLocation,
) -> Result<CatalogImage, String> {
    if !valid_image_name(&req.name) {
        return Err(format!("name {:?} must be a DNS-1123 name", req.name));
    }
    let (url, digest, format, insecure) = match stored {
        Some(s) => {
            let (url, insecure) = loc.blob_url(s.hex()).ok_or(
                "nodes can't reach the image store: set VEYRON_IMAGE_STORE_PUBLIC_URL or VEYRON_API_NODE_HOST",
            )?;
            (url, s.digest.clone(), s.format.clone(), insecure)
        }
        None => {
            let url = req
                .url
                .as_deref()
                .map(str::trim)
                .filter(|u| u.starts_with("http://") || u.starts_with("https://"))
                .ok_or("give from_store, or url (http:// or https://) with sha256 and format")?;
            let digest = req
                .sha256
                .as_deref()
                .and_then(normalize_digest)
                .ok_or("sha256 must be 64 hex characters, optionally prefixed with sha256:")?;
            let format = req
                .format
                .as_deref()
                .unwrap_or("qcow2")
                .to_ascii_lowercase();
            (
                url.to_string(),
                digest,
                format,
                req.insecure_skip_tls_verify,
            )
        }
    };
    let kind = match req.kind.as_deref() {
        Some(k) => k.to_ascii_lowercase(),
        None if format == "iso" => KIND_ISO.into(),
        None => KIND_DISK.into(),
    };
    if kind != KIND_DISK && kind != KIND_ISO {
        return Err("kind must be disk or iso".into());
    }
    // ISOs are attached as-is; nodes cache them as raw bytes.
    let format = match (kind.as_str(), format.as_str()) {
        (KIND_ISO, "iso" | "raw") => "raw".to_string(),
        (KIND_ISO, f) => return Err(format!("an iso image can't have format {f}")),
        (_, "iso") => return Err("an .iso upload must be published with kind iso".into()),
        (_, f) if STORE_FORMATS.contains(&f) => f.to_string(),
        (_, f) => return Err(format!("unsupported format {f}")),
    };
    let defaults = CatalogImageDefaults {
        cpu: req.cpu.clone(),
        memory: req.memory.clone(),
        disk_size: req.disk_size.clone(),
    };
    let spec = CatalogImageSpec {
        display_name: req.display_name.clone(),
        family: req.family.clone(),
        version: req.version.clone(),
        kind: Some(kind),
        os: req.os.clone(),
        source: ImageSource {
            http_url: Some(url),
            format: Some(format),
            insecure_skip_tls_verify: insecure,
            ..Default::default()
        },
        digest,
        defaults: (defaults != CatalogImageDefaults::default()).then_some(defaults),
        ..Default::default()
    };
    Ok(CatalogImage::new(&req.name, spec))
}

async fn publish(State(state): State<SharedState>, Json(req): Json<PublishRequest>) -> Reply {
    require_kairon()?;
    let stored = match req.from_store.as_deref() {
        Some(n) => Some(read_stored(&store_dir(), n).await.ok_or_else(|| {
            fail(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                format!("image {n} is not in the store"),
            )
        })?),
        None => None,
    };
    let img = catalog_image_for(&req, stored.as_ref(), &StoreLocation::from_env())
        .map_err(|e| fail(StatusCode::BAD_REQUEST, "INVALID_REQUEST", e))?;
    let client = state.read().await.client().client();
    match Api::<CatalogImage>::all(client)
        .create(&PostParams::default(), &img)
        .await
    {
        Ok(created) => ok(StatusCode::CREATED, catalog_json(&created)),
        Err(kube::Error::Api(e)) if e.code == 409 => Err(fail(
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
            format!(
                "MachineImage {} already exists; publish a new version name",
                req.name
            ),
        )),
        Err(e) => Err(fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "PUBLISH_FAILED",
            format!("create MachineImage {}: {e}", req.name),
        )),
    }
}

async fn delete_catalog(State(state): State<SharedState>, Path(name): Path<String>) -> Reply {
    require_kairon()?;
    let client = state.read().await.client().client();
    match Api::<CatalogImage>::all(client)
        .delete(&name, &DeleteParams::default())
        .await
    {
        Ok(_) => ok(StatusCode::OK, json!({"name": name})),
        Err(kube::Error::Api(e)) if e.code == 404 => Err(fail(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("MachineImage {name} not found"),
        )),
        Err(e) => Err(fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DELETE_FAILED",
            e.to_string(),
        )),
    }
}

/// Timed JSON routes, nested under `/api/v1`.
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/machine-images", get(list_catalog).post(publish))
        .route("/machine-images/:name", delete(delete_catalog))
        .route("/image-store", get(list_store))
        .route("/image-store/:name", delete(delete_stored))
        .with_state(state)
}

/// Upload and blob download: untimed and without the global body limit, at
/// absolute paths.
pub fn transfer_router(state: SharedState) -> Router {
    Router::new()
        .route("/api/v1/image-store/:name", put(upload))
        .route("/api/v1/image-store/blobs/:hex", get(download_blob))
        .layer(DefaultBodyLimit::disable())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(format: &str) -> StoredImage {
        StoredImage {
            name: "win2022.iso".into(),
            digest: format!("sha256:{}", "a".repeat(64)),
            size: 10,
            format: format.into(),
            uploaded_at: String::new(),
        }
    }

    #[test]
    fn names_and_blob_paths() {
        assert!(valid_image_name("windows-2022.v1"));
        assert!(!valid_image_name("Win"));
        assert!(!valid_image_name("-a"));
        assert!(is_blob_path(&format!("{BLOB_PREFIX}{}", "0f".repeat(32))));
        assert!(!is_blob_path(&format!("{BLOB_PREFIX}../etc/passwd")));
        assert!(!is_blob_path(&format!("{BLOB_PREFIX}{}", "F".repeat(64))));
    }

    #[test]
    fn upload_format_from_query_or_extension() {
        assert_eq!(upload_format(None, "win.iso").as_deref(), Some("iso"));
        assert_eq!(upload_format(None, "disk.img").as_deref(), Some("raw"));
        assert_eq!(upload_format(Some("QCOW2"), "x").as_deref(), Some("qcow2"));
        assert_eq!(upload_format(None, "noext"), None);
    }

    #[test]
    fn publish_url_disk_and_iso_kinds() {
        let req = PublishRequest {
            name: "ubuntu-24.04".into(),
            url: Some("https://images.example/u.qcow2".into()),
            sha256: Some("B".repeat(64)),
            disk_size: Some("20Gi".into()),
            ..Default::default()
        };
        let img = catalog_image_for(&req, None, &StoreLocation::default()).unwrap();
        assert_eq!(img.spec.kind.as_deref(), Some(KIND_DISK));
        assert_eq!(img.spec.source.format.as_deref(), Some("qcow2"));
        assert_eq!(img.spec.digest, format!("sha256:{}", "b".repeat(64)));
        assert_eq!(
            img.spec.defaults.unwrap().disk_size.as_deref(),
            Some("20Gi")
        );

        let iso = PublishRequest {
            format: Some("iso".into()),
            ..req.clone()
        };
        let img = catalog_image_for(&iso, None, &StoreLocation::default()).unwrap();
        assert_eq!(img.spec.kind.as_deref(), Some(KIND_ISO));
        assert_eq!(img.spec.source.format.as_deref(), Some("raw"));

        let bad = PublishRequest {
            kind: Some("disk".into()),
            ..iso
        };
        assert!(catalog_image_for(&bad, None, &StoreLocation::default()).is_err());
        assert!(
            catalog_image_for(
                &PublishRequest {
                    name: "x".into(),
                    ..Default::default()
                },
                None,
                &StoreLocation::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn publish_from_store_uses_node_url() {
        let req = PublishRequest {
            name: "win2022-iso".into(),
            from_store: Some("win2022.iso".into()),
            ..Default::default()
        };
        assert!(catalog_image_for(&req, Some(&stored("iso")), &StoreLocation::default()).is_err());
        let loc = StoreLocation {
            node_host: Some("10.0.0.5".into()),
            ..Default::default()
        };
        let img = catalog_image_for(&req, Some(&stored("iso")), &loc).unwrap();
        assert_eq!(img.spec.kind.as_deref(), Some(KIND_ISO));
        assert_eq!(
            img.spec.source.http_url.as_deref(),
            Some(format!("https://10.0.0.5:30151{BLOB_PREFIX}{}", "a".repeat(64)).as_str())
        );
        assert!(img.spec.source.insecure_skip_tls_verify);
        // Kairon's CRD field; serde's camelCase would give insecureSkipTlsVerify,
        // which the API server prunes.
        let v = serde_json::to_value(&img).unwrap();
        assert_eq!(v["spec"]["source"]["insecureSkipTLSVerify"], json!(true));

        let loc = StoreLocation {
            public_url: Some("https://images.example/".into()),
            ..loc
        };
        let img = catalog_image_for(&req, Some(&stored("iso")), &loc).unwrap();
        assert!(!img.spec.source.insecure_skip_tls_verify);
        assert!(
            img.spec
                .source
                .http_url
                .unwrap()
                .starts_with("https://images.example/api/v1/")
        );
    }
}
