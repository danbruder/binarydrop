use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

static TEST_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static TEST_CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    pub base_url: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            proxy_host: "0.0.0.0".to_string(),
            proxy_port: 80,
            url: "http://admin-api.localhost".to_string(),
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
pub fn get_config_dir() -> Result<PathBuf> {
    if let Some(dir) = TEST_CONFIG_DIR.get() {
        return Ok(dir.clone());
    }

    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("binarydrop");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
    }

    Ok(config_dir)
}

/// Get the data directory
pub fn get_data_dir() -> Result<PathBuf> {
    if let Some(dir) = TEST_DATA_DIR.get() {
        return Ok(dir.clone());
    }

    let data_dir = dirs::data_dir()
        .context("Could not determine data directory")?
        .join("binarydrop");

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).context("Failed to create data directory")?;
    }

    Ok(data_dir)
}

/// Get a unique port for a new app
pub async fn get_next_available_port(db_pool: &sqlx::Pool<sqlx::Sqlite>) -> Result<u16> {
    let start_port = 8000;

    // Get all currently used ports
    let rows = sqlx::query("SELECT port FROM apps")
        .fetch_all(db_pool)
        .await
        .context("Failed to query app ports from database")?;

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
pub fn get_app_dir(app_name: &str) -> Result<PathBuf> {
    let apps_dir = get_data_dir()?.join("apps");

    if !apps_dir.exists() {
        fs::create_dir_all(&apps_dir).context("Failed to create apps directory")?;
    }

    let app_dir = apps_dir.join(app_name);

    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).context("Failed to create app directory")?;
    }

    Ok(app_dir)
}

/// Get the app binary path
pub fn get_app_binary_path(app_name: &str) -> Result<PathBuf> {
    Ok(get_app_dir(app_name)?.join("app"))
}

/// Get the app binary path
pub fn get_app_data_dir(app_name: &str) -> Result<PathBuf> {
    Ok(get_app_dir(app_name)?.join("data"))
}

/// Get the app log file path
pub fn get_app_log_path(app_name: &str) -> Result<PathBuf> {
    Ok(get_app_dir(app_name)?.join(format!("{}.log", app_name)))
}

impl ServerConfig {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(config_path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        let contents = toml::to_string_pretty(self)?;
        fs::write(config_path, contents)?;
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf> {
        let mut path =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        path.push("bindrop");
        fs::create_dir_all(&path)?;
        path.push("server-config.toml");
        Ok(path)
    }
}

impl ClientConfig {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(config_path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        let contents = toml::to_string_pretty(self)?;
        fs::write(config_path, contents)?;
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf> {
        let mut path =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        path.push("bindrop");
        fs::create_dir_all(&path)?;
        path.push("client-config.toml");
        Ok(path)
    }
}
