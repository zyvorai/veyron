#[cfg(feature = "web")]
use axum::{
    Router,
    extract::Path,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// RDP session security protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum RdpSecurityProtocol {
    /// Standard RDP security
    Rdp,
    /// TLS/SSL security
    Tls,
    /// Network Level Authentication (NLA)
    Nla,
    /// NLA with extended protocol
    NlaExtended,
    /// Hybrid security (CredSSP + TLS)
    Hybrid,
    /// Auto-negotiate best available
    #[default]
    Auto,
}

impl std::fmt::Display for RdpSecurityProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rdp => write!(f, "rdp"),
            Self::Tls => write!(f, "tls"),
            Self::Nla => write!(f, "nla"),
            Self::NlaExtended => write!(f, "nla-ext"),
            Self::Hybrid => write!(f, "hybrid"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

/// RDP connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpConnectionConfig {
    /// Target VM hostname or IP
    pub hostname: String,
    /// RDP port (default 3389)
    pub port: u16,
    /// Username for authentication
    pub username: Option<String>,
    /// Domain for Windows authentication
    pub domain: Option<String>,
    /// Display width in pixels
    pub width: u32,
    /// Display height in pixels
    pub height: u32,
    /// Color depth (15, 16, 24, or 32)
    pub color_depth: u8,
    /// Security protocol
    pub security: RdpSecurityProtocol,
    /// Enable desktop composition (Aero)
    pub desktop_composition: bool,
    /// Enable font smoothing
    pub font_smoothing: bool,
    /// Enable wallpaper display
    pub wallpaper: bool,
    /// Enable full window drag
    pub full_window_drag: bool,
    /// Enable menu animations
    pub menu_animations: bool,
    /// Enable theming
    pub theming: bool,
    /// Resize method: "reconnect" or "display-update"
    pub resize_method: String,
    /// Enable audio playback redirection
    pub audio_playback: bool,
    /// Enable audio input (microphone) redirection
    pub audio_input: bool,
    /// Enable clipboard sharing
    pub clipboard: bool,
    /// Enable printer redirection
    pub printer: bool,
    /// Enable drive/file sharing redirection
    pub drive_redirection: bool,
    /// Shared drive path (if drive_redirection enabled)
    pub drive_path: Option<String>,
    /// Console (admin) session
    pub console_session: bool,
    /// Certificate validation is always enforced for security.
    /// This field is ignored and retained only for config compatibility.
    #[serde(default, skip_serializing)]
    pub ignore_cert: bool,
    /// Gateway hostname (for RD Gateway)
    pub gateway_hostname: Option<String>,
    /// Gateway port
    pub gateway_port: Option<u16>,
    /// Gateway username
    pub gateway_username: Option<String>,
    /// Gateway domain
    pub gateway_domain: Option<String>,
    /// Initial remote application to launch
    pub remote_app: Option<String>,
    /// Remote app arguments
    pub remote_app_args: Option<String>,
    /// Connection timeout in seconds
    pub timeout_secs: u32,
}

impl Default for RdpConnectionConfig {
    fn default() -> Self {
        Self {
            hostname: String::new(),
            port: 3389,
            username: None,
            domain: None,
            width: 1920,
            height: 1080,
            color_depth: 32,
            security: RdpSecurityProtocol::default(),
            desktop_composition: false,
            font_smoothing: true,
            wallpaper: false,
            full_window_drag: false,
            menu_animations: false,
            theming: true,
            resize_method: "display-update".to_string(),
            audio_playback: false,
            audio_input: false,
            clipboard: true,
            printer: false,
            drive_redirection: false,
            drive_path: None,
            console_session: false,
            ignore_cert: false,
            gateway_hostname: None,
            gateway_port: None,
            gateway_username: None,
            gateway_domain: None,
            remote_app: None,
            remote_app_args: None,
            timeout_secs: 30,
        }
    }
}

/// RDP session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RdpSessionState {
    /// Waiting for WebSocket connection
    Pending,
    /// Establishing RDP handshake
    Connecting,
    /// Security negotiation in progress
    Authenticating,
    /// Session is active
    Connected,
    /// Session is paused/idle
    Idle,
    /// Session is being disconnected
    Disconnecting,
    /// Session has ended
    Disconnected,
    /// Session encountered an error
    Error(String),
}

