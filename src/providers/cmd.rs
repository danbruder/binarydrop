use tracing::{info, instrument};

use crate::config;
use crate::models::App;
use crate::providers::{Handle, Provider};
use sqlx::{Pool, Sqlite};
use std::process::Child;
use std::process::Command;
use std::process::Stdio;

pub struct CmdProvider {}

#[derive(Debug, thiserror::Error)]
pub enum CmdProviderError {
    #[error("Invalid binary path: {0}")]
    InvalidBinaryPath(String),
    #[error("Log broken: {0}")]
    LogBroken(String),
    #[error("Failed to start app: {0}")]
    StartFailed(String),
    #[error("Config error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
}

impl Handle for Child {
    fn id(&self) -> u32 {
        self.id()
    }
}

impl Provider for CmdProvider {
    type Handle = Child;

    #[instrument(skip(self, pool))]
    async fn setup(&self, pool: &Pool<Sqlite>, app: &App) -> anyhow::Result<App> {
        // Create app directory
        let _ = config::get_app_dir(&app.name)?;

        // Get next available port
        let port = config::get_next_available_port(pool).await?;

        let app = app.with_port(port);

        Ok(app)
    }

    #[instrument(skip(self))]
    async fn teardown(&self, app: &App) -> anyhow::Result<App> {
        let app_dir = config::get_app_dir(&app.name)?;
        std::fs::remove_dir_all(&app_dir)?;

        Ok(app.clone())
    }

    #[instrument(skip(self, app))]
    async fn start(&self, app: &App) -> anyhow::Result<Child> {
        let binary_path = match &app.binary_path {
            Some(path) => path,
            None => {
                return Err(CmdProviderError::InvalidBinaryPath(app.name.clone()).into());
            }
        };

        info!("Starting app '{}' from binary {}", app.name, binary_path);

        // Get log file path
        let log_path = config::get_app_log_path(&app.name)?;
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| CmdProviderError::LogBroken(e.to_string()))?;

        // Get data directory path
        let data_dir = config::get_app_data_dir(&app.name)?;

        // Start process
        let mut cmd = Command::new(binary_path);

        // Add environment variables
        cmd.env("PORT", app.port.map_or("".to_string(), |p| p.to_string()));
        cmd.env("APP_NAME", &app.name);
        cmd.env("DATA_DIR", &data_dir);
        for (key, value) in &app.environment {
            cmd.env(key, value);
        }

        // Configure I/O
        let log_file_clone = log_file
            .try_clone()
            .map_err(|e| CmdProviderError::StartFailed(e.to_string()))?;
        cmd.stdout(Stdio::from(log_file_clone))
            .stderr(Stdio::from(log_file));

        // Start the process
        let child = cmd
            .spawn()
            .map_err(|e| CmdProviderError::StartFailed(e.to_string()))?;

        Ok(child)
    }
}
