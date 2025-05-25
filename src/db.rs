use anyhow::{Context, Result};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::PathBuf;
use tracing::{debug, info, instrument};

use crate::models::ProcessHistory;

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

    // Check if database file exists
    if !db_path.exists() {
        // Create an empty file
        std::fs::File::create(&db_path).context("Failed to create database file")?;
        info!("Created new database file at {}", db_path.display());
    }

    // Connect to the database
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

    info!("Database initialized successfully");

    Ok(pool)
}

/// App database operations
pub mod apps {

    use super::*;

    /// Save an app to the database
    #[instrument(skip(pool, app))]
    pub async fn save(pool: &Pool<Sqlite>, app: &App) -> Result<()> {
        // Serialize health check to JSON if present
        let health_check_json = match &app.health_check {
            Some(hc) => Some(serde_json::to_string(hc)?),
            None => None,
        };

        // Serialize environment variables to JSON
        let env_json = serde_json::to_string(&app.environment)?;

        // Update or insert
        let state = app.state.to_string();
        let restart_policy = app.restart_policy.to_string();
        let result = sqlx::query!(
            r#"
            INSERT INTO apps (
                id, name, created_at, updated_at, state, binary_path, binary_hash, 
                port, environment, process_id, host, restart_policy, max_restarts,
                restart_count, last_exit_code, last_exit_time, startup_timeout,
                shutdown_timeout, health_check
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                updated_at = excluded.updated_at,
                state = excluded.state,
                binary_path = excluded.binary_path,
                binary_hash = excluded.binary_hash,
                port = excluded.port,
                environment = excluded.environment,
                process_id = excluded.process_id,
                host = excluded.host,
                restart_policy = excluded.restart_policy,
                max_restarts = excluded.max_restarts,
                restart_count = excluded.restart_count,
                last_exit_code = excluded.last_exit_code,
                last_exit_time = excluded.last_exit_time,
                startup_timeout = excluded.startup_timeout,
                shutdown_timeout = excluded.shutdown_timeout,
                health_check = excluded.health_check
            "#,
            app.id,
            app.name,
            app.created_at,
            app.updated_at,
            state,
            app.binary_path,
            app.binary_hash,
            app.port,
            env_json,
            app.process_id,
            app.host,
            restart_policy,
            app.max_restarts,
            app.restart_count,
            app.last_exit_code,
            app.last_exit_time,
            app.startup_timeout,
            app.shutdown_timeout,
            health_check_json,
        )
        .execute(pool)
        .await?;

        debug!(
            "Saved app '{}' (affected rows: {})",
            app.name,
            result.rows_affected()
        );

        Ok(())
    }

    /// Get an app by name
    #[instrument(skip(pool))]
    pub async fn get_by_name(pool: &Pool<Sqlite>, name: &str) -> Result<Option<App>> {
        let record = sqlx::query!(
            r#"
            SELECT id, name, created_at, updated_at, state, binary_path, binary_hash,
                   port, environment, process_id, host, restart_policy, max_restarts,
                   restart_count, last_exit_code, last_exit_time, startup_timeout,
                   shutdown_timeout, health_check
            FROM apps 
            WHERE name = ?
            "#,
            name
        )
        .fetch_optional(pool)
        .await?;

        match record {
            Some(record) => {
                // Parse environment JSON
                let environment = serde_json::from_str(&record.environment)
                    .context("Failed to parse environment JSON")?;

                // Parse health check JSON if present
                let health_check = match record.health_check {
                    Some(json) => Some(
                        serde_json::from_str(&json).context("Failed to parse health check JSON")?,
                    ),
                    None => None,
                };

                // Parse app state
                let state = match record.state.as_str() {
                    "created" => AppState::Created,
                    "deployed" => AppState::Deployed,
                    "starting" => AppState::Starting,
                    "running" => AppState::Running,
                    "stopping" => AppState::Stopping,
                    "stopped" => AppState::Stopped,
                    "failed" => AppState::Failed,
                    "restarting" => AppState::Restarting,
                    "crashed" => AppState::Crashed,
                    _ => AppState::Created,
                };

                // Parse restart policy
                let restart_policy = match record.restart_policy.as_str() {
                    "always" => crate::models::RestartPolicy::Always,
                    "on-failure" => crate::models::RestartPolicy::OnFailure,
                    "never" => crate::models::RestartPolicy::Never,
                    _ => crate::models::RestartPolicy::OnFailure,
                };

                Ok(Some(App {
                    id: record.id,
                    name: record.name,
                    created_at: record.created_at.parse()?,
                    updated_at: record.updated_at.parse()?,
                    state,
                    binary_path: record.binary_path,
                    binary_hash: record.binary_hash,
                    port: record.port as u16,
                    environment,
                    process_id: record.process_id.map(|id| id as u32),
                    host: record.host,
                    restart_policy,
                    max_restarts: record.max_restarts.map(|m| m as u32),
                    restart_count: record.restart_count as u32,
                    last_exit_code: record.last_exit_code,
                    last_exit_time: record.last_exit_time.map(|t| t.and_utc()),
                    startup_timeout: record.startup_timeout as u32,
                    shutdown_timeout: record.shutdown_timeout as u32,
                    health_check,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get all apps with a specific state
    #[instrument(skip(pool))]
    pub async fn get_by_state(pool: &Pool<Sqlite>, state: AppState) -> Result<Vec<App>> {
        let state_str = state.to_string();

        let records = sqlx::query!(
            r#"
            SELECT id, name, created_at, updated_at, state, binary_path, binary_hash,
                   port, environment, process_id, host, restart_policy, max_restarts,
                   restart_count, last_exit_code, last_exit_time, startup_timeout,
                   shutdown_timeout, health_check
            FROM apps 
            WHERE state = ?
            "#,
            state_str
        )
        .fetch_all(pool)
        .await?;

        let mut apps = Vec::new();

        for record in records {
            // Parse environment JSON
            let environment = serde_json::from_str(&record.environment)
                .context("Failed to parse environment JSON")?;

            // Parse health check JSON if present
            let health_check = match record.health_check {
                Some(json) => {
                    Some(serde_json::from_str(&json).context("Failed to parse health check JSON")?)
                }
                None => None,
            };

            // Parse restart policy
            let restart_policy = match record.restart_policy.as_str() {
                "always" => crate::models::RestartPolicy::Always,
                "on-failure" => crate::models::RestartPolicy::OnFailure,
                "never" => crate::models::RestartPolicy::Never,
                _ => crate::models::RestartPolicy::OnFailure,
            };

            apps.push(App {
                id: record.id,
                name: record.name,
                created_at: record.created_at.parse()?,
                updated_at: record.updated_at.parse()?,
                state,
                binary_path: record.binary_path,
                binary_hash: record.binary_hash,
                port: record.port as u16,
                environment,
                process_id: record.process_id.map(|id| id as u32),
                host: record.host,
                restart_policy,
                max_restarts: record.max_restarts.map(|m| m as u32),
                restart_count: record.restart_count as u32,
                last_exit_code: record.last_exit_code,
                last_exit_time: record.last_exit_time.map(|t| t.and_utc()),
                startup_timeout: record.startup_timeout as u32,
                shutdown_timeout: record.shutdown_timeout as u32,
                health_check,
            });
        }

        Ok(apps)
    }

    // Delete
    #[instrument(skip(pool))]
    pub async fn delete_by_app_id(pool: &Pool<Sqlite>, id: &str) -> Result<()> {
        let _ = sqlx::query!(
            r#"
            DELETE FROM process_history
            WHERE app_id = ?;

            DELETE FROM apps
            WHERE id = ?;
            "#,
            id,
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get all apps
    #[instrument(skip(pool))]
    pub async fn get_all(pool: &Pool<Sqlite>) -> Result<Vec<App>> {
        let records = sqlx::query!(
            r#"
            SELECT id, name, created_at, updated_at, state, binary_path, binary_hash,
                   port, environment, process_id, host, restart_policy, max_restarts,
                   restart_count, last_exit_code, last_exit_time, startup_timeout,
                   shutdown_timeout, health_check
            FROM apps 
            ORDER BY name
            "#
        )
        .fetch_all(pool)
        .await?;

        let mut apps = Vec::new();

        for record in records {
            // Parse environment JSON
            let environment = serde_json::from_str(&record.environment)
                .context("Failed to parse environment JSON")?;

            // Parse health check JSON if present
            let health_check = match record.health_check {
                Some(json) => {
                    Some(serde_json::from_str(&json).context("Failed to parse health check JSON")?)
                }
                None => None,
            };

            // Parse app state
            let state = match record.state.as_str() {
                "created" => AppState::Created,
                "deployed" => AppState::Deployed,
                "starting" => AppState::Starting,
                "running" => AppState::Running,
                "stopping" => AppState::Stopping,
                "stopped" => AppState::Stopped,
                "failed" => AppState::Failed,
                "restarting" => AppState::Restarting,
                "crashed" => AppState::Crashed,
                _ => AppState::Created,
            };

            // Parse restart policy
            let restart_policy = match record.restart_policy.as_str() {
                "always" => crate::models::RestartPolicy::Always,
                "on-failure" => crate::models::RestartPolicy::OnFailure,
                "never" => crate::models::RestartPolicy::Never,
                _ => crate::models::RestartPolicy::OnFailure,
            };

            apps.push(App {
                id: record.id,
                name: record.name,
                created_at: record.created_at.parse()?,
                updated_at: record.updated_at.parse()?,
                state,
                binary_path: record.binary_path,
                binary_hash: record.binary_hash,
                port: record.port as u16,
                environment,
                process_id: record.process_id.map(|id| id as u32),
                host: record.host,
                restart_policy,
                max_restarts: record.max_restarts.map(|m| m as u32),
                restart_count: record.restart_count as u32,
                last_exit_code: record.last_exit_code,
                last_exit_time: record.last_exit_time.map(|t| t.and_utc()),
                startup_timeout: record.startup_timeout as u32,
                shutdown_timeout: record.shutdown_timeout as u32,
                health_check,
            });
        }

        Ok(apps)
    }
}

/// Process history repository
pub mod process_history {
    use super::*;

    /// Save a process history entry
    #[instrument(skip(pool, history))]
    pub async fn save(pool: &Pool<Sqlite>, history: &ProcessHistory) -> Result<()> {
        if history.ended_at.is_none() {
            // Insert new entry
            sqlx::query!(
                r#"
                INSERT INTO process_history (
                    id, app_id, started_at, ended_at, exit_code, exit_reason
                ) VALUES (?, ?, ?, ?, ?, ?)
                "#,
                history.id,
                history.app_id,
                history.started_at,
                history.ended_at,
                history.exit_code,
                history.exit_reason
            )
            .execute(pool)
            .await
            .context("Failed to insert process history")?;
        } else {
            // Update existing entry
            sqlx::query!(
                r#"
                UPDATE process_history 
                SET ended_at = ?, exit_code = ?, exit_reason = ?
                WHERE id = ?
                "#,
                history.ended_at,
                history.exit_code,
                history.exit_reason,
                history.id
            )
            .execute(pool)
            .await
            .context("Failed to update process history")?;
        }

        Ok(())
    }

    /// Get process history for an app
    #[instrument(skip(pool))]
    pub async fn get_by_app_id(pool: &Pool<Sqlite>, app_id: &str) -> Result<Vec<ProcessHistory>> {
        let records = sqlx::query!(
            r#"
            SELECT id, app_id, started_at, ended_at, exit_code, exit_reason
            FROM process_history
            WHERE app_id = ?
            ORDER BY started_at DESC
            "#,
            app_id
        )
        .fetch_all(pool)
        .await
        .context("Failed to get process history")?;

        let mut history_entries = Vec::new();

        for record in records {
            history_entries.push(ProcessHistory {
                id: record.id,
                app_id: record.app_id,
                started_at: record.started_at.and_utc(),
                ended_at: record.ended_at.map(|dt| dt.and_utc()),
                exit_code: record.exit_code,
                exit_reason: record.exit_reason,
            });
        }

        Ok(history_entries)
    }

    /// Get recent process history entries
    #[instrument(skip(pool))]
    pub async fn get_recent(pool: &Pool<Sqlite>, limit: i64) -> Result<Vec<ProcessHistory>> {
        let records = sqlx::query!(
            r#"
            SELECT id, app_id, started_at, ended_at, exit_code, exit_reason
            FROM process_history
            ORDER BY started_at DESC
            LIMIT ?
            "#,
            limit
        )
        .fetch_all(pool)
        .await
        .context("Failed to get recent process history")?;

        let mut history_entries = Vec::new();

        for record in records {
            history_entries.push(ProcessHistory {
                id: record.id,
                app_id: record.app_id,
                started_at: record.started_at.and_utc(),
                ended_at: record.ended_at.map(|dt| dt.and_utc()),
                exit_code: record.exit_code,
                exit_reason: record.exit_reason,
            });
        }

        Ok(history_entries)
    }

    /// Get a process history entry by ID
    #[instrument(skip(pool))]
    pub async fn get_by_id(pool: &Pool<Sqlite>, id: &str) -> Result<Option<ProcessHistory>> {
        let record = sqlx::query!(
            r#"
            SELECT id, app_id, started_at, ended_at, exit_code, exit_reason
            FROM process_history
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await
        .context("Failed to get process history by ID")?;

        match record {
            Some(record) => Ok(Some(ProcessHistory {
                id: record.id,
                app_id: record.app_id,
                started_at: record.started_at.and_utc(),
                ended_at: record.ended_at.map(|dt| dt.and_utc()),
                exit_code: record.exit_code,
                exit_reason: record.exit_reason,
            })),
            None => Ok(None),
        }
    }

    /// Delete process history for an app
    #[instrument(skip(pool))]
    pub async fn delete_by_app_id(pool: &Pool<Sqlite>, app_id: &str) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM process_history
            WHERE app_id = ?
            "#,
            app_id
        )
        .execute(pool)
        .await
        .context("Failed to delete process history")?;

        Ok(result.rows_affected())
    }
}
