pub mod api;
pub mod api_client;
pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod errors;
pub mod models;
pub mod supervisor;
pub mod supervisor2;
// pub mod proxy;
// pub mod utils;

#[cfg(test)]
mod tests;

use crate::models::{App, AppState};

struct AppActor {
    app: App,
}

impl AppActor {
    fn new(app: App) -> Self {
        AppActor { app }
    }

    fn start(&self) {
        // Check if app has been deployed
        let binary_path = match &app.binary_path {
            Some(path) => path,
            None => return Err(anyhow!("App '{}' has not been deployed yet", app.name)),
        };

        info!("Starting app '{}' from binary {}", app.name, binary_path);

        // Create a mutable copy to update
        let mut app = app.clone();

        // Update app state
        app.state = AppState::Starting;
        app.updated_at = Utc::now();
        db::apps::save(db_pool, &app).await?;

        // Get log file path
        let log_path = config::get_app_log_path(&app.name)?;
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .context(format!("Failed to open log file: {}", log_path.display()))?;
        let data_dir = config::get_app_data_dir(&app.name)?;

        // Start process
        let mut cmd = Command::new(binary_path);

        // Add environment variables
        cmd.env("PORT", app.port.to_string());
        cmd.env("APP_NAME", &app.name);
        cmd.env("DATA_DIR", &data_dir);
        for (key, value) in &app.environment {
            cmd.env(key, value);
        }

        // Configure I/O
        cmd.stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file));

        // Start the process
        let child = cmd
            .spawn()
            .context(format!("Failed to start app process: {}", binary_path))?;

        let process_id = child.id();

        // Add to running processes
        {
            let mut process_map = processes.lock().unwrap();
            process_map.insert(
                app.name.clone(),
                RunningProcess {
                    child,
                    started_at: Instant::now(),
                },
            );
        }

        // Record process history
        let history = ProcessHistory {
            id: Uuid::new_v4().to_string(),
            app_id: app.id.clone(),
            started_at: Utc::now(),
            ended_at: None,
            exit_code: None,
            exit_reason: None,
        };
        db::process_history::save(db_pool, &history).await?;

        // Update app with process ID
        app.process_id = Some(process_id);
        app.state = AppState::Running;
        app.updated_at = Utc::now();

        // Save app to database
        db::apps::save(db_pool, &app).await?;

        info!("Started app '{}' with PID {}", app.name, process_id);

        Ok(())
    }

    fn stop(&self) {
        // Logic to stop the app
        println!("Stopping app: {}", self.app.name);
    }

    fn restart(&self) {
        // Logic to restart the app
        println!("Restarting app: {}", self.app.name);
    }

    fn status(&self) -> AppState {
        // Logic to get the status of the app
        AppState::Running // Placeholder for actual status logic
    }
}
