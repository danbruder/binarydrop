use anyhow::{Context, Result};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, instrument};

use crate::db;
use crate::models::AppState;
use crate::supervisor;

/// Shared state for the proxy server
struct ProxyState {
    db_pool: sqlx::Pool<sqlx::Sqlite>,
}

/// Start the BinaryDrop server
#[instrument]
pub async fn execute(host: &str, port: u16) -> Result<()> {
    // Connect to database
    let pool = db::init_pool().await?;
    supervisor::init(pool.clone()).await?;

    // Create shared state
    let state = Arc::new(RwLock::new(ProxyState { db_pool: pool }));

    // Parse host and port
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .context(format!("Invalid host or port: {}:{}", host, port))?;

    // Create service
    let make_svc = make_service_fn(move |_conn| {
        let state = Arc::clone(&state);
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let state = Arc::clone(&state);
                async move { handle_request(state, req).await }
            }))
        }
    });

    // Create server
    let server = Server::bind(&addr).serve(make_svc);

    info!("Starting BinaryDrop server on http://{}:{}", host, port);
    println!("BinaryDrop server running at http://{}:{}", host, port);
    println!("Press Ctrl+C to stop");

    // Run server
    server.await.context("Server error")?;

    Ok(())
}

/// Handle incoming requests
async fn handle_request(
    state: Arc<RwLock<ProxyState>>,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    // Extract app name from host header before moving req
    let app_name = {
        let host = req
            .headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        host.split('.').next().unwrap_or("").to_string() // Convert to owned String to avoid borrowing issues
    };

    // Now we can safely move req
    if app_name == "admin" {
        // Handle admin interface
        return Ok(admin_interface(state).await);
    } else {
        // Handle proxy to app
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

/// Admin interface handler (separated from handle_request to avoid req ownership issues)
async fn admin_interface(state: Arc<RwLock<ProxyState>>) -> Response<Body> {
    // Get all apps
    let state_read = state.read().await;
    let apps = match db::apps::get_all(&state_read.db_pool).await {
        Ok(apps) => apps,
        Err(e) => {
            return Response::builder()
                .status(500)
                .body(Body::from(format!("Error: {}", e)))
                .unwrap();
        }
    };

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

    // Close HTML
    html.push_str(
        r#"
    </table>
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
    // Get app
    let state_read = state.read().await;
    let app = match db::apps::get_by_name(&state_read.db_pool, app_name).await? {
        Some(app) => app,
        None => {
            return Ok(Response::builder()
                .status(404)
                .body(Body::from(format!("App '{}' not found", app_name)))
                .unwrap());
        }
    };

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
