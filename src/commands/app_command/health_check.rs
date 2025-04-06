use anyhow::{anyhow, Result}

/// Remove a health check from an app
#[instrument]
pub async fn remove_health_check(app_name: &str) -> Result<()> {
    // Get database connection
    let pool = db::init_pool().await?;
    
    // Get app
    let mut app = db::apps::get_by_name(&pool, app_name)
        .await?
        .ok_or_else(|| anyhow!("App '{}' not found", app_name))?;
        
    // Remove health check
    app.health_check = None;
    app.updated_at = chrono::Utc::now();
    
    // Save app
    db::apps::save(&pool, &app).await?;
    
    println!("Removed health check from app '{}'", app_name);
    
    Ok(())
}

/// Show health check configuration for an app
#[instrument]
pub async fn show_health_check(app_name: &str) -> Result<()> {
    // Get database connection
    let pool = db::init_pool().await?;
    
    // Get app
    let app = db::apps::get_by_name(&pool, app_name)
        .await?
        .ok_or_else(|| anyhow!("App '{}' not found", app_name))?;
        
    // Show health check
    match &app.health_check {
        Some(hc) => {
            println!("Health check for app '{}':", app_name);
            
            // Show check type
            match &hc.check_type {
                HealthCheckType::HttpGet { path, expected_status } => {
                    println!("  Type: HTTP");
                    println!("  Path: {}", path);
                    println!("  Expected status: {}", expected_status);
                },
                HealthCheckType::TcpPort => {
                    println!("  Type: TCP Port");
                },
                HealthCheckType::Command { cmd, args, success_exit_code } => {
                    println!("  Type: Command");
                    println!("  Command: {}", cmd);
                    if !args.is_empty() {
                        println!("  Args: {:?}", args);
                    }
                    println!("  Success exit code: {}", success_exit_code);
                },
            }
            
            // Show common settings
            println!("  Interval: {} seconds", hc.interval);
            println!("  Timeout: {} seconds", hc.timeout);
            println!("  Retries: {}", hc.retries);
            println!("  Start period: {} seconds", hc.start_period);
        },
        None => {
            println!("No health check configured for app '{}'", app_name);
        },
    }
    
    Ok(())
}

/// Run health check for an app
#[instrument]
pub async fn run_health_check(app_name: &str) -> Result<()> {
    // Get supervisor
    let supervisor = SUPERVISOR.get()
        .ok_or_else(|| anyhow!("Process supervisor not initialized"))?;
        
    // Run health check
    match supervisor.check_app_health(app_name).await {
        Ok(_) => {
            println!("Health check passed for app '{}'", app_name);
        },
        Err(e) => {
            println!("Health check failed for app '{}': {}", app_name, e);
            return Err(e);
        },
    }
    
    Ok(())
};
use serde_json::json;
use tracing::instrument;

use crate::db;
use crate::models::{HealthCheck, HealthCheckType};
use crate::supervisor::SUPERVISOR;

/// Handle the health check commands
#[instrument(skip(cmd))]
pub async fn handle_health_check(cmd: &crate::cli::HealthCheckCommands) -> Result<()> {
    match cmd {
        crate::cli::HealthCheckCommands::Add(args) => {
            add_health_check(args).await
        },
        crate::cli::HealthCheckCommands::Remove { app_name } => {
            remove_health_check(app_name).await
        },
        crate::cli::HealthCheckCommands::Show { app_name } => {
            show_health_check(app_name).await
        },
        crate::cli::HealthCheckCommands::Run { app_name } => {
            run_health_check(app_name).await
        },
    }
}

/// Add a health check to an app
#[instrument(skip(args))]
pub async fn add_health_check(args: &crate::cli::HealthCheckArgs) -> Result<()> {
    // Get database connection
    let pool = db::init_pool().await?;
    
    // Get app
    let mut app = db::apps::get_by_name(&pool, &args.app_name)
        .await?
        .ok_or_else(|| anyhow!("App '{}' not found", args.app_name))?;
        
    // Create health check based on type
    let check_type = match args.type_.as_str() {
        "http" => HealthCheckType::HttpGet {
            path: args.path.clone(),
            expected_status: args.status,
        },
        "tcp" => HealthCheckType::TcpPort,
        "command" => {
            let cmd = args.command.clone()
                .ok_or_else(|| anyhow!("Command must be specified for command health check"))?;
                
            HealthCheckType::Command {
                cmd,
                args: args.args.clone(),
                success_exit_code: args.exit_code,
            }
        },
        _ => return Err(anyhow!("Invalid health check type: {}", args.type_)),
    };
    
    // Create health check
    let health_check = HealthCheck {
        check_type,
        interval: args.interval,
        timeout: args.timeout,
        retries: args.retries,
        start_period: args.start_period,
    };
    
    // Update app
    app.health_check = Some(health_check);
    app.updated_at = chrono::Utc::now();
    
    // Save app to database
    db::apps::save(&pool, &app).await?;
    
    println!("Added health check to app '{}'", args.app_name);
    
    Ok(())
}
