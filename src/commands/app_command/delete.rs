use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing::{info, instrument};

use crate::config;
use crate::db;

#[derive(Debug, thiserror::Error)]
pub enum DeleteError {
    #[error("App not found: {0}")]
    AppNotFound(String),
    #[error("App is running: {0}")]
    AppRunning(String),
    #[error("Failed to remove app directory: {0}")]
    DirectoryError(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] crate::db::DatabaseError),
    #[error("Config error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
}

type DeleteResult<T> = Result<T, DeleteError>;

/// Delete an app
#[instrument]
pub async fn execute(pool: &Pool<Sqlite>, app_name: &str) -> DeleteResult<()> {
    // Get app
    let app = db::apps::get_by_name(pool, app_name)
        .await?
        .ok_or_else(|| DeleteError::AppNotFound(app_name.to_string()))?;

    // Check if app is running
    if app.is_running() {
        return Err(DeleteError::AppRunning(app_name.to_string()));
    }

    // Remove app directory
    let app_dir = config::get_app_dir(app_name)?;
    std::fs::remove_dir_all(&app_dir).map_err(|e| {
        DeleteError::DirectoryError(format!(
            "Failed to remove app directory {}: {}",
            app_dir.display(),
            e
        ))
    })?;

    // Delete app from database
    db::apps::delete_by_app_id(pool, &app.id).await?;

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

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::test::get_test_pool;
    use crate::models::App;

    #[tokio::test]
    async fn test_deleting_non_existant_app() {
        let pool = get_test_pool().await;
        let got = execute(&pool, "non_existant_app").await.unwrap_err();
        let want = DeleteError::AppNotFound("non_existant_app".to_string());

        assert_eq!(format!("{}", got), format!("{}", want));
    }

    #[tokio::test]
    async fn test_deleting_running_app() {
        let pool = get_test_pool().await;
        let app_name = "test_app";

        // Create and save a running app
        let app = App::new(app_name, 8080).unwrap().running(1234);
        db::apps::save(&pool, &app).await.unwrap();

        // Create app directory
        let app_dir = config::get_app_dir(app_name).unwrap();
        std::fs::create_dir_all(&app_dir).unwrap();

        let got = execute(&pool, app_name).await.unwrap_err();
        let want = DeleteError::AppRunning(app_name.to_string());

        assert_eq!(format!("{}", got), format!("{}", want));

        // Clean up
        let _ = std::fs::remove_dir_all(&app_dir);
    }

    #[tokio::test]
    async fn test_delete_happy_path() {
        let pool = get_test_pool().await;
        let app_name = "test_app";

        // Create and save a stopped app
        let app = App::new(app_name, 8080).unwrap();
        db::apps::save(&pool, &app).await.unwrap();

        // Create app directory
        let app_dir = config::get_app_dir(app_name).unwrap();
        std::fs::create_dir_all(&app_dir).unwrap();

        // Delete the app
        execute(&pool, app_name).await.unwrap();

        // Verify app is deleted from database
        let app = db::apps::get_by_name(&pool, app_name).await.unwrap();
        assert!(app.is_none());

        // Verify app directory is deleted
        assert!(!app_dir.exists());
    }
}