impl std::fmt::Display for RdpSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Authenticating => write!(f, "Authenticating"),
            Self::Connected => write!(f, "Connected"),
            Self::Idle => write!(f, "Idle"),
            Self::Disconnecting => write!(f, "Disconnecting"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Error(e) => write!(f, "Error: {}", e),
        }
    }
}

/// RDP session response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpSessionResponse {
    /// Unique session identifier
    pub session_id: String,
    /// Target VM name
    pub vm_name: String,
    /// Kubernetes namespace
    pub namespace: String,
    /// Target hostname/IP
    pub hostname: String,
    /// RDP port
    pub port: u16,
    /// Current session state
    pub state: RdpSessionState,
    /// WebSocket endpoint URL for this session
    pub websocket_url: String,
    /// Display resolution
    pub resolution: String,
    /// Color depth
    pub color_depth: u8,
    /// Security protocol in use
    pub security: RdpSecurityProtocol,
    /// Connected username
    pub username: Option<String>,
    /// Connected domain
    pub domain: Option<String>,
    /// Session creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last activity timestamp (ISO 8601)
    pub last_activity: String,
    /// Session duration in seconds
    pub duration_secs: u64,
    /// Bytes sent to client
    pub bytes_sent: u64,
    /// Bytes received from client
    pub bytes_received: u64,
    /// Clipboard enabled
    pub clipboard_enabled: bool,
    /// Audio enabled
    pub audio_enabled: bool,
    /// Drive redirection enabled
    pub drive_enabled: bool,
}

/// Create RDP session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRdpSessionRequest {
    /// Target VM name
    pub vm_name: String,
    /// Kubernetes namespace
    pub namespace: String,
    /// Username for RDP authentication
    pub username: Option<String>,
    /// Domain for Windows authentication
    pub domain: Option<String>,
    /// Display width
    pub width: Option<u32>,
    /// Display height
    pub height: Option<u32>,
    /// Color depth (15, 16, 24, 32)
    pub color_depth: Option<u8>,
    /// Security protocol
    pub security: Option<RdpSecurityProtocol>,
    /// Enable clipboard sharing
    pub clipboard: Option<bool>,
    /// Enable audio redirection
    pub audio: Option<bool>,
    /// Enable drive redirection
    pub drive_redirection: Option<bool>,
    /// Shared drive path
    pub drive_path: Option<String>,
    /// Enable printer redirection
    pub printer: Option<bool>,
    /// Deprecated: certificate validation is always enforced.
    /// This field is accepted for compatibility but ignored.
    #[serde(default)]
    pub ignore_cert: Option<bool>,
    /// Connect to console/admin session
    pub console_session: Option<bool>,
    /// RD Gateway hostname
    pub gateway_hostname: Option<String>,
    /// Remote application to launch
    pub remote_app: Option<String>,
    /// Remote app arguments
    pub remote_app_args: Option<String>,
}

/// Resize RDP session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeRdpSessionRequest {
    /// New display width
    pub width: u32,
    /// New display height
    pub height: u32,
}

/// RDP session credentials update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpCredentialsRequest {
    /// Username
    pub username: String,
    /// Domain
    pub domain: Option<String>,
}

/// Maximum clipboard size (10 MiB) to prevent memory exhaustion
#[allow(dead_code)]
const MAX_CLIPBOARD_SIZE: usize = 10 * 1024 * 1024;

/// RDP clipboard transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpClipboardRequest {
    /// Clipboard content (text, max 10 MiB)
    pub text: Option<String>,
    /// Clipboard content type
    pub content_type: String,
}

/// RDP session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpSessionStats {
    /// Session ID
    pub session_id: String,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Frames per second
    pub fps: f32,
    /// Average latency in milliseconds
    pub latency_ms: u32,
    /// Packet loss percentage
    pub packet_loss_pct: f32,
    /// Session uptime in seconds
    pub uptime_secs: u64,
    /// Number of resize events
    pub resize_count: u32,
    /// Number of clipboard transfers
    pub clipboard_transfers: u32,
    /// Current bandwidth usage (bytes/sec)
    pub bandwidth_bps: u64,
}

/// List of available RDP-capable VMs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpCapableVm {
    /// VM name
    pub name: String,
    /// Namespace
    pub namespace: String,
    /// VM IP address
    pub ip_address: Option<String>,
    /// RDP port
    pub rdp_port: u16,
    /// Whether RDP port is reachable
    pub reachable: bool,
    /// OS type detected
    pub os_type: String,
    /// Existing active sessions count
    pub active_sessions: u32,
}

