use anyhow::{anyhow, Result};
use tracing::instrument;

use crate::db;

/// Show app status
#[instrument]
pub async fn execute(app_name: Option<&str>) -> Result<()> {
    // Connect to database
    let pool = db::init_pool().await?;

    match app_name {
        Some(name) => {
            // Show status for a specific app
            let app = db::apps::get_by_name(&pool, name)
                .await?
                .ok_or_else(|| anyhow!("App '{}' not found", name))?;

            println!("App: {}", app.name);
            println!("Status: {}", app.state);
            println!("Port: {:?}", app.port);

            if let Some(pid) = app.process_id {
                println!("Process ID: {}", pid);
            }

            if let Some(path) = &app.binary_path {
                println!("Binary: {}", path);
            }

            if !app.environment.is_empty() {
                println!("Environment Variables:");
                for (key, value) in &app.environment {
                    println!("  {}={}", key, value);
                }
            }
        }
        None => {
            // Show status for all apps
            let apps = db::apps::get_all(&pool).await?;

            if apps.is_empty() {
                println!("No apps found");
                return Ok(());
            }

            println!(
                "{:<20} {:<10} {:<8} {:<10}",
                "NAME", "STATUS", "PORT", "PID"
            );
            println!(
                "{:<20} {:<10} {:<8} {:<10}",
                "----", "------", "----", "---"
            );

            for app in apps {
                let pid = match app.process_id {
                    Some(pid) => pid.to_string(),
                    None => "-".to_string(),
                };

                println!(
                    "{:<20} {:<10} {:<8} {:<10}",
                    app.name,
                    app.state,
                    app.port.map_or("-".to_string(), |p| p.to_string()),
                    pid
                );
            }
        }
    }

    Ok(())
}
