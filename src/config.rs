use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub apps: AppsConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub proxy_host: String,
    pub proxy_port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppsConfig {
    pub data_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub binaries_dir: PathBuf,
    pub port_range_start: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                proxy_host: "127.0.0.1".to_string(),
                proxy_port: 8080,
            },
            apps: AppsConfig {
                data_dir: get_data_dir().unwrap_or_else(|_| PathBuf::from("./data")),
                logs_dir: get_logs_dir().unwrap_or_else(|_| PathBuf::from("./logs")),
                binaries_dir: get_binaries_dir().unwrap_or_else(|_| PathBuf::from("./binaries")),
                port_range_start: 8000,
            },
        }
    }
}

/// Get the config directory
pub fn get_config_dir() -> Result<PathBuf> {
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
    let data_dir = dirs::data_dir()
        .context("Could not determine data directory")?
        .join("binarydrop");

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).context("Failed to create data directory")?;
    }

    Ok(data_dir)
}

/// Get the logs directory
pub fn get_logs_dir() -> Result<PathBuf> {
    let logs_dir = get_data_dir()?.join("logs");

    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir).context("Failed to create logs directory")?;
    }

    Ok(logs_dir)
}

/// Get the binaries directory
pub fn get_binaries_dir() -> Result<PathBuf> {
    let binaries_dir = get_data_dir()?.join("binaries");

    if !binaries_dir.exists() {
        fs::create_dir_all(&binaries_dir).context("Failed to create binaries directory")?;
    }

    Ok(binaries_dir)
}

/// Get the config file path
pub fn get_config_file_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.toml"))
}

/// Load the config from the config file
pub fn load() -> Result<Config> {
    let config_path = get_config_file_path()?;

    if config_path.exists() {
        let config_str = fs::read_to_string(&config_path).context("Failed to read config file")?;

        let config: Config = toml::from_str(&config_str).context("Failed to parse config file")?;

        Ok(config)
    } else {
        // Create default config
        let config = Config::default();
        save(&config)?;

        Ok(config)
    }
}

/// Save the config to the config file
pub fn save(config: &Config) -> Result<()> {
    let config_path = get_config_file_path()?;

    let config_str = toml::to_string_pretty(config).context("Failed to serialize config")?;

    fs::write(&config_path, config_str).context("Failed to write config file")?;

    Ok(())
}

/// Get a unique port for a new app
pub async fn get_next_available_port(db_pool: &sqlx::Pool<sqlx::Sqlite>) -> Result<u16> {
    let config = load()?;
    let start_port = config.apps.port_range_start;

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

/// Get the app log file path
pub fn get_app_log_path(app_name: &str) -> Result<PathBuf> {
    let logs_dir = get_logs_dir()?;
    Ok(logs_dir.join(format!("{}.log", app_name)))
}
