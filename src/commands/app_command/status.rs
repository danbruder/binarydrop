use anyhow::{anyhow, Result};
use chrono::Utc;
use prettytable::{cell, format, row, Table};
use tracing::instrument;

use crate::db;
use crate::models::AppState;
use crate::supervisor::SUPERVISOR;

/// Get status of all apps or a specific app
#[instrument]
pub async fn execute(app_name: Option<&str>) -> Result<()> {
    // Connect to database
    let pool = db::init_pool().await?;

    // Get supervisor
    let supervisor = SUPERVISOR
        .get()
        .ok_or_else(|| anyhow!("Process supervisor not initialized"))?;

    // Create table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![
        "NAME",
        "STATE",
        "PORT",
        "PID",
        "UPTIME",
        "RESTARTS",
        "LAST EXIT",
        "HOST"
    ]);

    match app_name {
        Some(name) => {
            // Single app status
            let app = db::apps::get_by_name(&pool, name)
                .await?
                .ok_or_else(|| anyhow!("App '{}' not found", name))?;

            // Get stats from supervisor
            let stats = supervisor
                .get_app_stats(name)
                .await?
                .ok_or_else(|| anyhow!("Failed to get app stats"))?;

            // Format uptime
            let uptime = match stats.uptime {
                Some(duration) => {
                    let seconds = duration.as_secs();
                    if seconds < 60 {
                        format!("{}s", seconds)
                    } else if seconds < 3600 {
                        format!("{}m {}s", seconds / 60, seconds % 60)
                    } else {
                        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
                    }
                }
                None => "N/A".to_string(),
            };

            // Format last exit
            let last_exit = match (stats.last_exit_code, stats.last_exit_time) {
                (Some(code), Some(time)) => {
                    let age = Utc::now().signed_duration_since(time);
                    let age_str = if age.num_minutes() < 60 {
                        format!("{}m ago", age.num_minutes())
                    } else if age.num_hours() < 24 {
                        format!("{}h ago", age.num_hours())
                    } else {
                        format!("{}d ago", age.num_days())
                    };

                    format!("{} ({})", code, age_str)
                }
                _ => "N/A".to_string(),
            };

            // Add row to table
            table.add_row(row![
                app.name,
                app.state.to_string(),
                app.port.to_string(),
                stats
                    .pid
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
                uptime,
                stats.restart_count.to_string(),
                last_exit,
                app.host
            ]);

            // Print table
            table.printstd();

            // Print additional details
            println!("\nDetails:");
            println!(
                "  Binary: {}",
                app.binary_path
                    .unwrap_or_else(|| "Not deployed".to_string())
            );
            println!("  Created: {}", app.created_at.format("%Y-%m-%d %H:%M:%S"));
            println!("  Updated: {}", app.updated_at.format("%Y-%m-%d %H:%M:%S"));
            println!("  Restart Policy: {}", app.restart_policy);
            if let Some(max) = app.max_restarts {
                println!("  Max Restarts: {}", max);
            } else {
                println!("  Max Restarts: unlimited");
            }
            println!("  Total Runs: {}", stats.total_runs);

            // Print health check status if configured
            if let Some(health_check) = &app.health_check {
                println!("\nHealth Check:");
                match &health_check.check_type {
                    crate::models::HealthCheckType::HttpGet {
                        path,
                        expected_status,
                    } => {
                        println!("  Type: HTTP");
                        println!("  Path: {}", path);
                        println!("  Expected Status: {}", expected_status);
                    }
                    crate::models::HealthCheckType::TcpPort => {
                        println!("  Type: TCP Port");
                    }
                    crate::models::HealthCheckType::Command {
                        cmd,
                        args,
                        success_exit_code,
                    } => {
                        println!("  Type: Command");
                        println!("  Command: {}", cmd);
                        if !args.is_empty() {
                            println!("  Args: {:?}", args);
                        }
                        println!("  Success Exit Code: {}", success_exit_code);
                    }
                }
                println!("  Interval: {}s", health_check.interval);
                println!("  Timeout: {}s", health_check.timeout);
                println!("  Retries: {}", health_check.retries);
                println!("  Start Period: {}s", health_check.start_period);
            }

            // Print environment variables
            if !app.environment.is_empty() {
                println!("\nEnvironment Variables:");
                for (key, value) in &app.environment {
                    // Mask sensitive values
                    let display_value = if key.to_lowercase().contains("token")
                        || key.to_lowercase().contains("secret")
                        || key.to_lowercase().contains("password")
                        || key.to_lowercase().contains("key")
                    {
                        "********".to_string()
                    } else {
                        value.clone()
                    };
                    println!("  {}={}", key, display_value);
                }
            }
        }
        None => {
            // All apps status
            let apps = db::apps::get_all(&pool).await?;

            for app in apps {
                // Get stats from supervisor
                let stats = match supervisor.get_app_stats(&app.name).await? {
                    Some(stats) => stats,
                    None => continue,
                };

                // Format uptime
                let uptime = match stats.uptime {
                    Some(duration) => {
                        let seconds = duration.as_secs();
                        if seconds < 60 {
                            format!("{}s", seconds)
                        } else if seconds < 3600 {
                            format!("{}m", seconds / 60)
                        } else {
                            format!("{}h", seconds / 3600)
                        }
                    }
                    None => "N/A".to_string(),
                };

                // Format last exit
                let last_exit = match (stats.last_exit_code, stats.last_exit_time) {
                    (Some(code), Some(time)) => {
                        let age = Utc::now().signed_duration_since(time);
                        if age.num_minutes() < 60 {
                            format!("{} ({}m)", code, age.num_minutes())
                        } else if age.num_hours() < 24 {
                            format!("{} ({}h)", code, age.num_hours())
                        } else {
                            format!("{} ({}d)", code, age.num_days())
                        }
                    }
                    _ => "N/A".to_string(),
                };

                // Add row to table
                table.add_row(row![
                    app.name,
                    app.state.to_string(),
                    app.port.to_string(),
                    stats
                        .pid
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "N/A".to_string()),
                    uptime,
                    stats.restart_count.to_string(),
                    last_exit,
                    app.host
                ]);
            }

            if table.is_empty() {
                println!("No apps found");
            } else {
                // Print table
                table.printstd();
                println!("\nTo see detailed information about an app, run:");
                println!("  binarydrop status <app-name>");
            }
        }
    }

    Ok(())
}
