use thiserror::Error;

#[derive(Error, Debug)]
pub enum VMRogueError {
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
}
