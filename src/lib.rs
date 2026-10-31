use axum::{Json, routing::get};

pub fn router() -> axum::Router {
    axum::Router::new().route("/health", get(health))
}

pub(crate) async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}
