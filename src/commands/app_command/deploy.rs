use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tracing::{info, instrument};

use crate::config;
use crate::db;
use crate::models::AppState;

/// Deploy a binary to an app
#[instrument(skip(binary_path))]
pub async fn execute(app_name: &str, binary_path: &str) -> Result<()> {
    // Check if binary exists
    let binary_path = PathBuf::from(binary_path);
    if !binary_path.exists() {
        return Err(anyhow!("Binary file not found: {}", binary_path.display()));
    }

    let data_dir = config::get_app_data_dir(app_name)?;
    // Create data directory if it doesn't exist
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).context(format!(
            "Failed to create data directory: {}",
            data_dir.display()
        ))?;
    }
    info!("Created data directory: {}", data_dir.display());

    // Check if binary is executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&binary_path)?;
        let permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            return Err(anyhow!(
                "Binary file is not executable: {}",
                binary_path.display()
            ));
        }
    }

    // Connect to database
    let pool = db::init_pool().await?;

    // Get app
    let mut app = db::apps::get_by_name(&pool, app_name)
        .await?
        .ok_or_else(|| anyhow!("App '{}' not found", app_name))?;

    // Calculate binary hash
    let binary_data = fs::read(&binary_path).context(format!(
        "Failed to read binary file: {}",
        binary_path.display()
    ))?;
    let mut hasher = Sha256::new();
    hasher.update(&binary_data);
    let hash = hex::encode(hasher.finalize());

    // Check if binary is the same
    if let Some(current_hash) = &app.binary_hash {
        if current_hash == &hash {
            println!("Binary is identical to the currently deployed version.");
            return Ok(());
        }
    }

    // Copy binary to app directory
    let target_path = config::get_app_binary_path(app_name)?;
    fs::copy(&binary_path, &target_path).context(format!(
        "Failed to copy binary to app directory: {}",
        target_path.display()
    ))?;

    // Make binary executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms)?;
    }

    // Update app
    app.state = AppState::Deployed;
    app.binary_path = Some(target_path.to_string_lossy().to_string());
    app.binary_hash = Some(hash);
    app.updated_at = Utc::now();

    // Save app to database
    db::apps::save(&pool, &app).await?;

    info!("Deployed binary to app '{}'", app_name);
    println!("Successfully deployed binary to app '{}'", app_name);
    println!(
        "You can now start the app with: binarydrop start {}",
        app_name
    );

    Ok(())
}
