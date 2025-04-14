use anyhow::{Context, Result};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, instrument};

use crate::commands::app_command::{deploy, start, stop};
use crate::db;
use crate::models::AppState;
use tokio::sync::oneshot;

// Message types for communication between servers
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ApiRequest {
    ListApps,
    GetApp { name: String },
    CreateApp { name: String },
    DeleteApp { id: String },
    StartApp { id: String },
    StopApp { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ApiResponse {
    Apps(Vec<AppInfo>),
    App(Option<AppInfo>),
    Success(String),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppInfo {
    id: String,
    name: String,
    state: String,
    host: String,
    port: u16,
    process_id: Option<u32>,
}

/// Shared state for the proxy server
struct ProxyState {
    db_pool: sqlx::Pool<sqlx::Sqlite>,
}

/// Start the BinaryDrop server
#[instrument]
pub async fn execute(host: &str, port: u16) -> Result<()> {
    // Connect to database
    let pool = db::init_pool().await?;
    let _ = crate::supervisor::init(pool.clone());

    // Create shared state
    let proxy_state = Arc::new(RwLock::new(ProxyState {
        db_pool: pool.clone(),
    }));

    // Parse host and port for proxy server
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .context(format!("Invalid host or port: {}:{}", host, port))?;

    // Create service for proxy server
    let make_svc = make_service_fn(move |_conn| {
        let state = Arc::clone(&proxy_state);
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let state = Arc::clone(&state);
                async move { handle_request(state, req).await }
            }))
        }
    });

    // Create proxy server
    let server = Server::bind(&addr).serve(make_svc);

    info!(
        "Starting BinaryDrop proxy server on http://{}:{}",
        host, port
    );
    println!(
        "BinaryDrop proxy server running at http://{}:{}",
        host, port
    );
    println!("Press Ctrl+C to stop");

    // Run proxy server
    tokio::select! {
        result = server => {
            result.context("Proxy server error")?;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("Shutting down...");
        }
    }

    println!("API server stopped");

    Ok(())
}

/// Handle incoming requests to the proxy server
async fn handle_request(
    state: Arc<RwLock<ProxyState>>,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    // Extract app name from host header
    let host_parts = {
        let host = req
            .headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        // Split by '.' to get subdomain
        let parts: Vec<&str> = host.split('.').collect();
        parts
    };

    let app_name = host_parts.first().cloned().unwrap_or("").to_string();
    let path = req.uri().path();

    // Handle different subdomains
    if path.starts_with("/____bindrop_api/") {
        // Route to API server via message passing
        match handle_api_request(Arc::clone(&state), req).await {
            Ok(response) => Ok(response),
            Err(e) => {
                error!("API error: {}", e);
                Ok(Response::builder()
                    .status(500)
                    .body(Body::from(format!("API error: {}", e)))
                    .unwrap())
            }
        }
    } else if app_name == "admin" {
        // Regular admin interface
        Ok(admin_interface(state).await)
    } else {
        // Regular proxy to app
        match proxy_to_app(state, &app_name, req).await {
            Ok(response) => Ok(response),
            Err(e) => {
                error!("Proxy error: {}", e);
                Ok(Response::builder()
                    .status(500)
                    .body(Body::from(format!("Proxy error: {}", e)))
                    .unwrap())
            }
        }
    }
}

/// Handle API requests by sending messages to the API server
async fn handle_api_request(
    state: Arc<RwLock<ProxyState>>,
    req: Request<Body>,
) -> anyhow::Result<Response<Body>> {
    let path = req.uri().path();
    let method = req.method().clone();
    let pool = state.read().await;

    // Parse path to determine API action
    let api_request = match (method.as_str(), path) {
        ("GET", "/____bindrop_api/apps") => {
            let apps = db::apps::get_all(&pool.db_pool)
                .await?
                .into_iter()
                .map(|app| AppInfo {
                    id: app.id.to_string(),
                    name: app.name,
                    state: app.state.to_string(),
                    host: app.host,
                    port: app.port,
                    process_id: app.process_id,
                })
                .collect::<Vec<AppInfo>>();
            Some(serde_json::to_string(&apps)?)
        }
        ("GET", path) if path.starts_with("/____bindrop_api/apps/") => {
            let name = path.trim_start_matches("/api/apps/").to_string();
            let app = db::apps::get_by_name(&pool.db_pool, &name).await?;
            if let Some(app) = app {
                let app_info = AppInfo {
                    id: app.id.to_string(),
                    name: app.name,
                    state: app.state.to_string(),
                    host: app.host,
                    port: app.port,
                    process_id: app.process_id,
                };
                Some(serde_json::to_string(&app_info)?)
            } else {
                None
            }
        }
        ("POST", path)
            if path.starts_with("/____bindrop_api/apps/") && path.ends_with("/start") =>
        {
            let app_name = path
                .trim_start_matches("/____bindrop_api/apps/")
                .trim_end_matches("/start")
                .to_string();
            match start::execute(&app_name).await {
                Ok(()) => Some(serde_json::to_string(&format!(
                    "App '{}' started",
                    app_name
                ))?),
                Err(e) => {
                    error!("Failed to start app: {}", e);
                    return Ok(Response::builder()
                        .status(400)
                        .body(Body::from("Failed to start app"))
                        .unwrap());
                }
            }
        }
        ("POST", path) if path.starts_with("/____bindrop_api/apps/") && path.ends_with("/stop") => {
            let app_name = path
                .trim_start_matches("/____bindrop_api/apps/")
                .trim_end_matches("/stop")
                .to_string();
            match stop::execute(&app_name).await {
                Ok(()) => Some(serde_json::to_string(&format!(
                    "App '{}' stoped",
                    app_name
                ))?),
                Err(e) => {
                    error!("Failed to stop app: {}", e);
                    return Ok(Response::builder()
                        .status(400)
                        .body(Body::from("Failed to stop app"))
                        .unwrap());
                }
            }
        }
        _ => None,
    };

    // If we have a valid API request, send it to the API server
    if let Some(response_body) = api_request {
        // Build HTTP response
        Ok(Response::builder()
            .header("Content-Type", "application/json")
            .body(Body::from(response_body))
            .unwrap())
    } else {
        // Unknown API endpoint
        Ok(Response::builder()
            .status(404)
            .body(Body::from("API endpoint not found"))
            .unwrap())
    }
}

