// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VeyronError {
    #[error("VM '{0}' not found")]
    VmNotFound(String),

    #[error("VM '{0}' already exists")]
    VmExists(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Operation timed out: {0}")]
    Timeout(String),

    #[error("Feature not implemented: {0}")]
    NotImplemented(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Resource conflict: {0}")]
    ResourceConflict(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Kubernetes API error: {0}")]
    KubeError(String),
}

#[cfg(feature = "web")]
impl axum::response::IntoResponse for VeyronError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::response::Json;

        let (status, error_code) = match &self {
            VeyronError::VmNotFound(_) => (StatusCode::NOT_FOUND, "VM_NOT_FOUND"),
            VeyronError::VmExists(_) => (StatusCode::CONFLICT, "VM_EXISTS"),
            VeyronError::ValidationError(_) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            VeyronError::ConfigError(_) => (StatusCode::BAD_REQUEST, "CONFIG_ERROR"),
            VeyronError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, "TIMEOUT"),
            VeyronError::NotImplemented(_) => (StatusCode::NOT_IMPLEMENTED, "NOT_IMPLEMENTED"),
            VeyronError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            VeyronError::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            VeyronError::ResourceConflict(_) => (StatusCode::CONFLICT, "RESOURCE_CONFLICT"),
            VeyronError::ServiceUnavailable(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "SERVICE_UNAVAILABLE")
            }
            VeyronError::KubeError(_) => (StatusCode::BAD_GATEWAY, "KUBERNETES_ERROR"),
        };

        // Sanitize KubeError to avoid leaking internal K8s API details
        let message = match &self {
            VeyronError::KubeError(_) => "Kubernetes API error".to_string(),
            _ => self.to_string(),
        };

        let body = serde_json::json!({
            "error": error_code,
            "message": message,
        });

        (status, Json(body)).into_response()
    }
}
