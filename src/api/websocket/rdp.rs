use serde::{Deserialize, Serialize};

/// RDP WebSocket session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpSessionRequest {
    /// Target VM name
    pub vm_name: String,
    /// Kubernetes namespace
    pub namespace: String,
    /// Username for RDP authentication
    pub username: Option<String>,
    /// Domain for Windows authentication
    pub domain: Option<String>,
    /// Display width in pixels
    pub width: Option<u32>,
    /// Display height in pixels
    pub height: Option<u32>,
    /// Color depth
    pub color_depth: Option<u8>,
    /// Security protocol
    pub security: Option<String>,
    /// Enable clipboard sharing
    pub clipboard: Option<bool>,
    /// Enable audio redirection
    pub audio: Option<bool>,
}

/// RDP WebSocket session info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpSession {
    /// Unique session identifier
    pub session_id: String,
    /// Target VM name
    pub vm_name: String,
    /// Kubernetes namespace
    pub namespace: String,
    /// Whether the session is connected
    pub connected: bool,
    /// Session creation timestamp
    pub created_at: String,
    /// Display width
    pub width: u32,
    /// Display height
    pub height: u32,
    /// Color depth
    pub color_depth: u8,
}

/// RDP message types sent/received over WebSocket
///
/// The WebSocket protocol uses a binary frame format:
/// - Client-to-server: input events (mouse, keyboard, clipboard)
/// - Server-to-client: display updates (bitmap regions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpMessage {
    /// Session identifier
    pub session_id: String,
    /// Message payload (base64-encoded for binary data)
    pub data: String,
    /// Message type
    pub message_type: RdpMessageType,
}

/// Types of RDP WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RdpMessageType {
    /// Mouse input event (position, buttons, wheel)
    MouseInput,
    /// Keyboard input event (scancode, key state)
    KeyboardInput,
    /// Bitmap/display update from server
    BitmapUpdate,
    /// Clipboard data transfer (client -> server)
    ClipboardSend,
    /// Clipboard data transfer (server -> client)
    ClipboardReceive,
    /// Audio data from server
    AudioData,
    /// Display resize request
    Resize,
    /// Connection state change notification
    StateChange,
    /// Cursor shape/position update from server
    CursorUpdate,
    /// Keep-alive ping
    Ping,
    /// Close session
    Close,
    /// Error notification
    Error,
}

/// Mouse input event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpMouseEvent {
    /// X coordinate
    pub x: u16,
    /// Y coordinate
    pub y: u16,
    /// Button flags (bit 0: left, bit 1: right, bit 2: middle)
    pub buttons: u8,
    /// Whether this is a button press (true) or release (false)
    pub pressed: bool,
    /// Mouse wheel delta (positive = up, negative = down)
    pub wheel_delta: i16,
}

/// Keyboard input event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpKeyboardEvent {
    /// RDP scancode
    pub scancode: u16,
    /// Whether the key is pressed (true) or released (false)
    pub pressed: bool,
    /// Whether this is an extended key
    pub extended: bool,
}

/// Bitmap update region from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpBitmapUpdate {
    /// Left coordinate of update region
    pub left: u16,
    /// Top coordinate of update region
    pub top: u16,
    /// Right coordinate of update region
    pub right: u16,
    /// Bottom coordinate of update region
    pub bottom: u16,
    /// Width of bitmap data
    pub width: u16,
    /// Height of bitmap data
    pub height: u16,
    /// Bits per pixel
    pub bpp: u8,
    /// Whether the data is compressed
    pub compressed: bool,
    /// Bitmap data (base64-encoded)
    pub data: String,
}

/// Cursor update from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpCursorUpdate {
    /// Cursor hotspot X
    pub hotspot_x: u16,
    /// Cursor hotspot Y
    pub hotspot_y: u16,
    /// Cursor width
    pub width: u16,
    /// Cursor height
    pub height: u16,
    /// Cursor image data (base64-encoded RGBA)
    pub data: String,
    /// Whether to hide the cursor
    pub hidden: bool,
}

/// RDP connection state change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpStateChange {
    /// Previous state
    pub from: String,
    /// New state
    pub to: String,
    /// Reason for state change
    pub reason: Option<String>,
    /// Disconnect error code (if applicable)
    pub error_code: Option<u32>,
}
