use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;

use crate::db;
use crate::error::AppError;
use crate::users::{parse_id, UserOut};
use crate::AppState;

#[derive(serde::Deserialize)]
pub struct NewTeam {
    name: Option<String>,
    created_by: Option<i64>,
}

#[derive(serde::Deserialize)]
pub struct AddMember {
    user_id: Option<i64>,
}

#[derive(serde::Serialize, Clone)]
pub struct TeamOut {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

fn team_out(conn: &rusqlite::Connection, id: i64) -> rusqlite::Result<TeamOut> {
    conn.query_row(
        "SELECT id, name, created_at FROM teams WHERE id = ?1",
        [id],
        |row| {
            Ok(TeamOut {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        },
    )
}

fn row_exists(conn: &rusqlite::Connection, table: &str, id: i64) -> rusqlite::Result<bool> {
    match conn.query_row(
        &format!("SELECT 1 FROM {table} WHERE id = ?1"),
        [id],
        |row| row.get::<_, i64>(0),
    ) {
        Ok(_) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e),
    }
}

fn member_actor_id(headers: &HeaderMap) -> Result<i64, AppError> {
    headers
        .get("x-user-id")
        .ok_or_else(|| AppError::Unauthorized("missing X-User-Id header".into()))?
        .to_str()
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| AppError::Unauthorized("invalid X-User-Id header".into()))
}

pub async fn create_team(
    State(state): State<AppState>,
    json: Result<axum::Json<NewTeam>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<TeamOut>), AppError> {
    let NewTeam { name, created_by } = json.map_err(AppError::from)?.0;
    let name = match name.and_then(|s| {
        let t = s.trim().to_owned();
        (!t.is_empty()).then_some(t)
    }) {
        Some(name) => name,
        None => return Err(AppError::BadRequest("name is required".into())),
    };
    let created_by = match created_by {
        Some(id) => id,
        None => return Err(AppError::BadRequest("created_by is required".into())),
    };

    let conn = state.db.lock().unwrap();
    if !row_exists(&conn, "users", created_by).map_err(|e| AppError::Internal(e.to_string()))? {
        return Err(AppError::NotFound("creator not found".into()));
    }
    match conn.execute("INSERT INTO teams (name) VALUES (?1)", [name.as_str()]) {
        Ok(_) => {
            let team_id = conn.last_insert_rowid();
            // Creator joins on the same lock (one lock, two statements).
            if let Err(e) = conn.execute(
                "INSERT INTO team_members (team_id, user_id) VALUES (?1, ?2)",
                rusqlite::params![team_id, created_by],
            ) {
                return Err(AppError::Internal(e.to_string()));
            }
            let team = team_out(&conn, team_id).map_err(|e| AppError::Internal(e.to_string()))?;
            Ok((StatusCode::CREATED, Json(team)))
        }
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

pub async fn get_team(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<Json<TeamOut>, AppError> {
    let id = parse_id(&id_str)?;
    let conn = state.db.lock().unwrap();
    match team_out(&conn, id) {
        Ok(team) => Ok(Json(team)),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(AppError::NotFound("team not found".into()))
        }
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

pub async fn add_member(
    State(state): State<AppState>,
    Path(team_id_str): Path<String>,
    headers: HeaderMap,
    json: Result<axum::Json<AddMember>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let actor_id = member_actor_id(&headers)?;
    let team_id = parse_id(&team_id_str)?;

    let conn = state.db.lock().unwrap();
    if !row_exists(&conn, "users", actor_id).map_err(|e| AppError::Internal(e.to_string()))? {
        return Err(AppError::Unauthorized("unknown user".into()));
    }
    if !row_exists(&conn, "teams", team_id).map_err(|e| AppError::Internal(e.to_string()))? {
        return Err(AppError::NotFound("team not found".into()));
    }

    let is_member = conn
        .query_row(
            "SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2",
            rusqlite::params![team_id, actor_id],
            |_| Ok(()),
        )
        .map(|()| true)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(false),
            other => Err(other),
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if !is_member {
        return Err(AppError::Forbidden("not a member of this team".into()));
    }

    let user_id = match json.map_err(AppError::from)?.0.user_id {
        Some(id) => id,
        None => return Err(AppError::BadRequest("user_id is required".into())),
    };
    if !row_exists(&conn, "users", user_id).map_err(|e| AppError::Internal(e.to_string()))? {
        return Err(AppError::NotFound("user not found".into()));
    }
    match conn.execute(
        "INSERT INTO team_members (team_id, user_id) VALUES (?1, ?2)",
        rusqlite::params![team_id, user_id],
    ) {
        Ok(_) => Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({ "team_id": team_id, "user_id": user_id })),
        )),
        Err(e) if db::is_unique(&e) => Err(AppError::Conflict("already a member".into())),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

pub async fn list_members(
    State(state): State<AppState>,
    Path(team_id_str): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserOut>>, AppError> {
    let actor_id = member_actor_id(&headers)?;
    let team_id = parse_id(&team_id_str)?;

    let conn = state.db.lock().unwrap();
    if !row_exists(&conn, "users", actor_id).map_err(|e| AppError::Internal(e.to_string()))? {
        return Err(AppError::Unauthorized("unknown user".into()));
    }
    if !row_exists(&conn, "teams", team_id).map_err(|e| AppError::Internal(e.to_string()))? {
        return Err(AppError::NotFound("team not found".into()));
    }

    let is_member = conn
        .query_row(
            "SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2",
            rusqlite::params![team_id, actor_id],
            |_| Ok(()),
        )
        .map(|()| true)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(false),
            other => Err(other),
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if !is_member {
        return Err(AppError::Forbidden("not a member of this team".into()));
    }
    let mut stmt = conn
        .prepare(
            "SELECT u.id, u.name, u.email, u.created_at
             FROM users u
             JOIN team_members tm ON tm.user_id = u.id
             WHERE tm.team_id = ?1
             ORDER BY tm.user_id",
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let members = stmt
        .query_map([team_id], |row| {
            Ok(UserOut {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| AppError::Internal(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(members))
}
