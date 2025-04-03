use anyhow::{Context, Result};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::PathBuf;
use tracing::{info, instrument};

use crate::config;
use crate::models::{App, AppState};

/// Get the database file path
pub fn get_db_path() -> Result<PathBuf> {
    let config_dir = config::get_config_dir()?;
    let db_path = config_dir.join("binarydrop.db");
    Ok(db_path)
}

/// Initialize the database connection pool
#[instrument(skip_all)]
pub async fn init_pool() -> Result<Pool<Sqlite>> {
    let db_path = get_db_path()?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create config directory")?;
    }

    let db_url = format!("sqlite:{}", db_path.display());
    info!("Connecting to database at {}", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .context("Failed to connect to SQLite database")?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;

    Ok(pool)
}

/// App database operations
pub mod apps {
    use super::*;
    use sqlx::Row;
    use std::collections::HashMap;
    use tracing::instrument;

    /// Save an app to the database
    #[instrument(skip(pool))]
    pub async fn save(pool: &Pool<Sqlite>, app: &App) -> Result<()> {
        let env_json = serde_json::to_string(&app.environment)?;

        sqlx::query(
            r#"
            INSERT INTO apps (id, name, created_at, updated_at, state, binary_path, binary_hash, port, environment, process_id, host)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                updated_at = excluded.updated_at,
                state = excluded.state,
                binary_path = excluded.binary_path,
                binary_hash = excluded.binary_hash,
                port = excluded.port,
                environment = excluded.environment,
                process_id = excluded.process_id,
                host = excluded.host
            "#,
        )
        .bind(&app.id)
        .bind(&app.name)
        .bind(app.created_at)
        .bind(app.updated_at)
        .bind(app.state.to_string())
        .bind(&app.binary_path)
        .bind(&app.binary_hash)
        .bind(app.port)
        .bind(env_json)
        .bind(app.process_id.map(|pid| pid as i64))
        .bind(&app.host)
        .execute(pool)
        .await
        .context("Failed to save app to database")?;

        Ok(())
    }

    /// Get an app by name
    #[instrument(skip(pool))]
    pub async fn get_by_name(pool: &Pool<Sqlite>, name: &str) -> Result<Option<App>> {
        let row = sqlx::query("SELECT * FROM apps WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await
            .context("Failed to query app from database")?;

        match row {
            Some(row) => {
                let env_json: String = row.get("environment");
                let environment: HashMap<String, String> = serde_json::from_str(&env_json)?;

                let state_str: String = row.get("state");
                let state = match state_str.as_str() {
                    "created" => AppState::Created,
                    "deployed" => AppState::Deployed,
                    "starting" => AppState::Starting,
                    "running" => AppState::Running,
                    "stopping" => AppState::Stopping,
                    "stopped" => AppState::Stopped,
                    "failed" => AppState::Failed,
                    _ => AppState::Created,
                };

                let process_id: Option<i64> = row.get("process_id");

                Ok(Some(App {
                    id: row.get("id"),
                    name: row.get("name"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    state,
                    binary_path: row.get("binary_path"),
                    binary_hash: row.get("binary_hash"),
                    port: row.get("port"),
                    environment,
                    process_id: process_id.map(|pid| pid as u32),
                    host: row.get("host"),
                }))
            }
            None => Ok(None),
        }
    }

    /// List all apps
    #[instrument(skip(pool))]
    pub async fn list_all(pool: &Pool<Sqlite>) -> Result<Vec<App>> {
        let rows = sqlx::query("SELECT * FROM apps ORDER BY name")
            .fetch_all(pool)
            .await
            .context("Failed to query apps from database")?;

        let mut apps = Vec::with_capacity(rows.len());

        for row in rows {
            let env_json: String = row.get("environment");
            let environment: HashMap<String, String> = serde_json::from_str(&env_json)?;

            let state_str: String = row.get("state");
            let state = match state_str.as_str() {
                "created" => AppState::Created,
                "deployed" => AppState::Deployed,
                "starting" => AppState::Starting,
                "running" => AppState::Running,
                "stopping" => AppState::Stopping,
                "stopped" => AppState::Stopped,
                "failed" => AppState::Failed,
                _ => AppState::Created,
            };

            let process_id: Option<i64> = row.get("process_id");

            apps.push(App {
                id: row.get("id"),
                name: row.get("name"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                state,
                binary_path: row.get("binary_path"),
                binary_hash: row.get("binary_hash"),
                port: row.get("port"),
                environment,
                process_id: process_id.map(|pid| pid as u32),
                host: row.get("host"),
            });
        }

        Ok(apps)
    }

    /// Delete an app by name
    #[instrument(skip(pool))]
    pub async fn delete_by_name(pool: &Pool<Sqlite>, name: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM apps WHERE name = ?")
            .bind(name)
            .execute(pool)
            .await
            .context("Failed to delete app from database")?;

        Ok(result.rows_affected() > 0)
    }
}
