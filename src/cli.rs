use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{
    app_command::{app_env, create, delete, deploy, logs, status},
    server_command::serve,
};
use crate::api_client::ApiClient;

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

fn get_api_base_url() -> String {
    std::env::var("BINDROP_API_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let api_client = ApiClient::new(get_api_base_url());

    match cli.command {
        Commands::Create { app_name } => api_client.create_app(&app_name).await,
        Commands::Start { app_name } => api_client.start_app(&app_name).await,
        Commands::Stop { app_name } => api_client.stop_app(&app_name).await,
        Commands::Restart { app_name } => api_client.restart_app(&app_name).await,
        Commands::Delete { app_name } => api_client.delete_app(&app_name).await,
        Commands::Deploy { app_name, binary_path } => api_client.deploy_app(&app_name, &binary_path).await,
        Commands::Env { app_name, key, value, delete } => api_client.set_env(&app_name, &key, &value, delete).await,
        Commands::Status { app_name } => api_client.get_status(app_name.as_deref()).await,
        Commands::Logs {
            app_name,
            lines,
            follow,
        } => {
            match api_client.get_logs(&app_name, lines, follow).await? {
                crate::api_client::LogStream::Full(logs) => println!("{}", logs),
                crate::api_client::LogStream::Lines(mut stream) => {
                    use futures_util::StreamExt;
                    while let Some(line) = stream.next().await {
                        match line {
                            Ok(l) => print!("{}", l),
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                }
            }
            Ok(())
        },
        Commands::Serve { host, port } => {
            if std::env::var("BINDROP_SERVER_MODE").is_err() {
                return Err(anyhow::anyhow!("The serve command can only be run in server mode."));
            }
            serve::execute(&host, port).await
        },
    }
}
