use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite};
use std::fs;
use tracing::{info, instrument};

use crate::config;
use crate::db;

#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    #[error("App not found: {0}")]
    AppNotFound(String),
    #[error("Failed to copy binary: {0}")]
    CopyError(String),
    #[error("Failed to set permissions: {0}")]
    PermissionError(String),
    #[error("ConfigError error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
    #[error("DatabaseError: {0}")]
    DatabaseError(#[from] crate::db::DatabaseError),
}

type Result<T> = anyhow::Result<T, DeployError>;

/// Deploy a binary to an app
#[instrument(skip(pool, binary_data))]
pub async fn execute(pool: &Pool<Sqlite>, app_name: &str, binary_data: &[u8]) -> Result<()> {
    info!("Deploying binary to app '{}'", app_name);

    // Get app
    let app = db::apps::get_by_name(pool, app_name)
        .await?
        .ok_or_else(|| DeployError::AppNotFound(app_name.to_string()))?;

    let hash = hash_binary(binary_data);

    if !app.is_hash_changed(&hash) {
        info!("Binary is identical to the currently deployed version.");
        return Ok(());
    }

    let target_path = config::get_app_binary_path(app_name)?
        .to_string_lossy()
        .to_string();
    info!("Target path for deployment: {}", target_path);
    copy_and_set_permissions(&target_path, binary_data)?;

    // Update and save to database
    let app = app.deployed(target_path, hash);
    db::apps::save(&pool, &app).await?;

    info!("Deployed binary to app '{}'", app_name);

    Ok(())
}

#[instrument(skip(binary_data))]
fn hash_binary(binary_data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(binary_data);
    hex::encode(hasher.finalize())
}

#[instrument(skip(binary_data))]
fn copy_and_set_permissions(target_path: &str, binary_data: &[u8]) -> Result<()> {
    // Save binary to app directory
    fs::write(target_path, binary_data).map_err(|err| DeployError::CopyError(err.to_string()))?;

    // Make binary executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(target_path)
            .map_err(|err| DeployError::PermissionError(err.to_string()))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms)
            .map_err(|err| DeployError::PermissionError(err.to_string()))?;
    }
    Ok(())
}
