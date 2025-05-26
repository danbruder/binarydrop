pub mod cmd;

use crate::models::App;
use anyhow::Result;
use sqlx::{Pool, Sqlite};

pub trait Provider {
    type Handle: Handle;

    async fn start(&self, app: &App) -> Result<Self::Handle>;
    async fn setup(&self, pool: &Pool<Sqlite>, app: &App) -> Result<App>;
}

pub trait Handle {
    fn id(&self) -> u32;
    // fn is_running(&self) -> bool;
    // fn stop(&mut self) -> Result<()>;
    // fn restart(&mut self) -> Result<()>;
    //fn name(&self) -> String;
}
