use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::config;
use crate::db;
use crate::models::{App, AppState, HealthCheckType, ProcessHistory};

use once_cell::sync::OnceCell;
