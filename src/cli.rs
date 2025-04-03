use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{
    app_command::{create, deploy, logs, start, status, stop},
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

    /// Deploy a binary to an app
    Deploy {
        /// Name of the app
        app_name: String,

        /// Path to the binary file
        binary_path: String,
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
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { app_name } => create::execute(&app_name).await,
        Commands::Deploy {
            app_name,
            binary_path,
        } => deploy::execute(&app_name, &binary_path).await,
        Commands::Start { app_name } => start::execute(&app_name).await,
        Commands::Stop { app_name } => stop::execute(&app_name).await,
        Commands::Status { app_name } => status::execute(app_name.as_deref()).await,
        Commands::Logs {
            app_name,
            lines,
            follow,
        } => logs::execute(&app_name, lines, follow).await,
        Commands::Serve { host, port } => serve::execute(&host, port).await,
    }
}
