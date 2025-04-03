use std::path::PathBuf;
use thiserror::Error;

/// Custom error types for BinaryDrop
#[derive(Error, Debug)]
pub enum BinaryDropError {
    #[error("App not found: {0}")]
    AppNotFound(String),

    #[error("App already exists: {0}")]
    AppAlreadyExists(String),

    #[error("Invalid app name: {0}")]
    InvalidAppName(String),

    #[error("Binary not found: {0}")]
    BinaryNotFound(PathBuf),

    #[error("Binary is not executable: {0}")]
    BinaryNotExecutable(PathBuf),

    #[error("Process error: {0}")]
    ProcessError(String),

    #[error("Process not running: app={0}")]
    ProcessNotRunning(String),

    #[error("Failed to start process: {0}")]
    ProcessStartFailed(String),

    #[error("Failed to stop process: {0}")]
    ProcessStopFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Database connection failed: {0}")]
    DatabaseConnectionFailed(String),

    #[error("Database migration failed: {0}")]
    DatabaseMigrationFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Directory creation failed: {0}")]
    DirectoryCreationFailed(PathBuf),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Configuration load failed: {0}")]
    ConfigLoadFailed(String),

    #[error("Configuration save failed: {0}")]
    ConfigSaveFailed(String),

    #[error("Port allocation failed: {0}")]
    PortAllocationFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Proxy error: {0}")]
    ProxyError(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("SQLite error: {0}")]
    SqliteError(#[from] sqlx::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("TOML serialization error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<String> for BinaryDropError {
    fn from(s: String) -> Self {
        BinaryDropError::InternalError(s)
    }
}

impl From<&str> for BinaryDropError {
    fn from(s: &str) -> Self {
        BinaryDropError::InternalError(s.to_string())
    }
}

/// Result type alias for BinaryDrop errors
pub type BinaryDropResult<T> = Result<T, BinaryDropError>;

/// Helper functions for error handling
pub mod util {
    use super::*;
    use std::path::Path;

    /// Check if a file exists, return an error if not
    pub fn check_file_exists(path: impl AsRef<Path>) -> BinaryDropResult<PathBuf> {
        let path_buf = path.as_ref().to_path_buf();
        if path_buf.exists() {
            Ok(path_buf)
        } else {
            Err(BinaryDropError::FileNotFound(path_buf))
        }
    }

    /// Check if a binary is executable, return an error if not
    #[cfg(unix)]
    pub fn check_binary_executable(path: impl AsRef<Path>) -> BinaryDropResult<PathBuf> {
        use std::os::unix::fs::PermissionsExt;

        let path_buf = path.as_ref().to_path_buf();
        if !path_buf.exists() {
            return Err(BinaryDropError::BinaryNotFound(path_buf));
        }

        let metadata = std::fs::metadata(&path_buf)
            .map_err(|_| BinaryDropError::IoError(std::io::Error::last_os_error()))?;

        let permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            return Err(BinaryDropError::BinaryNotExecutable(path_buf));
        }

        Ok(path_buf)
    }

    /// Create a directory if it doesn't exist
    pub fn ensure_directory(path: impl AsRef<Path>) -> BinaryDropResult<PathBuf> {
        let path_buf = path.as_ref().to_path_buf();
        if !path_buf.exists() {
            std::fs::create_dir_all(&path_buf)
                .map_err(|_| BinaryDropError::DirectoryCreationFailed(path_buf.clone()))?;
        }
        Ok(path_buf)
    }

    /// Validate app name
    pub fn validate_app_name(name: &str) -> BinaryDropResult<()> {
        if name.is_empty() || name.len() > 64 {
            return Err(BinaryDropError::InvalidAppName(
                "App name must be between 1 and 64 characters".to_string(),
            ));
        }

        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return Err(BinaryDropError::InvalidAppName(
                "App name must contain only lowercase letters, numbers, hyphens, and underscores"
                    .to_string(),
            ));
        }

        Ok(())
    }
}
