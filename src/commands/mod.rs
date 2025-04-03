// src/commands/mod.rs
pub mod app_command;
pub mod server_command;

// src/errors.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BinaryDropError {
    #[error("App not found: {0}")]
    AppNotFound(String),

    #[error("App already exists: {0}")]
    AppAlreadyExists(String),

    #[error("Invalid app name: {0}")]
    InvalidAppName(String),

    #[error("Binary not found: {0}")]
    BinaryNotFound(String),

    #[error("Process error: {0}")]
    ProcessError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}
