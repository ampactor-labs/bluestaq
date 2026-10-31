use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::db;
use crate::error::AppError;
use crate::AppState;

#[derive(serde::Deserialize)]
pub struct NewUser {
    name: Option<String>,
    email: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct UserOut {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub created_at: String,
}

fn user_out(conn: &rusqlite::Connection, id: i64) -> rusqlite::Result<UserOut> {
    conn.query_row(
        "SELECT id, name, email, created_at FROM users WHERE id = ?1",
        [id],
        |row| {
            Ok(UserOut {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                created_at: row.get(3)?,
            })
        },
    )
}

/// Parse `{id}` path params: non-numeric ids are treated as nonexistent (404).
pub(crate) fn parse_id(id_str: &str) -> Result<i64, AppError> {
    id_str
        .parse::<i64>()
        .map_err(|_| AppError::NotFound("not found".into()))
}

pub async fn create_user(
    State(state): State<AppState>,
    json: Result<axum::Json<NewUser>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<UserOut>), AppError> {
    let NewUser { name, email } = json.map_err(AppError::from)?.0;
    let name = name.and_then(|s| {
        let t = s.trim().to_owned();
        (!t.is_empty()).then_some(t)
    });
    let email = email.and_then(|s| {
        let t = s.trim().to_lowercase();
        (!t.is_empty()).then_some(t)
    });
    let (name, email) = match (name, email) {
        (Some(name), Some(email)) => (name, email),
        _ => return Err(AppError::BadRequest("name and email are required".into())),
    };

    let conn = state.db.lock().unwrap();
    match conn.execute("INSERT INTO users (name, email) VALUES (?1, ?2)", rusqlite::params![name, email]) {
        Ok(_) => {
            let id = conn.last_insert_rowid();
            let user = user_out(&conn, id).map_err(|e| AppError::Internal(e.to_string()))?;
            Ok((StatusCode::CREATED, Json(user)))
        }
        Err(e) if db::is_unique(&e) => Err(AppError::Conflict("email already registered".into())),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

pub async fn get_user(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<Json<UserOut>, AppError> {
    let id = parse_id(&id_str)?;
    let conn = state.db.lock().unwrap();
    match user_out(&conn, id) {
        Ok(user) => Ok(Json(user)),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(AppError::NotFound("user not found".into()))
        }
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}
