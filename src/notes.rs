use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use rusqlite::{params, OptionalExtension};

use crate::error::AppError;
use crate::users;
use crate::AppState;

#[derive(serde::Deserialize)]
pub struct NewNote {
    title: Option<String>,
    body: Option<String>,
    team_id: Option<i64>,
}

#[derive(serde::Deserialize)]
pub struct PatchNote {
    title: Option<String>,
    body: Option<String>,
    version: Option<i64>,
}

#[derive(serde::Serialize, Clone)]
pub struct NoteOut {
    pub id: i64,
    pub owner_id: i64,
    pub team_id: Option<i64>,
    pub title: String,
    pub body: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

enum Perm {
    Owner,
    Write,
    Read,
}

struct NoteRow {
    id: i64,
    owner_id: i64,
    team_id: Option<i64>,
    title: String,
    body: String,
    version: i64,
    created_at: String,
    updated_at: String,
}

fn access(conn: &rusqlite::Connection, user: i64, note: &NoteRow) -> Option<Perm> {
    if note.owner_id == user {
        return Some(Perm::Owner);
    }

    let share: Option<String> = match conn
        .query_row(
            "SELECT permission FROM note_shares WHERE note_id = ?1 AND user_id = ?2",
            params![note.id, user],
            |row| row.get(0),
        )
        .optional()
    {
        Ok(share) => share,
        Err(_) => return None,
    };
    match share.as_deref() {
        Some("write") => return Some(Perm::Write),
        Some("read") => return Some(Perm::Read),
        Some(_) => return None,
        None => {}
    }

    let team_id = note.team_id?;
    let is_member: bool = match conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2
         )",
        params![team_id, user],
        |row| row.get(0),
    ) {
        Ok(is_member) => is_member,
        Err(_) => return None,
    };
    is_member.then_some(Perm::Read)
}

fn permits(actual: Perm, required: Perm) -> bool {
    matches!(
        (actual, required),
        (Perm::Owner, _) | (Perm::Write, Perm::Write | Perm::Read) | (Perm::Read, Perm::Read)
    )
}

impl From<NoteRow> for NoteOut {
    fn from(note: NoteRow) -> Self {
        Self {
            id: note.id,
            owner_id: note.owner_id,
            team_id: note.team_id,
            title: note.title,
            body: note.body,
            version: note.version,
            created_at: note.created_at,
            updated_at: note.updated_at,
        }
    }
}

