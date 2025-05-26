use std::collections::HashMap;
use std::default::Default;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub state: AppState,
    pub binary_path: Option<String>,
    pub binary_hash: Option<String>,
    pub port: u16,
    pub environment: HashMap<String, String>,
    pub process_id: Option<u32>,
    pub host: String,
    // New fields for process management
    pub restart_policy: RestartPolicy,
    pub max_restarts: Option<u32>,
    pub restart_count: u32,
    pub last_exit_code: Option<i64>,
    pub last_exit_time: Option<DateTime<Utc>>,
    pub startup_timeout: u32,  // Seconds
    pub shutdown_timeout: u32, // Seconds
    pub health_check: Option<HealthCheck>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Invalid app name: {0}. App names must be lowercase alphanumeric with optional hyphens or underscores.")]
    InvalidName(String),
    #[error("Invalid port number: {0}")]
    InvalidPort(u16),
}

impl App {
    pub fn new(name: &str, port: u16) -> Result<Self, AppError> {
        let now = Utc::now();

        if !is_valid_app_name(name) {
            return Err(AppError::InvalidName(name.to_string()));
        }

        if port < 1024 {
            return Err(AppError::InvalidPort(port));
        }

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            created_at: now,
            updated_at: now,
            state: AppState::Created,
            binary_path: None,
            binary_hash: None,
            port,
            environment: HashMap::new(),
            process_id: None,
            host: "0.0.0.0".to_string(),
            // New field defaults
            restart_policy: RestartPolicy::OnFailure,
            max_restarts: Some(5),
            restart_count: 0,
            last_exit_code: None,
            last_exit_time: None,
            startup_timeout: 30,
            shutdown_timeout: 10,
            health_check: Some(HealthCheck::default()),
        })
    }

    // Check if app should be restarted based on its policy
    pub fn should_restart(&self) -> bool {
        match self.restart_policy {
            RestartPolicy::Always => true,
            RestartPolicy::Never => false,
            RestartPolicy::OnFailure => {
                // Only restart on non-zero exit codes
                self.last_exit_code.unwrap_or(0) != 0
            }
        }
    }

    // Check if max restarts has been reached
    pub fn reached_max_restarts(&self) -> bool {
        if let Some(max) = self.max_restarts {
            self.restart_count >= max
        } else {
            false
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state, AppState::Running)
    }

    pub fn is_deployed(&self) -> bool {
        matches!(self.state, AppState::Deployed)
    }

    pub fn is_hash_changed(&self, new_hash: &str) -> bool {
        match &self.binary_hash {
            Some(current_hash) => current_hash != new_hash,
            None => true, // If no hash is set, consider it changed
        }
    }

    pub fn deploy(&self, binary_path: String, binary_hash: String) -> Self {
        Self {
            binary_path: Some(binary_path),
            binary_hash: Some(binary_hash),
            updated_at: Utc::now(),
            state: AppState::Deployed,
            ..self.clone()
        }
    }
}

/// Validate app name
fn is_valid_app_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }

    let valid_chars = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');

    valid_chars
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppState {
    Created,
    Deployed,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Restarting,
    Crashed,
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppState::Created => write!(f, "created"),
            AppState::Deployed => write!(f, "deployed"),
            AppState::Starting => write!(f, "starting"),
            AppState::Running => write!(f, "running"),
            AppState::Stopping => write!(f, "stopping"),
            AppState::Stopped => write!(f, "stopped"),
            AppState::Failed => write!(f, "failed"),
            AppState::Restarting => write!(f, "restarting"),
            AppState::Crashed => write!(f, "crashed"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartPolicy {
    Always,
    OnFailure,
    Never,
}

impl std::fmt::Display for RestartPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestartPolicy::Always => write!(f, "always"),
            RestartPolicy::OnFailure => write!(f, "on-failure"),
            RestartPolicy::Never => write!(f, "never"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub check_type: HealthCheckType,
    pub interval: u32, // Seconds
    pub timeout: u32,  // Seconds
    pub retries: u32,
    pub start_period: u32, // Seconds to wait after starting before performing health checks
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckType {
    HttpGet { path: String, expected_status: u16 },
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            check_type: HealthCheckType::HttpGet {
                path: "/".into(),
                expected_status: 200,
            },
            interval: 10,
            timeout: 10,
            retries: 10,
            start_period: 10,
        }
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHistory {
    pub id: String,
    pub app_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i64>,
    pub exit_reason: Option<String>,
}

impl ProcessHistory {
    pub fn new(app_id: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            app_id: app_id.to_string(),
            started_at: Utc::now(),
            ended_at: None,
            exit_code: None,
            exit_reason: None,
        }
    }
}
