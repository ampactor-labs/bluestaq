use axum::{http::StatusCode, Json, routing::{delete, get, patch, post}};
use std::sync::{Arc, Mutex};

pub mod db;
pub mod auth;
pub mod error;
pub mod notes;
pub mod shares;
pub mod teams;
pub mod users;

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
        .route("/users", post(users::create_user))
        .route("/users/{id}", get(users::get_user))
        .route("/teams", post(teams::create_team))
        .route("/teams/{id}", get(teams::get_team))
        .route("/teams/{id}/members", post(teams::add_member).get(teams::list_members))
        .route("/notes", post(notes::create_note).get(notes::list_notes))
        .route("/notes/{id}", get(notes::get_note))
        .route("/notes/{id}", patch(notes::patch_note))
        .route("/notes/{id}", delete(notes::delete_note))
        .route(
            "/notes/{id}/shares",
            axum::routing::post(shares::create_share).get(shares::list_shares),
        )
        .route(
            "/notes/{id}/shares/{user_id}",
            axum::routing::delete(shares::delete_share),
        )
        .fallback(fallback_json)
        .with_state(state)
}

async fn fallback_json() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "not found" })),
    )
}

pub(crate) async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}
