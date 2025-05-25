use reqwest::Client;
use anyhow::{anyhow, Result};
use futures_util::stream::BoxStream;
use futures_util::StreamExt;
use bytes::Bytes;

pub enum LogStream {
    Lines(BoxStream<'static, anyhow::Result<String>>),
    Full(String),
}

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
        // Read the binary file
        let binary_data = tokio::fs::read(binary_path).await?;

        // Create multipart form
        let form = reqwest::multipart::Form::new()
            .part("binary", reqwest::multipart::Part::bytes(binary_data));

        let response = self.client
            .post(&format!("{}/api/apps/{}/deploy", self.base_url, app_name))
            .multipart(form)
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

    pub async fn get_logs(&self, app_name: &str, lines: usize, follow: bool) -> Result<LogStream> {
        let url = format!("{}/api/apps/{}/logs?lines={}&follow={}", self.base_url, app_name, lines, follow);
        let response = self.client.get(&url).send().await?;
        if response.status().is_success() {
            if follow {
                let stream = response
                    .bytes_stream()
                    .map(|chunk: Result<Bytes, reqwest::Error>| {
                        chunk
                            .map_err(|e| anyhow::anyhow!(e))
                            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
                    })
                    .boxed();
                Ok(LogStream::Lines(stream))
            } else {
                Ok(LogStream::Full(response.text().await?))
            }
        } else {
            Err(anyhow::anyhow!("Failed to fetch logs: {}", response.status()))
        }
    }

    // Add other methods (start_app, stop_app, etc.) similarly
} 