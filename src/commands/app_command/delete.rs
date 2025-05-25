use anyhow::{anyhow, Context, Result};
use tracing::{info, instrument};

use crate::config;
use crate::db;

/// Create a new app
#[instrument]
pub async fn execute(app_name: &str) -> Result<()> {
    // Validate app name
    if !is_valid_app_name(app_name) {
        return Err(anyhow!("Invalid app name. App names must be lowercase alphanumeric with optional hyphens or underscores."));
    }

    // Connect to database
    let pool = db::init_pool().await?;

    let app = db::apps::get_by_name(&pool, app_name).await?;

    if app.is_none() {
        return Err(anyhow!("App '{}' does not exist", app_name));
    }
    let app = app.unwrap();

    // Check if app is running
    if app.is_running() {
        return Err(anyhow!(
            "App '{}' is currently running. Please stop it before deleting.",
            app_name
        ));
    }

    // Create app directory
    let app_dir = config::get_app_dir(app_name)?;
    std::fs::remove_dir_all(&app_dir).context(format!(
        "Failed to remove app directory: {}",
        app_dir.display()
    ))?;

    // Save app to database
    db::apps::delete_by_app_id(&pool, &app.id).await?;

    info!("Deleted app '{}'", &app.name);
    println!("App '{}' deleted successfully", app.name);

    Ok(())
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
