use axum::body::Body;
use axum::http::Request;
use rusqlite::params;
use tower::ServiceExt;

async fn req(
    app: axum::Router,
    method: &str,
    path: &str,
    user_id: Option<&str>,
    body: Option<&str>,
) -> (u16, Vec<u8>) {
    let body = body.map_or_else(Body::empty, |value| Body::from(value.to_owned()));
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(axum::http::header::CONTENT_TYPE, "application/json");
    if let Some(user_id) = user_id {
        builder = builder.header("x-user-id", user_id);
    }
    let response = app.oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, bytes.to_vec())
}

async fn make_user(app: &axum::Router, name: &str, email: &str) -> i64 {
    let body = format!(r#"{{"name":"{name}","email":"{email}"}}"#);
    let (status, response) = req(app.clone(), "POST", "/users", None, Some(&body)).await;
    assert_eq!(status, 201);
    serde_json::from_slice::<serde_json::Value>(&response).unwrap()["id"]
        .as_i64()
        .unwrap()
}

async fn setup() -> (axum::Router, i64, i64, i64, i64, i64) {
    let state = notes::AppState::in_memory();
    let app = notes::router(state.clone());
    let owner_id = make_user(&app, "Owner", "owner@example.com").await;
    let writer_id = make_user(&app, "Writer", "writer@example.com").await;
    let reader_id = make_user(&app, "Reader", "reader@example.com").await;
    let outsider_id = make_user(&app, "Outsider", "outsider@example.com").await;

    let owner_header = owner_id.to_string();
    let (status, response) = req(
        app.clone(),
        "POST",
        "/notes",
        Some(&owner_header),
        Some(r#"{"title":"Shared note"}"#),
    )
    .await;
    assert_eq!(status, 201);
    let note_id = serde_json::from_slice::<serde_json::Value>(&response).unwrap()["id"]
        .as_i64()
        .unwrap();

    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO note_shares (note_id, user_id, permission) VALUES (?1, ?2, ?3)",
        params![note_id, writer_id, "write"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO note_shares (note_id, user_id, permission) VALUES (?1, ?2, ?3)",
        params![note_id, reader_id, "read"],
    )
    .unwrap();
    drop(conn);

    (app, owner_id, writer_id, reader_id, outsider_id, note_id)
}

#[tokio::test]
async fn outsider_get_is_not_found() {
    let (app, _, _, _, outsider_id, note_id) = setup().await;
    let outsider_id = outsider_id.to_string();

    let (status, body) = req(
        app,
        "GET",
        &format!("/notes/{note_id}"),
        Some(&outsider_id),
        None,
    )
    .await;
    assert_eq!(status, 404);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"],
        "note not found"
    );
}

#[tokio::test]
async fn write_share_can_patch_and_increments_version() {
    let (app, _, writer_id, _, _, note_id) = setup().await;
    let writer_id = writer_id.to_string();

    let (status, body) = req(
        app,
        "PATCH",
        &format!("/notes/{note_id}"),
        Some(&writer_id),
        Some(r#"{"title":"Writer update","version":1}"#),
    )
    .await;
    assert_eq!(status, 200);
    let note: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(note["version"], 2);
    assert_eq!(note["title"], "Writer update");
}

#[tokio::test]
async fn read_share_cannot_patch() {
    let (app, _, _, reader_id, _, note_id) = setup().await;
    let reader_id = reader_id.to_string();

    let (status, body) = req(
        app,
        "PATCH",
        &format!("/notes/{note_id}"),
        Some(&reader_id),
        Some(r#"{"title":"Reader update","version":1}"#),
    )
    .await;
    assert_eq!(status, 403);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"],
        "insufficient permissions"
    );
}

#[tokio::test]
async fn write_share_cannot_delete() {
    let (app, _, writer_id, _, _, note_id) = setup().await;
    let writer_id = writer_id.to_string();

    let (status, body) = req(
        app,
        "DELETE",
        &format!("/notes/{note_id}"),
        Some(&writer_id),
        None,
    )
    .await;
    assert_eq!(status, 403);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"],
        "insufficient permissions"
    );
}

#[tokio::test]
async fn owner_can_delete_note() {
    let (app, owner_id, _, _, _, note_id) = setup().await;
    let owner_id = owner_id.to_string();
    let path = format!("/notes/{note_id}");

    let (status, body) = req(app.clone(), "DELETE", &path, Some(&owner_id), None).await;
    assert_eq!(status, 204);
    assert!(body.is_empty());

    let (status, _) = req(app, "GET", &path, Some(&owner_id), None).await;
    assert_eq!(status, 404);
}
