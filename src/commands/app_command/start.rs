use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::process::{Command, Stdio};
use tracing::{info, instrument};

use crate::config;
use crate::db;
use crate::models::AppState;

/// Start an app
#[instrument]
pub async fn execute(app_name: &str) -> Result<()> {
    // Connect to database
    let pool = db::init_pool().await?;

    // Get app
    let mut app = db::apps::get_by_name(&pool, app_name)
        .await?
        .ok_or_else(|| anyhow!("App '{}' not found", app_name))?;

    // Check if app is already running
    if app.state == AppState::Running {
        println!("App '{}' is already running", app_name);
        return Ok(());
    }

    // Check if app has been deployed
    let binary_path = match &app.binary_path {
        Some(path) => path,
        None => return Err(anyhow!("App '{}' has not been deployed yet", app_name)),
    };

    info!("Starting app '{}' from binary {}", app_name, binary_path);

    // Update app state
    app.state = AppState::Starting;
    app.updated_at = Utc::now();
    db::apps::save(&pool, &app).await?;

    // Get log file path
    let log_path = config::get_app_log_path(app_name)?;
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .context(format!("Failed to open log file: {}", log_path.display()))?;

    // Start process
    let mut cmd = Command::new(binary_path);

    // Add environment variables
    cmd.env("PORT", app.port.to_string());
    for (key, value) in &app.environment {
        cmd.env(key, value);
    }

    // Configure I/O
    cmd.stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file));

    // Start the process
    let child = cmd
        .spawn()
        .context(format!("Failed to start app process: {}", binary_path))?;

    // Update app with process ID
    app.process_id = Some(child.id());
    app.state = AppState::Running;
    app.updated_at = Utc::now();

    // Save app to database
    db::apps::save(&pool, &app).await?;

    info!("Started app '{}' with PID {}", app_name, child.id());
    println!("Successfully started app '{}'", app_name);
    println!("App is now available at http://localhost:{}", app.port);

    Ok(())
}
