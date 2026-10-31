use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{error::AppError, AppState};

#[derive(Deserialize)]
pub(crate) struct NewShare {
    user_id: Option<i64>,
    permission: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct ShareOut {
    user_id: i64,
    permission: String,
}

fn header_user_id(headers: &HeaderMap) -> Result<i64, AppError> {
    let value = headers
        .get("x-user-id")
        .ok_or_else(|| AppError::Unauthorized("missing X-User-Id header".to_owned()))?;
    let value = value
        .to_str()
        .map_err(|_| AppError::Unauthorized("invalid X-User-Id header".to_owned()))?;
    value
        .parse::<i64>()
        .map_err(|_| AppError::Unauthorized("invalid X-User-Id header".to_owned()))
}

fn map_db_error(_error: rusqlite::Error) -> AppError {
    AppError::Internal("database error".to_owned())
}

fn require_owner(conn: &Connection, user_id: i64, note_id: i64) -> Result<i64, AppError> {
    let note: Option<(i64, Option<i64>)> = conn
        .query_row(
            "SELECT owner_id, team_id FROM notes WHERE id = ?",
            [note_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(map_db_error)?;
    let (owner_id, team_id) = note.ok_or_else(|| AppError::NotFound("note not found".to_owned()))?;

    if owner_id == user_id {
        return Ok(owner_id);
    }

    let share_permission: Option<String> = conn
        .query_row(
            "SELECT permission FROM note_shares WHERE note_id = ? AND user_id = ?",
            params![note_id, user_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_db_error)?;
    if share_permission.is_some() {
        return Err(AppError::Forbidden("insufficient permissions".to_owned()));
    }

    let is_team_member = if let Some(team_id) = team_id {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = ? AND user_id = ?)",
            params![team_id, user_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(map_db_error)?
    } else {
        false
    };

    if is_team_member {
        Err(AppError::Forbidden("insufficient permissions".to_owned()))
    } else {
        Err(AppError::NotFound("note not found".to_owned()))
    }
}

fn authenticate(conn: &Connection, user_id: i64) -> Result<(), AppError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)",
            [user_id],
            |row| row.get(0),
        )
        .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(AppError::Unauthorized("unknown user".to_owned()))
    }
}

pub(crate) async fn create_share(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
    headers: HeaderMap,
    json: Result<Json<NewShare>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<ShareOut>), AppError> {
    let user_id = header_user_id(&headers)?;
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("database lock poisoned".to_owned()))?;
    authenticate(&conn, user_id)?;
    let note_id = note_id
        .parse::<i64>()
        .map_err(|_| AppError::NotFound("note not found".to_owned()))?;
    let owner_id = require_owner(&conn, user_id, note_id)?;

    let Json(input) = json.map_err(AppError::from)?;
    let target_id = input
        .user_id
        .ok_or_else(|| AppError::BadRequest("user_id and permission are required".to_owned()))?;
    let permission = input
        .permission
        .ok_or_else(|| AppError::BadRequest("user_id and permission are required".to_owned()))?;
    if permission != "read" && permission != "write" {
        return Err(AppError::BadRequest(
            "permission must be 'read' or 'write'".to_owned(),
        ));
    }

    let target_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)",
            [target_id],
            |row| row.get(0),
        )
        .map_err(map_db_error)?;
    if !target_exists {
        return Err(AppError::NotFound("user not found".to_owned()));
    }
    if target_id == owner_id {
        return Err(AppError::Conflict("already the owner".to_owned()));
    }

    let already_shared: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM note_shares WHERE note_id = ? AND user_id = ?)",
            params![note_id, target_id],
            |row| row.get(0),
        )
        .map_err(map_db_error)?;
    if already_shared {
        return Err(AppError::Conflict("already shared".to_owned()));
    }

    conn.execute(
        "INSERT INTO note_shares (note_id, user_id, permission) VALUES (?, ?, ?)",
        params![note_id, target_id, permission],
    )
    .map_err(map_db_error)?;

    Ok((
        StatusCode::CREATED,
        Json(ShareOut {
            user_id: target_id,
            permission,
        }),
    ))
}

pub(crate) async fn list_shares(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<ShareOut>>, AppError> {
    let user_id = header_user_id(&headers)?;
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("database lock poisoned".to_owned()))?;
    authenticate(&conn, user_id)?;
    let note_id = note_id
        .parse::<i64>()
        .map_err(|_| AppError::NotFound("note not found".to_owned()))?;
    require_owner(&conn, user_id, note_id)?;

    let mut statement = conn
        .prepare(
            "SELECT user_id, permission FROM note_shares WHERE note_id = ? ORDER BY user_id",
        )
        .map_err(map_db_error)?;
    let rows = statement
        .query_map([note_id], |row| {
            Ok(ShareOut {
                user_id: row.get(0)?,
                permission: row.get(1)?,
            })
        })
        .map_err(map_db_error)?;
    let shares = rows.collect::<Result<Vec<_>, _>>().map_err(map_db_error)?;
    Ok(Json(shares))
}

pub(crate) async fn delete_share(
    State(state): State<AppState>,
    Path((note_id, target_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let user_id = header_user_id(&headers)?;
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("database lock poisoned".to_owned()))?;
    authenticate(&conn, user_id)?;
    let note_id = note_id
        .parse::<i64>()
        .map_err(|_| AppError::NotFound("note not found".to_owned()))?;
    let target_id = target_id
        .parse::<i64>()
        .map_err(|_| AppError::NotFound("user not found".to_owned()))?;
    require_owner(&conn, user_id, note_id)?;
    conn.execute(
        "DELETE FROM note_shares WHERE note_id = ? AND user_id = ?",
        params![note_id, target_id],
    )
    .map_err(map_db_error)?;
    Ok(StatusCode::NO_CONTENT)
}
