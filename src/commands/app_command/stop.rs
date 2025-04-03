use anyhow::{anyhow, Result};
use chrono::Utc;
use tracing::{info, instrument};

use crate::db;
use crate::models::AppState;

/// Stop an app
#[instrument]
pub async fn execute(app_name: &str) -> Result<()> {
    // Connect to database
    let pool = db::init_pool().await?;

    // Get app
    let mut app = db::apps::get_by_name(&pool, app_name)
        .await?
        .ok_or_else(|| anyhow!("App '{}' not found", app_name))?;

    // Check if app is running
    if app.state != AppState::Running {
        println!("App '{}' is not running", app_name);
        return Ok(());
    }

    // Get process ID
    let pid = match app.process_id {
        Some(pid) => pid,
        None => return Err(anyhow!("App '{}' has no process ID", app_name)),
    };

    // Update app state
    app.state = AppState::Stopping;
    app.updated_at = Utc::now();
    db::apps::save(&pool, &app).await?;

    info!("Stopping app '{}' with PID {}", app_name, pid);

    // Stop process
    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill").arg(pid.to_string()).status()?;
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .status()?;
    }

    // Update app state
    app.state = AppState::Stopped;
    app.process_id = None;
    app.updated_at = Utc::now();
    db::apps::save(&pool, &app).await?;

    info!("Stopped app '{}'", app_name);
    println!("Successfully stopped app '{}'", app_name);

    Ok(())
}
