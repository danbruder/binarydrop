use anyhow::{anyhow, Context, Result};
use tracing::{info, instrument};

use crate::config;
use crate::db;
use crate::models::App;

/// Create a new app
#[instrument]
pub async fn execute(app_name: &str, pool: &sqlx::Pool<sqlx::Sqlite>) -> Result<()> {
    // Validate app name
    if !is_valid_app_name(app_name) {
        return Err(anyhow!("Invalid app name. App names must be lowercase alphanumeric with optional hyphens or underscores."));
    }

    // Check if app already exists
    if let Some(_) = db::apps::get_by_name(pool, app_name).await? {
        return Err(anyhow!("App '{}' already exists", app_name));
    }

    // Get next available port
    let port = config::get_next_available_port(pool).await?;

    // Create app
    let app = App::new(app_name, port);

    // Create app directory
    let app_dir = config::get_app_dir(app_name)?;
    std::fs::create_dir_all(&app_dir).context(format!(
        "Failed to create app directory: {}",
        app_dir.display()
    ))?;

    // Save app to database
    db::apps::save(pool, &app).await?;

    info!("Created app '{}' on port {}", app_name, port);
    println!("Successfully created app '{}'", app_name);
    println!(
        "App will be available at http://localhost:{} once deployed and started",
        port
    );

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