/// Admin interface handler
async fn admin_interface(state: Arc<RwLock<ProxyState>>) -> Response<Body> {
    let pool = state.read().await.db_pool.clone();
    let apps = db::apps::get_all(&pool).await.unwrap_or_else(|_| vec![]);

    // Build HTML response
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>BinaryDrop Admin</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; margin: 0; padding: 20px; }
        h1 { color: #333; }
        table { border-collapse: collapse; width: 100%; }
        th, td { text-align: left; padding: 8px; }
        tr:nth-child(even) { background-color: #f2f2f2; }
        th { background-color: #4CAF50; color: white; }
        .running { color: green; }
        .stopped { color: red; }
        .created { color: blue; }
    </style>
</head>
<body>
    <h1>BinaryDrop Admin</h1>
    <h2>Apps</h2>
    <table>
        <tr>
            <th>Name</th>
            <th>Status</th>
            <th>Port</th>
            <th>PID</th>
            <th>URL</th>
        </tr>
"#,
    );

    // Add rows for each app
    for app in apps {
        let status_class = match app.state {
            AppState::Running => "running",
            AppState::Stopped => "stopped",
            _ => "created",
        };

        let pid = match app.process_id {
            Some(pid) => pid.to_string(),
            None => "-".to_string(),
        };

        html.push_str(&format!(
            r#"<tr>
            <td>{}</td>
            <td class="{}">{}</td>
            <td>{}</td>
            <td>{}</td>
            <td><a href="http://{}:{}" target="_blank">http://{}:{}</a></td>
        </tr>"#,
            app.name,
            status_class,
            app.state,
            app.port,
            pid,
            app.host,
            app.port,
            app.host,
            app.port,
        ));
    }

    // Add API information
    html.push_str(
        r#"
    </table>
    <h2>API Endpoints</h2>
    <ul>
        <li><a href="/api/apps">GET /api/apps</a> - List all apps</li>
        <li>POST /api/apps - Create a new app</li>
        <li>GET /api/apps/:id - Get app details</li>
        <li>DELETE /api/apps/:id - Delete an app</li>
        <li>POST /api/apps/:id/start - Start an app</li>
        <li>POST /api/apps/:id/stop - Stop an app</li>
    </ul>
</body>
</html>
"#,
    );

    Response::builder()
        .header("Content-Type", "text/html")
        .body(Body::from(html))
        .unwrap()
}

/// Proxy request to app
async fn proxy_to_app(
    state: Arc<RwLock<ProxyState>>,
    app_name: &str,
    req: Request<Body>,
) -> anyhow::Result<Response<Body>> {
    let pool = state.read().await.db_pool.clone();
    let app = db::apps::get_by_name(&pool, app_name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("App '{}' not found", app_name))?;

    // Check if app is running
    if app.state != AppState::Running {
        return Ok(Response::builder()
            .status(503)
            .body(Body::from(format!("App '{}' is not running", app_name)))
            .unwrap());
    }

    // Create URI for proxying
    let path_and_query = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("");
    let uri = format!("http://{}:{}{}", app.host, app.port, path_and_query);

    // Create new request
    let (parts, body) = req.into_parts();
    let mut new_req = Request::builder().method(parts.method).uri(uri);

    // Copy headers
    for (name, value) in parts.headers.iter() {
        if name != "host" {
            new_req = new_req.header(name, value);
        }
    }

    // Add custom headers
    if let Some(host) = parts.headers.get("host") {
        new_req = new_req.header("X-Forwarded-Host", host);
    }
    new_req = new_req.header("X-Forwarded-Proto", "http");

    // Build request
    let new_req = new_req.body(body).context("Failed to build request")?;

    // Send request
    let client = Client::new();
    let resp = client
        .request(new_req)
        .await
        .context("Proxy request failed")?;

    Ok(resp)
}
