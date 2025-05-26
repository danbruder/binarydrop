use sqlx::{Pool, Sqlite};
use tracing::info;

use crate::db;
use crate::models::App;
use crate::providers::{Handle, Provider};

#[derive(Debug, thiserror::Error)]
pub enum AppCreateError {
    #[error("App already exists: {0}")]
    AppAlreadyExists(String),
    #[error("Failed to create app: {0}")]
    AppError(#[from] crate::models::AppError),
    #[error("Failed to create app: {0}")]
    DatabaseError(#[from] crate::db::DatabaseError),
    #[error("Internal Error")]
    InternalError,
    #[error("Provider error: {0}")]
    CmdProviderError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, AppCreateError>;

/// Create a new app
pub async fn execute<H: Handle>(
    pool: &Pool<Sqlite>,
    app_name: &str,
    provider: impl Provider<Handle = H>,
) -> Result<()> {
    // Check if app already exists
    if let Some(_) = db::apps::get_by_name(pool, app_name).await? {
        return Err(AppCreateError::AppAlreadyExists(app_name.to_string()));
    }

    // Create app
    let app = App::new(app_name)?;

    provider.setup(pool, &app).await?;

    // Save app to database
    db::apps::save(pool, &app).await?;

    info!("Created app '{}' on port {:?}", app.name, app.port);

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::test::get_test_pool;
    use crate::models::App;
    use crate::providers::cmd::CmdProvider;

    #[tokio::test]
    async fn test_create_app_already_exists() {
        let pool = get_test_pool().await;
        let app_name = "test_app";
        let app = App::new(app_name).unwrap();
        db::apps::save(&pool, &app).await.unwrap();

        let got = execute(&pool, app_name, CmdProvider {}).await.unwrap_err();
        match got {
            AppCreateError::AppAlreadyExists(ref n) if n == app_name => {}
            _ => panic!("Expected AppAlreadyExists, got: {:?}", got),
        }
    }

    #[tokio::test]
    async fn test_create_app_happy_path() {
        let pool = get_test_pool().await;
        let app_name = "test_app_happy";
        execute(&pool, app_name, CmdProvider {}).await.unwrap();
        let app = db::apps::get_by_name(&pool, app_name).await.unwrap();
        assert!(app.is_some());
    }

    #[tokio::test]
    async fn test_create_app_invalid_name() {
        let pool = get_test_pool().await;
        let app_name = "";
        let got = execute(&pool, app_name, CmdProvider {}).await.unwrap_err();
        match got {
            AppCreateError::AppError(_) => {}
            _ => panic!("Expected AppError for invalid name, got: {:?}", got),
        }
    }
}