/// RDP gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpGatewayConfig {
    /// Gateway hostname
    pub hostname: String,
    /// Gateway port
    pub port: u16,
    /// Whether to use gateway credentials
    pub use_gateway_credentials: bool,
    /// Gateway authentication method
    pub auth_method: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        // Session management
        .route(
            "/rdp/sessions",
            get(list_rdp_sessions).post(create_rdp_session),
        )
        .route(
            "/rdp/sessions/{id}",
            get(get_rdp_session).delete(delete_rdp_session),
        )
        .route("/rdp/sessions/{id}/resize", post(resize_rdp_session))
        .route(
            "/rdp/sessions/{id}/clipboard",
            get(get_clipboard).post(send_clipboard),
        )
        .route("/rdp/sessions/{id}/stats", get(get_session_stats))
        .route("/rdp/sessions/{id}/screenshot", get(take_screenshot))
        .route(
            "/rdp/sessions/{id}/disconnect",
            post(disconnect_rdp_session),
        )
        .route("/rdp/sessions/{id}/reconnect", post(reconnect_rdp_session))
        // VM discovery
        .route("/rdp/vms", get(list_rdp_capable_vms))
        // Configuration
        .route("/rdp/config/defaults", get(get_default_config))
        .route(
            "/rdp/gateway",
            get(get_gateway_config).post(set_gateway_config),
        )
}

#[cfg(feature = "web")]
async fn list_rdp_sessions() -> impl IntoResponse {
    not_implemented("RDP sessions")
}

#[cfg(feature = "web")]
async fn create_rdp_session(axum::Json(req): axum::Json<CreateRdpSessionRequest>) -> impl IntoResponse {
    let _ = req;
    not_implemented("RDP session creation")
}

#[cfg(feature = "web")]
async fn get_rdp_session(Path(id): Path<String>) -> impl IntoResponse {
    let _ = id;
    not_implemented("RDP session retrieval")
}

#[cfg(feature = "web")]
async fn delete_rdp_session(Path(id): Path<String>) -> impl IntoResponse {
    let _ = id;
    not_implemented("RDP session deletion")
}

#[cfg(feature = "web")]
async fn resize_rdp_session(
    Path(id): Path<String>,
    axum::Json(req): axum::Json<ResizeRdpSessionRequest>,
) -> impl IntoResponse {
    let _ = (id, req);
    not_implemented("RDP session resize")
}

#[cfg(feature = "web")]
async fn get_clipboard(Path(id): Path<String>) -> impl IntoResponse {
    let _ = id;
    not_implemented("RDP clipboard retrieval")
}

#[cfg(feature = "web")]
async fn send_clipboard(
    Path(id): Path<String>,
    axum::Json(req): axum::Json<RdpClipboardRequest>,
) -> impl IntoResponse {
    let _ = (id, req);
    not_implemented("RDP clipboard send")
}

#[cfg(feature = "web")]
async fn get_session_stats(Path(id): Path<String>) -> impl IntoResponse {
    let _ = id;
    not_implemented("RDP session statistics")
}

#[cfg(feature = "web")]
async fn take_screenshot(Path(id): Path<String>) -> impl IntoResponse {
    let _ = id;
    not_implemented("RDP screenshot")
}

#[cfg(feature = "web")]
async fn disconnect_rdp_session(Path(id): Path<String>) -> impl IntoResponse {
    let _ = id;
    not_implemented("RDP session disconnect")
}

#[cfg(feature = "web")]
async fn reconnect_rdp_session(Path(id): Path<String>) -> impl IntoResponse {
    let _ = id;
    not_implemented("RDP session reconnect")
}

#[cfg(feature = "web")]
async fn list_rdp_capable_vms() -> impl IntoResponse {
    not_implemented("RDP-capable VM discovery")
}

#[cfg(feature = "web")]
async fn get_default_config() -> impl IntoResponse {
    not_implemented("RDP default configuration")
}

#[cfg(feature = "web")]
async fn get_gateway_config() -> impl IntoResponse {
    not_implemented("RDP gateway configuration")
}

#[cfg(feature = "web")]
async fn set_gateway_config(axum::Json(req): axum::Json<RdpGatewayConfig>) -> impl IntoResponse {
    let _ = req;
    not_implemented("RDP gateway configuration")
}
