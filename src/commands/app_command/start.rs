use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::config;
use crate::db;
use crate::models::{AppState, ProcessHistory};

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum StartError {
    #[error("App not found: {0}")]
    AppNotFound(String),
    #[error("App already running: {0}")]
    AppAlreadyRunning(String),
    #[error("App not deployed: {0}")]
    AppNotDeployed(String),
    #[error("Failed to start app: {0}")]
    AppStartFailed(String),
    #[error("App directory broken: {0}")]
    AppDirectoryBroken(String),
    #[error("App log broken: {0}")]
    AppLogBroken(String),
    #[error("Failed to start app process: {0}")]
    InternalError(String),
}

/// Start an app using the supervisor
#[instrument]
pub async fn execute(pool: &Pool<Sqlite>, app_name: &str) -> Result<Child, StartError> {
    // Get app
    let app = db::apps::get_by_name(pool, app_name)
        .await
        .map_err(|_| StartError::AppNotFound(app_name.to_string()))?
        .ok_or_else(|| StartError::AppNotFound(app_name.to_string()))?;

    // Check if app is already running
    if app.is_running() {
        println!("App '{}' is already running", app_name);
        return Err(StartError::AppAlreadyRunning(app_name.to_string()));
    }

    // Check if app has been deployed
    if !app.is_deployed() {
        return Err(StartError::AppNotDeployed(app.name.clone()));
    }

    // Check if app has been deployed
    let binary_path = match &app.binary_path {
        Some(path) => path,
        None => {
            return Err(StartError::AppNotDeployed(app.name.clone()));
        }
    };

    info!("Starting app '{}' from binary {}", app.name, binary_path);

    // Create a mutable copy to update
    let mut app = app.clone();

    // Update app state
    app.state = AppState::Starting;
    app.updated_at = Utc::now();
    db::apps::save(pool, &app)
        .await
        .map_err(|e| StartError::InternalError(e.to_string()))?;

    // Get log file path
    let log_path =
        config::get_app_log_path(&app.name).map_err(|e| StartError::AppLogBroken(e.to_string()))?;
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| StartError::AppLogBroken(e.to_string()))?;

    // Get data directory path
    let data_dir = config::get_app_data_dir(&app.name)
        .map_err(|e| StartError::AppDirectoryBroken(e.to_string()))?;

    // Start process
    let mut cmd = Command::new(binary_path);

    // Add environment variables
    cmd.env("PORT", app.port.to_string());
    cmd.env("APP_NAME", &app.name);
    cmd.env("DATA_DIR", &data_dir);
    for (key, value) in &app.environment {
        cmd.env(key, value);
    }

    // Configure I/O
    let log_file_clone = log_file
        .try_clone()
        .map_err(|e| StartError::AppStartFailed(e.to_string()))?;
    cmd.stdout(Stdio::from(log_file_clone))
        .stderr(Stdio::from(log_file));

    // Start the process
    let child = cmd
        .spawn()
        .context(format!("Failed to start app process: {}", binary_path))
        .map_err(|e| StartError::AppStartFailed(e.to_string()))?;

    let process_id = child.id();

    // Record process history
    let history = ProcessHistory {
        id: Uuid::new_v4().to_string(),
        app_id: app.id.clone(),
        started_at: Utc::now(),
        ended_at: None,
        exit_code: None,
        exit_reason: None,
    };
    db::process_history::save(pool, &history)
        .await
        .map_err(|e| StartError::InternalError(e.to_string()))?;

    // Update app with process ID
    app.process_id = Some(process_id);
    app.state = AppState::Running;
    app.updated_at = Utc::now();
    // Save app to database
    db::apps::save(pool, &app)
        .await
        .map_err(|e| StartError::InternalError(e.to_string()))?;

    info!("Started app '{}' with PID {}", app.name, process_id);

    Ok(child)
}

#[cfg(test)]
mod test {
    use crate::db::apps;
    use crate::models::{App, AppState};

    #[tokio::test]
    async fn test_starting_non_existant_app() {
        let pool = crate::db::test::get_test_pool().await;
        let got = super::execute(&pool, "non_existant_app").await.unwrap_err();
        let want = super::StartError::AppNotFound("non_existant_app".to_string());

        assert_eq!(got, want);
    }

    #[tokio::test]
    async fn test_starting_not_deployed_app() {
        let pool = crate::db::test::get_test_pool().await;
        let app_name = "app";
        let app = App::new(app_name, 8080).unwrap();
        apps::save(&pool, &app).await.unwrap();

        let got = super::execute(&pool, app_name).await.unwrap_err();
        let want = super::StartError::AppNotDeployed(app_name.to_string());

        assert_eq!(got, want);
    }

    #[tokio::test]
    async fn test_starting_already_running_app() {
        let pool = crate::db::test::get_test_pool().await;
        let app_name = "app";
        let mut app = App::new(app_name, 8080).unwrap();
        app.state = AppState::Deployed;
        apps::save(&pool, &app).await.unwrap();

        let got = super::execute(&pool, app_name).await.unwrap_err();
        let want = super::StartError::AppAlreadyRunning(app_name.to_string());

        assert_eq!(got, want);
    }
}
