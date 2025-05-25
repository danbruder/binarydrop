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
            .post(&format!("{}/apps", self.base_url))
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

    pub async fn start_app(&self, app_name: &str) -> Result<()> {
        let response = self.client
            .post(&format!("{}/apps/{}/start", self.base_url, app_name))
            .send()
            .await?;

        if response.status().is_success() {
            println!("App '{}' started successfully", app_name);
            Ok(())
        } else {
            let error = response.text().await?;
            Err(anyhow!("Failed to start app: {}", error))
        }
    }

    pub async fn stop_app(&self, app_name: &str) -> Result<()> {
        let response = self.client
            .post(&format!("{}/apps/{}/stop", self.base_url, app_name))
            .send()
            .await?;

        if response.status().is_success() {
            println!("App '{}' stopped successfully", app_name);
            Ok(())
        } else {
            let error = response.text().await?;
            Err(anyhow!("Failed to stop app: {}", error))
        }
    }

    pub async fn restart_app(&self, app_name: &str) -> Result<()> {
        let response = self.client
            .post(&format!("{}/apps/{}/restart", self.base_url, app_name))
            .send()
            .await?;

        if response.status().is_success() {
            println!("App '{}' restarted successfully", app_name);
            Ok(())
        } else {
            let error = response.text().await?;
            Err(anyhow!("Failed to restart app: {}", error))
        }
    }

    pub async fn delete_app(&self, app_name: &str) -> Result<()> {
        let response = self.client
            .delete(&format!("{}/apps/{}", self.base_url, app_name))
            .send()
            .await?;

        if response.status().is_success() {
            println!("App '{}' deleted successfully", app_name);
            Ok(())
        } else {
            let error = response.text().await?;
            Err(anyhow!("Failed to delete app: {}", error))
        }
    }

    pub async fn deploy_app(&self, app_name: &str, binary_path: &str) -> Result<()> {
        let response = self.client
            .post(&format!("{}/apps/{}/deploy", self.base_url, app_name))
            .json(&serde_json::json!({ "binary_path": binary_path }))
            .send()
            .await?;

        if response.status().is_success() {
            println!("App '{}' deployed successfully", app_name);
            Ok(())
        } else {
            let error = response.text().await?;
            Err(anyhow!("Failed to deploy app: {}", error))
        }
    }

    pub async fn set_env(&self, app_name: &str, key: &str, value: &str, delete: bool) -> Result<()> {
        let response = self.client
            .post(&format!("{}/apps/{}/env", self.base_url, app_name))
            .json(&serde_json::json!({ "key": key, "value": value, "delete": delete }))
            .send()
            .await?;

        if response.status().is_success() {
            println!("Environment variable set for app '{}'", app_name);
            Ok(())
        } else {
            let error = response.text().await?;
            Err(anyhow!("Failed to set environment variable: {}", error))
        }
    }

    pub async fn get_status(&self, app_name: Option<&str>) -> Result<()> {
        let url = match app_name {
            Some(name) => format!("{}/apps/{}", self.base_url, name),
            None => format!("{}/apps", self.base_url),
        };

        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status().is_success() {
            let status = response.text().await?;
            println!("Status: {}", status);
            Ok(())
        } else {
            let error = response.text().await?;
            Err(anyhow!("Failed to get status: {}", error))
        }
    }

    pub async fn get_logs(&self, app_name: &str, lines: usize, follow: bool) -> Result<String> {
        let url = format!("{}/api/apps/{}/logs?lines={}&follow={}", self.base_url, app_name, lines, follow);
        let response = reqwest::get(&url).await?;
        if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Err(anyhow::anyhow!("Failed to fetch logs: {}", response.status()))
        }
    }

    // Add other methods (start_app, stop_app, etc.) similarly
} 