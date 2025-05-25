use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{
    app_command::{app_env, create, delete, deploy, logs, start, status, stop},
    server_command::serve,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new app
    Create {
        /// Name of the app
        app_name: String,
    },

    /// Delete an app
    Delete {
        /// Name of the app
        app_name: String,
    },

    /// Deploy a binary to an app
    Deploy {
        /// Name of the app
        app_name: String,

        /// Path to the binary file
        binary_path: String,
    },

    /// Deploy a binary to an app
    Env {
        /// Name of the app
        app_name: String,

        /// Environment variable key
        key: String,

        /// Environment variable value
        value: String,

        #[arg(long)]
        delete: bool,
    },

    /// Start an app
    Start {
        /// Name of the app
        app_name: String,
    },

    /// Stop an app
    Stop {
        /// Name of the app
        app_name: String,
    },

    /// Restart an app
    Restart {
        /// Name of the app
        app_name: String,
    },

    /// Show app status
    Status {
        /// Name of the app (optional, shows all apps if not specified)
        app_name: Option<String>,
    },

    /// View app logs
    Logs {
        /// Name of the app
        app_name: String,

        /// Number of lines to show
        #[arg(short, long, default_value = "50")]
        lines: usize,

        /// Follow logs in real time
        #[arg(short, long)]
        follow: bool,
    },

    /// Start the BinaryDrop server
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value = "80")]
        port: u16,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { app_name } => create::execute(&app_name).await,
        Commands::Delete { app_name } => delete::execute(&app_name).await,
        Commands::Deploy {
            app_name,
            binary_path,
        } => deploy::execute(&app_name, &binary_path).await,
        Commands::Env {
            app_name,
            key,
            value,
            delete,
        } => app_env::set_env(&app_name, &key, &value, delete).await,
        Commands::Start { app_name } => {
            // Post to an endpoint
            let client = reqwest::Client::new();
            let url = format!(
                "http://localhost:80/____bindrop_api/apps/{}/start",
                app_name
            );
            let response = client
                .post(&url)
                .header("Content-Type", "application/json")
                .send()
                .await?;
            if response.status().is_success() {
                println!("App {} started successfully", app_name);
            } else {
                println!("Failed to start app {}: {}", app_name, response.status());
            }
            Ok(())
        }
        Commands::Restart { app_name } => {
            // Post to an endpoint
            let client = reqwest::Client::new();
            let url = format!(
                "http://localhost:80/____bindrop_api/apps/{}/restart",
                app_name
            );
            let response = client
                .post(&url)
                .header("Content-Type", "application/json")
                .send()
                .await?;
            if response.status().is_success() {
                println!("App {} restarted successfully", app_name);
            } else {
                println!("Failed to start app {}: {}", app_name, response.status());
            }
            Ok(())
        }
        Commands::Stop { app_name } => {
            // Post to an endpoint
            let client = reqwest::Client::new();
            let url = format!("http://localhost:80/____bindrop_api/apps/{}/stop", app_name);
            let response = client
                .post(&url)
                .header("Content-Type", "application/json")
                .send()
                .await?;
            if response.status().is_success() {
                println!("App {} stopped successfully", app_name);
            } else {
                println!("Failed to stopped app {}: {}", app_name, response.status());
            }
            Ok(())
        }
        Commands::Status { app_name } => status::execute(app_name.as_deref()).await,
        Commands::Logs {
            app_name,
            lines,
            follow,
        } => logs::execute(&app_name, lines, follow).await,
        Commands::Serve { host, port } => serve::execute(&host, port).await,
    }
}
