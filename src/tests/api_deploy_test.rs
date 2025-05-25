use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::api_client::ApiClient;
use crate::db;
use crate::tests::test_utils::TestServer;

async fn setup_test_app(server: &TestServer) -> Result<(ApiClient, String)> {
    // Create a test app name
    let app_name = format!("test-app-{}", uuid::Uuid::new_v4());
    
    // Initialize the API client with the test server's URL
    let api_client = ApiClient::new(server.base_url());
    
    // Create the app
    api_client.create_app(&app_name).await?;
    
    Ok((api_client, app_name))
}

async fn create_test_binary() -> Result<(tempfile::TempDir, PathBuf)> {
    // Create a temporary directory for the test binary
    let temp_dir = tempfile::tempdir()?;
    let binary_path = temp_dir.path().join("test_binary");
    
    // Create a simple binary file (just some bytes)
    let mut file = File::create(&binary_path).await?;
    file.write_all(b"test binary content").await?;
    
    // Make it executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms)?;
    }
    
    Ok((temp_dir, binary_path))
}

#[tokio::test]
async fn test_binary_deployment() -> Result<()> {
    let server = TestServer::new().await?;
    let (api_client, app_name) = setup_test_app(&server).await?;
    let (temp_dir, binary_path) = create_test_binary().await?;
    // temp_dir is kept alive for the duration of the test

    // Deploy the binary
    api_client.deploy_app(&app_name, binary_path.to_str().unwrap()).await?;

    // Fetch the app and verify its state
    let app = api_client.get_app_info(&app_name).await?;
    assert_eq!(app.state, "deployed");

    // Clean up
    api_client.delete_app(&app_name).await?;
    drop(temp_dir); // Explicitly drop temp_dir at the end
    Ok(())
} 