use reqwest::Client;
use anyhow::{anyhow, Result};

pub struct ApiClient {
    base_url: String,
    client: Client,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
        }
    }

    pub async fn create_app(&self, app_name: &str) -> Result<()> {
        let response = self.client
            .post(&format!("{}/____bindrop_api/apps", self.base_url))
            .json(&serde_json::json!({ "name": app_name }))
            .send()
            .await?;

        if response.status().is_success() {
            println!("App '{}' created successfully", app_name);
            Ok(())
        } else {
            let error = response.text().await?;
            Err(anyhow!("Failed to create app: {}", error))
        }
    }

    // Add other methods (start_app, stop_app, etc.) similarly
} 