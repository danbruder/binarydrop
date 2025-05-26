use std::process::Child;

use sqlx::{Pool, Sqlite};
use tracing::{info, instrument};

use crate::db;
use crate::models::ProcessHistory;
use crate::providers::{Handle, Provider};

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum StartError {
    #[error("App not found: {0}")]
    AppNotFound(String),
    #[error("App already running: {0}")]
    AppAlreadyRunning(String),
    #[error("App not deployed: {0}")]
    AppNotDeployed(String),
    #[error("Failed to start app: {0}")]
    AppStartFailed(String),
    #[error("App log broken: {0}")]
    AppLogBroken(String),
    #[error("Invalid binary path: {0}")]
    InvalidBinaryPath(String),
    #[error("Database error")]
    DatabaseError(String),
    #[error("Config error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
}

type Result<T> = anyhow::Result<T, StartError>;

impl From<crate::db::DatabaseError> for StartError {
    fn from(err: crate::db::DatabaseError) -> Self {
        StartError::DatabaseError(err.to_string())
    }
}

/// Start an app using the supervisor
#[instrument(skip(pool, provider))]
pub async fn execute<H: Handle>(
    pool: &Pool<Sqlite>,
    app_name: &str,
    provider: impl Provider<Handle = H>,
) -> Result<H> {
    // VALIDATION
    let app = db::apps::get_by_name(pool, app_name)
        .await?
        .ok_or_else(|| StartError::AppNotFound(app_name.to_string()))?;

    if app.is_running() {
        return Err(StartError::AppAlreadyRunning(app_name.to_string()));
    }

    if !app.is_deployed() {
        return Err(StartError::AppNotDeployed(app.name.clone()));
    }

    // Start
    let app = app.started();
    db::apps::save(pool, &app).await?;

    let handle = provider
        .start(&app)
        .await
        .map_err(|e| StartError::AppStartFailed(e.to_string()))?;
    let process_id = handle.id();

    // SAVE STATE

    // Record process history
    let history = ProcessHistory::new(&app.id);
    db::process_history::save(pool, &history).await?;

    // Update app with process ID
    let app = app.running(process_id);
    db::apps::save(pool, &app).await?;

    info!("Started app '{}' with PID {}", app.name, process_id);

    Ok(handle)
}

#[cfg(test)]
mod test {
    use crate::db::apps;
    use crate::models::App;
    use crate::providers::{Handle, Provider};

    struct TestProvider;

    impl Provider for TestProvider {
        type Handle = bool;

        async fn start(&self, _app: &App) -> anyhow::Result<bool> {
            Ok(true)
        }
    }

    impl Handle for bool {
        fn id(&self) -> u32 {
            1
        }
    }

    #[tokio::test]
    async fn test_starting_non_existant_app() {
        let pool = crate::db::test::get_test_pool().await;
        let got = super::execute(&pool, "non_existant_app", TestProvider)
            .await
            .unwrap_err();
        let want = super::StartError::AppNotFound("non_existant_app".to_string());

        assert_eq!(got, want);
    }

    #[tokio::test]
    async fn test_starting_not_deployed_app() {
        let pool = crate::db::test::get_test_pool().await;
        let app_name = "app";
        let app = App::new(app_name, 8080).unwrap();
        apps::save(&pool, &app).await.unwrap();

        let got = super::execute(&pool, app_name, TestProvider)
            .await
            .unwrap_err();
        let want = super::StartError::AppNotDeployed(app_name.to_string());

        assert_eq!(got, want);
    }

    #[tokio::test]
    async fn test_starting_already_running_app() {
        let pool = crate::db::test::get_test_pool().await;
        let app_name = "app";
        let app = App::new(app_name, 8080)
            .unwrap()
            .deployed("some_path".into(), "some_hash".into())
            .running(1);
        apps::save(&pool, &app).await.unwrap();

        let got = super::execute(&pool, app_name, TestProvider)
            .await
            .unwrap_err();
        let want = super::StartError::AppAlreadyRunning(app_name.to_string());

        assert_eq!(got, want);
    }

    #[tokio::test]
    async fn test_start_happy_path() {
        let pool = crate::db::test::get_test_pool().await;
        let app_name = "app";
        let app = App::new(app_name, 8080)
            .unwrap()
            .deployed("some_path".into(), "some_hash".into());
        apps::save(&pool, &app).await.unwrap();

        let got = super::execute(&pool, app_name, TestProvider).await.unwrap();
        let want = true;

        assert_eq!(got, want);
    }
}
