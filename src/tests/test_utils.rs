use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::api;
use crate::db;
use crate::commands::server_command::serve::ProxyState;
use tempfile::TempDir;
use std::path::PathBuf;
use crate::config;

pub struct TestServer {
    pub addr: SocketAddr,
    pub state: Arc<RwLock<ProxyState>>,
    _temp_dir: TempDir, // Keep the temp directory alive
}

impl TestServer {
    pub async fn new() -> Result<Self> {
        // Create a temporary directory for the test database and data
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        
        // Set up data directories
        let data_dir = temp_dir.path().join("data");
        let config_dir = temp_dir.path().join("config");
        let logs_dir = data_dir.join("logs");
        let binaries_dir = data_dir.join("binaries");
        let apps_dir = data_dir.join("apps");

        // Create directories
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&config_dir)?;
        std::fs::create_dir_all(&logs_dir)?;
        std::fs::create_dir_all(&binaries_dir)?;
        std::fs::create_dir_all(&apps_dir)?;
        
        // Set environment variables
        std::env::set_var("DATABASE_URL", format!("sqlite:{}", db_path.display()));
        std::env::set_var("BINDROP_DATA_DIR", data_dir.to_string_lossy().to_string());
        std::env::set_var("BINDROP_CONFIG_DIR", config_dir.to_string_lossy().to_string());
        
        // Set test directories
        config::set_test_dirs(data_dir, config_dir);
        
        // Initialize database pool
        let pool = db::init_pool().await?;
        
        // Create shared state
        let state = Arc::new(RwLock::new(ProxyState {
            db_pool: pool,
        }));

        // Create API router
        let app = api::create_api_router(Arc::clone(&state));

        // Create server
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let server = axum::Server::bind(&addr)
            .serve(app.into_make_service());

        // Get the actual address the server is bound to
        let addr = server.local_addr();

        // Spawn the server in the background
        tokio::spawn(async move {
            server.await.unwrap();
        });

        Ok(Self { 
            addr, 
            state,
            _temp_dir: temp_dir,
        })
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }
} 