use axum::http::HeaderMap;

use crate::error::AppError;
use crate::AppState;

/// Resolve the request's X-User-Id to an existing user.
pub fn identity(state: &AppState, headers: &HeaderMap) -> Result<i64, AppError> {
    let id = headers
        .get("x-user-id")
        .ok_or_else(|| AppError::Unauthorized("missing X-User-Id header".into()))?
        .to_str()
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| AppError::Unauthorized("invalid X-User-Id header".into()))?;

    let conn = state.db.lock().unwrap();
    match conn.query_row("SELECT 1 FROM users WHERE id = ?1", [id], |_| Ok(())) {
        Ok(()) => Ok(id),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(AppError::Unauthorized("unknown user".into()))
        }
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}
