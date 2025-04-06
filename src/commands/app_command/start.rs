use anyhow::{anyhow, Context, Result};
use tracing::{info, instrument};

use crate::db;
use crate::models::AppState;
use crate::supervisor::SUPERVISOR;

/// Start an app using the supervisor
#[instrument]
pub async fn execute(app_name: &str) -> Result<()> {
    // Connect to database
    let pool = db::init_pool().await?;

    // Get app
    let app = db::apps::get_by_name(&pool, app_name)
        .await?
        .ok_or_else(|| anyhow!("App '{}' not found", app_name))?;

    // Check if app is already running
    if app.state == AppState::Running {
        println!("App '{}' is already running", app_name);
        return Ok(());
    }

    // Check if app has been deployed
    if app.binary_path.is_none() {
        return Err(anyhow!("App '{}' has not been deployed yet", app_name));
    }

    // Get the supervisor from global state
    let supervisor = SUPERVISOR
        .get()
        .ok_or_else(|| anyhow!("Process supervisor not initialized"))?;

    // Start the app through the supervisor
    info!("Sending start request for app '{}'", app_name);
    supervisor.start_app(app_name).await?;

    // Wait a bit for the app to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Fetch the app again to get the updated state
    let app = db::apps::get_by_name(&pool, app_name)
        .await?
        .ok_or_else(|| anyhow!("App '{}' not found", app_name))?;

    if app.state == AppState::Running {
        println!("Successfully started app '{}'", app_name);
        println!("App is now available at http://{}:{}", app.host, app.port);

        // Print restart policy information
        println!("Restart policy: {}", app.restart_policy);
        if let Some(max) = app.max_restarts {
            println!("Maximum restarts: {}", max);
        } else {
            println!("Maximum restarts: unlimited");
        }
    } else {
        println!("App '{}' is starting...", app_name);
        println!("Check status with: binarydrop status {}", app_name);
    }

    Ok(())
}
