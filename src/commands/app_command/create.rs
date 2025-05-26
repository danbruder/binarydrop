use anyhow::Result;
use tracing::info;

use crate::config;
use crate::db;
use crate::models::App;

#[derive(Debug, thiserror::Error)]
pub enum AppCreateError {
    #[error("App already exists: {0}")]
    AppAlreadyExists(String),
    #[error("Failed to get next available port")]
    InvalidPort,
    #[error("Failed to create app: {0}")]
    AppError(#[from] crate::models::AppError),
    #[error("Failed to configure app: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
    #[error("Failed to create app: {0}")]
    DatabaseError(#[from] crate::db::DatabaseError),
    #[error("Internal Error")]
    InternalError,
}

/// Create a new app
pub async fn execute(
    app_name: &str,
    pool: &sqlx::Pool<sqlx::Sqlite>,
) -> Result<(), AppCreateError> {
    // Check if app already exists
    if let Some(_) = db::apps::get_by_name(pool, app_name).await? {
        return Err(AppCreateError::AppAlreadyExists(app_name.to_string()));
    }

    // Get next available port
    let port = config::get_next_available_port(pool).await?;

    // Create app
    let app = App::new(app_name, port)?;

    // Create app directory
    let _ = config::get_app_dir(app_name)?;

    // Save app to database
    db::apps::save(pool, &app).await?;

    info!("Created app '{}' on port {}", app_name, port);

    Ok(())
}
