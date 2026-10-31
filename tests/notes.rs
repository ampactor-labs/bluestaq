use axum::body::Body;
use axum::http::Request;
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

async fn make_team(app: &axum::Router, creator_id: i64) -> i64 {
    let body = format!(r#"{{"name":"Team","created_by":{creator_id}}}"#);
    let (status, response) = req(app.clone(), "POST", "/teams", None, Some(&body)).await;
    assert_eq!(status, 201);
    serde_json::from_slice::<serde_json::Value>(&response).unwrap()["id"]
        .as_i64()
        .unwrap()
}

async fn setup() -> (axum::Router, i64, i64, i64) {
    let app = notes::router(notes::AppState::in_memory());
    let owner_id = make_user(&app, "Owner", "owner@example.com").await;
    let other_id = make_user(&app, "Other", "other@example.com").await;
    let team_id = make_team(&app, owner_id).await;
    (app, owner_id, other_id, team_id)
}

async fn make_note(app: &axum::Router, owner_id: i64) -> i64 {
    let owner_id = owner_id.to_string();
    let (status, response) = req(
        app.clone(),
        "POST",
        "/notes",
        Some(&owner_id),
        Some(r#"{"title":"First note"}"#),
    )
    .await;
    assert_eq!(status, 201);
    serde_json::from_slice::<serde_json::Value>(&response).unwrap()["id"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn create_note_returns_201_at_version_one() {
    let (app, owner_id, _, _) = setup().await;
    let owner_id = owner_id.to_string();
    let (status, body) = req(
        app,
        "POST",
        "/notes",
        Some(&owner_id),
        Some(r#"{"title":"First note"}"#),
    )
    .await;

    assert_eq!(status, 201);
    let note: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(note["id"].as_i64().unwrap() > 0);
    assert_eq!(note["title"], "First note");
    assert_eq!(note["body"], "");
    assert_eq!(note["version"], 1);
    assert!(note["team_id"].is_null());
}

#[tokio::test]
async fn identity_rejections_have_distinct_401_messages() {
    let (app, _, _, _) = setup().await;

    let (status, body) = req(app.clone(), "GET", "/notes/1", None, None).await;
    assert_eq!(status, 401);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"],
        "missing X-User-Id header"
    );

    let (status, body) = req(app.clone(), "GET", "/notes/1", Some("not-an-id"), None).await;
    assert_eq!(status, 401);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"],
        "invalid X-User-Id header"
    );

    let (status, body) = req(app, "GET", "/notes/1", Some("9999"), None).await;
    assert_eq!(status, 401);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"],
        "unknown user"
    );
}

#[tokio::test]
async fn non_owner_get_is_not_found() {
    let (app, owner_id, other_id, _) = setup().await;
    let note_id = make_note(&app, owner_id).await;
    let other_id = other_id.to_string();

    let (status, body) = req(
        app,
        "GET",
        &format!("/notes/{note_id}"),
        Some(&other_id),
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
async fn patch_updates_note_and_increments_version() {
    let (app, owner_id, _, _) = setup().await;
    let note_id = make_note(&app, owner_id).await;
    let owner_id = owner_id.to_string();
    let path = format!("/notes/{note_id}");

    let (status, body) = req(
        app.clone(),
        "PATCH",
        &path,
        Some(&owner_id),
        Some(r#"{"title":"Updated note","version":1}"#),
    )
    .await;
    assert_eq!(status, 200);
    let note: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(note["version"], 2);
    assert_eq!(note["title"], "Updated note");

    let (status, body) = req(app, "GET", &path, Some(&owner_id), None).await;
    assert_eq!(status, 200);
    let note: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(note["title"], "Updated note");
}

#[tokio::test]
async fn patch_with_stale_version_conflicts() {
    let (app, owner_id, _, _) = setup().await;
    let note_id = make_note(&app, owner_id).await;
    let owner_id = owner_id.to_string();
    let (status, body) = req(
        app,
        "PATCH",
        &format!("/notes/{note_id}"),
        Some(&owner_id),
        Some(r#"{"title":"Stale update","version":0}"#),
    )
    .await;
    assert_eq!(status, 409);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"],
        "version conflict"
    );
}

#[tokio::test]
async fn patch_without_version_is_bad_request() {
    let (app, owner_id, _, _) = setup().await;
    let note_id = make_note(&app, owner_id).await;
    let owner_id = owner_id.to_string();
    let (status, body) = req(
        app,
        "PATCH",
        &format!("/notes/{note_id}"),
        Some(&owner_id),
        Some(r#"{"title":"Missing version"}"#),
    )
    .await;
    assert_eq!(status, 400);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"],
        "version is required"
    );
}

#[tokio::test]
async fn owner_can_delete_note_with_no_content_response() {
    let (app, owner_id, _, _) = setup().await;
    let note_id = make_note(&app, owner_id).await;
    let owner_id = owner_id.to_string();
    let path = format!("/notes/{note_id}");

    let (status, body) = req(app.clone(), "DELETE", &path, Some(&owner_id), None).await;
    assert_eq!(status, 204);
    assert!(body.is_empty());

    let (status, _) = req(app, "GET", &path, Some(&owner_id), None).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn non_member_cannot_create_note_in_team() {
    let (app, _, other_id, team_id) = setup().await;
    let other_id = other_id.to_string();
    let body = format!(r#"{{"title":"Team note","team_id":{team_id}}}"#);
    let (status, response) = req(app, "POST", "/notes", Some(&other_id), Some(&body)).await;
    assert_eq!(status, 403);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&response).unwrap()["error"],
        "not a member of that team"
    );
}
