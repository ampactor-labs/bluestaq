use axum::{Json, routing::get};
use std::sync::{Arc, Mutex};

pub mod db;
pub mod error;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
}

impl AppState {
    pub fn in_memory() -> Self {
        Self {
            db: Arc::new(Mutex::new(db::new_in_memory())),
        }
    }
}

pub fn router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/health", get(health))
        .with_state(state)
}

pub(crate) async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}
