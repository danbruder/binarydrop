use anyhow::{anyhow, Result};
use tracing::{info, instrument};

use crate::db;
use crate::models::AppState;
use crate::supervisor::SUPERVISOR;

/// Stop an app using the supervisor
#[instrument]
pub async fn execute(app_name: &str) -> Result<()> {
    // Connect to database
    let pool = db::init_pool().await?;

    // Get app
    let app = db::apps::get_by_name(&pool, app_name)
        .await?
        .ok_or_else(|| anyhow!("App '{}' not found", app_name))?;

    // Check if app is already running
    if app.state == AppState::Stopped {
        println!("App '{}' is already stopped", app_name);
        return Ok(());
    }

    // Get the supervisor from global state
    let supervisor = SUPERVISOR
        .get()
        .ok_or_else(|| anyhow!("Process supervisor not initialized"))?;

    // Start the app through the supervisor
    info!("Sending stop request for app '{}'", app_name);
    supervisor.stop_app(app_name).await?;

    // Wait a bit for the app to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Fetch the app again to get the updated state
    let app = db::apps::get_by_name(&pool, app_name)
        .await?
        .ok_or_else(|| anyhow!("App '{}' not found", app_name))?;

    if app.state == AppState::Stopped {
        println!("Successfully stopped app '{}'", app_name);
    } else {
        println!("App '{}' is stopping...", app_name);
        println!("Check status with: binarydrop status {}", app_name);
    }

    Ok(())
}
