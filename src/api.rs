use crate::commands::app_command::create;
use crate::commands::app_command::{start, stop};
use crate::commands::server_command::serve::ProxyState;
use crate::db;
use axum::{
    extract::{Path, State, Query},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::commands::app_command::logs;
use axum::http::StatusCode;

pub fn create_api_router(state: Arc<RwLock<ProxyState>>) -> Router {
    Router::new()
        .route("/apps", get(list_apps))
        .route("/apps", post(create_app))
        .route("/apps/:name", get(get_app))
        .route("/apps/:name/start", post(start_app))
        .route("/apps/:name/stop", post(stop_app))
        .route("/apps/:name/restart", post(restart_app))
        .route("/api/apps/:app_name/logs", get(get_logs))
        .with_state(state)
}

async fn list_apps(State(state): State<Arc<RwLock<ProxyState>>>) -> impl IntoResponse {
    let pool = state.read().await.db_pool.clone();
    match db::apps::get_all(&pool).await {
        Ok(apps) => {
            let app_infos = apps
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
            Json(app_infos).into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list apps: {}", e),
        )
            .into_response(),
    }
}

async fn get_app(
    State(state): State<Arc<RwLock<ProxyState>>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let pool = state.read().await.db_pool.clone();
    match db::apps::get_by_name(&pool, &name).await {
        Ok(Some(app)) => {
            let app_info = AppInfo {
                id: app.id.to_string(),
                name: app.name,
                state: app.state.to_string(),
                host: app.host,
                port: app.port,
                process_id: app.process_id,
            };
            Json(app_info).into_response()
        }
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            format!("App '{}' not found", name),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get app: {}", e),
        )
            .into_response(),
    }
}

async fn start_app(
    State(_state): State<Arc<RwLock<ProxyState>>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match start::execute(&name).await {
        Ok(_) => (
            axum::http::StatusCode::OK,
            format!("App '{}' started", name),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to start app: {}", e),
        )
            .into_response(),
    }
}

async fn stop_app(
    State(_state): State<Arc<RwLock<ProxyState>>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match stop::execute(&name).await {
        Ok(_) => (
            axum::http::StatusCode::OK,
            format!("App '{}' stopped", name),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to stop app: {}", e),
        )
            .into_response(),
    }
}

async fn restart_app(
    State(_state): State<Arc<RwLock<ProxyState>>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // First stop the app
    if let Err(e) = stop::execute(&name).await {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to stop app: {}", e),
        )
            .into_response();
    }

    // Then start it again
    match start::execute(&name).await {
        Ok(_) => (
            axum::http::StatusCode::OK,
            format!("App '{}' restarted", name),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to start app: {}", e),
        )
            .into_response(),
    }
}

async fn get_logs(
    Path(app_name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<RwLock<ProxyState>>>,
) -> impl IntoResponse {
    let lines = params.get("lines").and_then(|l| l.parse::<usize>().ok()).unwrap_or(50);
    let follow = params.get("follow").and_then(|f| f.parse::<bool>().ok()).unwrap_or(false);

    match logs::execute(&app_name, lines, follow).await {
        Ok(logs) => (StatusCode::OK, logs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Debug, serde::Deserialize)]
struct CreateAppRequest {
    name: String,
}

async fn create_app(
    State(state): State<Arc<RwLock<ProxyState>>>,
    Json(payload): Json<CreateAppRequest>,
) -> impl IntoResponse {
    let pool = state.read().await.db_pool.clone();
    match create::execute(&payload.name, &pool).await {
        Ok(_) => (
            axum::http::StatusCode::CREATED,
            format!("App '{}' created", payload.name),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create app: {}", e),
        )
            .into_response(),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct AppInfo {
    id: String,
    name: String,
    state: String,
    host: String,
    port: u16,
    process_id: Option<u32>,
}
