use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
}

impl App {
    pub fn new(name: &str, port: u16) -> Self {
        let now = Utc::now();
        Self {
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
            host: "localhost".to_string(),
        }
    }
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
        }
    }
}