fn note_row(conn: &rusqlite::Connection, id: i64) -> rusqlite::Result<NoteRow> {
    conn.query_row(
        "SELECT id, owner_id, team_id, title, body, version, created_at, updated_at
         FROM notes WHERE id = ?1",
        [id],
        |row| {
            Ok(NoteRow {
                id: row.get(0)?,
                owner_id: row.get(1)?,
                team_id: row.get(2)?,
                title: row.get(3)?,
                body: row.get(4)?,
                version: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
}

fn note_error(error: rusqlite::Error) -> AppError {
    match error {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("note not found".into()),
        other => AppError::Internal(other.to_string()),
    }
}

fn identity(conn: &rusqlite::Connection, headers: &HeaderMap) -> Result<i64, AppError> {
    let id = headers
        .get("x-user-id")
        .ok_or_else(|| AppError::Unauthorized("missing X-User-Id header".into()))?
        .to_str()
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| AppError::Unauthorized("invalid X-User-Id header".into()))?;

    match conn.query_row("SELECT 1 FROM users WHERE id = ?1", [id], |_| Ok(())) {
        Ok(()) => Ok(id),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(AppError::Unauthorized("unknown user".into()))
        }
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

pub async fn create_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    json: Result<axum::Json<NewNote>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<NoteOut>), AppError> {
    let conn = state.db.lock().unwrap();
    let owner_id = identity(&conn, &headers)?;
    let NewNote {
        title,
        body,
        team_id,
    } = json.map_err(AppError::from)?.0;
    let title = title
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::BadRequest("title is required".into()))?;
    let body = body.unwrap_or_default();

    if let Some(team_id) = team_id {
        let team_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM teams WHERE id = ?1)",
                [team_id],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if !team_exists {
            return Err(AppError::NotFound("team not found".into()));
        }

        let is_member: bool = conn
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2
                 )",
                params![team_id, owner_id],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if !is_member {
            return Err(AppError::Forbidden("not a member of that team".into()));
        }
    }

    conn.execute(
        "INSERT INTO notes (owner_id, team_id, title, body, version)
         VALUES (?1, ?2, ?3, ?4, 1)",
        params![owner_id, team_id, title, body],
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;
    let note_id = conn.last_insert_rowid();
    let note = note_row(&conn, note_id).map_err(note_error)?;
    Ok((StatusCode::CREATED, Json(note.into())))
}

pub async fn get_note(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> Result<Json<NoteOut>, AppError> {
    let conn = state.db.lock().unwrap();
    let user_id = identity(&conn, &headers)?;
    let id = users::parse_id(&id_str)?;
    let note = note_row(&conn, id).map_err(note_error)?;
    let permission =
        access(&conn, user_id, &note).ok_or_else(|| AppError::NotFound("note not found".into()))?;
    debug_assert!(permits(permission, Perm::Read));
    Ok(Json(note.into()))
}

pub async fn patch_note(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    json: Result<axum::Json<PatchNote>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<NoteOut>, AppError> {
    let conn = state.db.lock().unwrap();
    let user_id = identity(&conn, &headers)?;
    let id = users::parse_id(&id_str)?;
    let PatchNote {
        title,
        body,
        version,
    } = json.map_err(AppError::from)?.0;
    let version = version.ok_or_else(|| AppError::BadRequest("version is required".into()))?;
    if title.is_none() && body.is_none() {
        return Err(AppError::BadRequest("title or body is required".into()));
    }
    let title = match title {
        Some(value) => {
            let value = value.trim().to_owned();
            if value.is_empty() {
                return Err(AppError::BadRequest("title is required".into()));
            }
            Some(value)
        }
        None => None,
    };

    let current = note_row(&conn, id).map_err(note_error)?;
    let permission = access(&conn, user_id, &current)
        .ok_or_else(|| AppError::NotFound("note not found".into()))?;
    if !permits(permission, Perm::Write) {
        return Err(AppError::Forbidden("insufficient permissions".into()));
    }
    if current.version != version {
        return Err(AppError::Conflict("version conflict".into()));
    }

    let changed = match (title.as_deref(), body.as_deref()) {
        (Some(title), Some(body)) => conn.execute(
            "UPDATE notes SET title = ?1, body = ?2, version = version + 1,
             updated_at = datetime('now') WHERE id = ?3 AND version = ?4",
            params![title, body, id, version],
        ),
        (Some(title), None) => conn.execute(
            "UPDATE notes SET title = ?1, version = version + 1,
             updated_at = datetime('now') WHERE id = ?2 AND version = ?3",
            params![title, id, version],
        ),
        (None, Some(body)) => conn.execute(
            "UPDATE notes SET body = ?1, version = version + 1,
             updated_at = datetime('now') WHERE id = ?2 AND version = ?3",
            params![body, id, version],
        ),
        (None, None) => unreachable!("title or body was validated above"),
    }
    .map_err(|e| AppError::Internal(e.to_string()))?;
    if changed == 0 {
        return Err(AppError::Conflict("version conflict".into()));
    }

    let updated = note_row(&conn, id).map_err(note_error)?;
    Ok(Json(updated.into()))
}

pub async fn delete_note(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let conn = state.db.lock().unwrap();
    let user_id = identity(&conn, &headers)?;
    let id = users::parse_id(&id_str)?;
    let note = note_row(&conn, id).map_err(note_error)?;
    let permission =
        access(&conn, user_id, &note).ok_or_else(|| AppError::NotFound("note not found".into()))?;
    if !matches!(permission, Perm::Owner) {
        return Err(AppError::Forbidden("insufficient permissions".into()));
    }
    conn.execute(
        "DELETE FROM notes WHERE id = ?1 AND owner_id = ?2",
        params![id, user_id],
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
