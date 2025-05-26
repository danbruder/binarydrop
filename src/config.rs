use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

static TEST_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static TEST_CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    IoError(String),
    #[error("Port error: {0}")]
    PortError(String),
    #[error("TOML parsing error: {0}")]
    TomlError(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub proxy_host: String,
    pub proxy_port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    pub base_url: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            proxy_host: "0.0.0.0".to_string(),
            proxy_port: 80,
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://admin-api.localhost".to_string(),
        }
    }
}

/// Set test directories
pub fn set_test_dirs(data_dir: PathBuf, config_dir: PathBuf) {
    TEST_DATA_DIR.set(data_dir).unwrap();
    TEST_CONFIG_DIR.set(config_dir).unwrap();
}

/// Get the config directory
pub fn get_config_dir() -> Result<PathBuf, ConfigError> {
    if let Some(dir) = TEST_CONFIG_DIR.get() {
        return Ok(dir.clone());
    }

    let config_dir = dirs::config_dir()
        .ok_or(ConfigError::IoError(
            "Could not determine config directory".to_string(),
        ))?
        .join("binarydrop");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|_| ConfigError::IoError("Failed to create config directory".to_string()))?;
    }

    Ok(config_dir)
}

/// Get the data directory
pub fn get_data_dir() -> Result<PathBuf, ConfigError> {
    if let Some(dir) = TEST_DATA_DIR.get() {
        return Ok(dir.clone());
    }

    let data_dir = dirs::data_dir()
        .ok_or(ConfigError::IoError(
            "Could not determine data directory".to_string(),
        ))?
        .join("binarydrop");

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|_| ConfigError::IoError("Failed to create data directory".to_string()))?;
    }

    Ok(data_dir)
}

/// Get a unique port for a new app
pub async fn get_next_available_port(
    db_pool: &sqlx::Pool<sqlx::Sqlite>,
) -> Result<u16, ConfigError> {
    let start_port = 8000;

    // Get all currently used ports
    let rows = sqlx::query("SELECT max(port) FROM apps")
        .fetch_all(db_pool)
        .await
        .context("Failed to query app ports from database")
        .map_err(|e| ConfigError::PortError(e.to_string()))?;

    let mut used_ports = Vec::with_capacity(rows.len());
    for row in &rows {
        used_ports.push(row.get::<u16, _>("port"));
    }

    // Find first available port
    let mut port = start_port;
    while used_ports.contains(&port) {
        port += 1;
    }

    Ok(port)
}

/// Get the app directory
pub fn get_app_dir(app_name: &str) -> Result<PathBuf, ConfigError> {
    let apps_dir = get_data_dir()?.join("apps");

    if !apps_dir.exists() {
        fs::create_dir_all(&apps_dir)
            .map_err(|_| ConfigError::IoError("Failed to create apps directory".to_string()))?;
    }

    let app_dir = apps_dir.join(app_name);

    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)
            .map_err(|_| ConfigError::IoError("Failed to create app directory".to_string()))?;
    }

    Ok(app_dir)
}

/// Get the app binary path
pub fn get_app_binary_path(app_name: &str) -> Result<PathBuf, ConfigError> {
    let app_dir = get_app_dir(app_name)?.join("app");
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)
            .context(format!(
                "Failed to create app binary directory: {}",
                app_dir.display()
            ))
            .map_err(|_| {
                ConfigError::IoError("Failed to create app binary directory".to_string())
            })?;
    }
    Ok(app_dir)
}

/// Get the app binary path
pub fn get_app_data_dir(app_name: &str) -> Result<PathBuf, ConfigError> {
    let data_dir = get_app_dir(app_name)?.join("data");
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .context(format!(
                "Failed to create data directory: {}",
                data_dir.display()
            ))
            .map_err(|_| ConfigError::IoError("Failed to create app data directory".to_string()))?;
    }
    Ok(data_dir)
}

/// Get the app log file path
pub fn get_app_log_path(app_name: &str) -> Result<PathBuf, ConfigError> {
    Ok(get_app_dir(app_name)?.join(format!("{}.log", app_name)))
}

impl ServerConfig {
    #[tracing::instrument]
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::get_config_path()?;
        tracing::info!("Loading server config from {}", config_path.display());

        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(config_path)
            .map_err(|_| ConfigError::IoError("Failed to read config file".to_string()))?;
        let config: Self =
            toml::from_str(&contents).map_err(|e| ConfigError::TomlError(e.to_string()))?;
        Ok(config)
    }

    #[tracing::instrument]
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::get_config_path()?;
        let contents =
            toml::to_string_pretty(self).map_err(|e| ConfigError::TomlError(e.to_string()))?;
        tracing::info!("Saving server config to {}", config_path.display());
        fs::write(config_path, contents)
            .map_err(|_| ConfigError::IoError("Failed to write config file".to_string()))?;
        Ok(())
    }

    pub fn get_config_path() -> Result<PathBuf, ConfigError> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| ConfigError::IoError("Could not find config directory".to_string()))?;
        path.push("bindrop");
        fs::create_dir_all(&path)
            .map_err(|_| ConfigError::IoError("Failed to create config directory".to_string()))?;
        path.push("server-config.toml");
        Ok(path)
    }
}

impl ClientConfig {
    #[tracing::instrument]
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::get_config_path()?;
        tracing::info!("Loading client config from {}", config_path.display());

        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(config_path)
            .map_err(|_| ConfigError::IoError("Failed to read config file".to_string()))?;
        let config: Self =
            toml::from_str(&contents).map_err(|e| ConfigError::TomlError(e.to_string()))?;
        Ok(config)
    }

    #[tracing::instrument]
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::get_config_path()?;
        let contents =
            toml::to_string_pretty(self).map_err(|e| ConfigError::TomlError(e.to_string()))?;
        tracing::info!("Saving client config to {}", config_path.display());
        fs::write(config_path, contents)
            .map_err(|_| ConfigError::IoError("Failed to write config file".to_string()))?;
        Ok(())
    }

    pub fn get_config_path() -> Result<PathBuf, ConfigError> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        path.push("bindrop");
        fs::create_dir_all(&path)
            .map_err(|_| ConfigError::IoError("Failed to create config directory".to_string()))?;
        path.push("client-config.toml");
        Ok(path)
    }
}
