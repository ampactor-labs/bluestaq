use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

async fn req(app: axum::Router, method: &str, path: &str, body: Option<&str>) -> (u16, Vec<u8>) {
    req_with_actor(app, method, path, body, None).await
}

async fn req_as(
    app: axum::Router,
    method: &str,
    path: &str,
    body: Option<&str>,
    actor_id: i64,
) -> (u16, Vec<u8>) {
    req_with_actor(app, method, path, body, Some(actor_id)).await
}

async fn req_with_actor(
    app: axum::Router,
    method: &str,
    path: &str,
    body: Option<&str>,
    actor_id: Option<i64>,
) -> (u16, Vec<u8>) {
    let body = match body {
        Some(b) => Body::from(b.to_owned()),
        None => Body::empty(),
    };
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(axum::http::header::CONTENT_TYPE, "application/json");
    if let Some(actor_id) = actor_id {
        request = request.header("X-User-Id", actor_id.to_string());
    }
    let response = app.oneshot(request.body(body).unwrap()).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, bytes.to_vec())
}

fn app() -> axum::Router {
    notes::router(notes::AppState::in_memory())
}

/// Create a user, return its id.
async fn make_user(app: &axum::Router, name: &str, email: &str) -> i64 {
    let (status, body) = req(
        app.clone(),
        "POST",
        "/users",
        Some(&format!(r#"{{"name":"{name}","email":"{email}"}}"#)),
    )
    .await;
    assert_eq!(status, 201);
    serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn create_team_201_creator_is_sole_member() {
    let app = app();
    let creator = make_user(&app, "Morgan", "m@example.com").await;
    let (status, body) = req(
        app.clone(),
        "POST",
        "/teams",
        Some(&format!(
            r#"{{"name":"Space Force","created_by":{creator}}}"#
        )),
    )
    .await;
    assert_eq!(status, 201);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let team_id = v["id"].as_i64().unwrap();
    assert!(team_id > 0);
    assert_eq!(v["name"], "Space Force");
    assert!(!v["created_at"].as_str().unwrap().is_empty());

    let (status, body) = req(app, "GET", &format!("/teams/{team_id}/members"), None).await;
    assert_eq!(status, 200);
    let members: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["id"], creator);
}

#[tokio::test]
async fn create_team_missing_name_400() {
    let (status, body) = req(app(), "POST", "/teams", Some(r#"{"created_by":1}"#)).await;
    assert_eq!(status, 400);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "name is required");
}

#[tokio::test]
async fn create_team_creator_not_found_404() {
    let (status, body) = req(
        app(),
        "POST",
        "/teams",
        Some(r#"{"name":"T","created_by":9999}"#),
    )
    .await;
    assert_eq!(status, 404);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "creator not found");
}

#[tokio::test]
async fn get_team_200() {
    let app = app();
    let creator = make_user(&app, "Morgan", "m@example.com").await;
    let (status, body) = req(
        app.clone(),
        "POST",
        "/teams",
        Some(&format!(r#"{{"name":"T","created_by":{creator}}}"#)),
    )
    .await;
    assert_eq!(status, 201);
    let team_id = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    let (status, body) = req(app, "GET", &format!("/teams/{team_id}"), None).await;
    assert_eq!(status, 200);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["id"], team_id);
    assert_eq!(v["name"], "T");
}

#[tokio::test]
async fn get_team_9999_404() {
    let (status, body) = req(app(), "GET", "/teams/9999", None).await;
    assert_eq!(status, 404);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "team not found");
}

#[tokio::test]
async fn creator_adds_member_201_and_members_now_two() {
    let app = app();
    let creator = make_user(&app, "A", "a@example.com").await;
    let (status, body) = req(
        app.clone(),
        "POST",
        "/teams",
        Some(&format!(r#"{{"name":"T","created_by":{creator}}}"#)),
    )
    .await;
    assert_eq!(status, 201);
    let team_id = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();
    let other = make_user(&app, "B", "b@example.com").await;

    let (status, body) = req_as(
        app.clone(),
        "POST",
        &format!("/teams/{team_id}/members"),
        Some(&format!(r#"{{"user_id":{other}}}"#)),
        creator,
    )
    .await;
    assert_eq!(status, 201);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["team_id"], team_id);
    assert_eq!(v["user_id"], other);

    let (status, body) = req(app, "GET", &format!("/teams/{team_id}/members"), None).await;
    assert_eq!(status, 200);
    let members: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(members.len(), 2);
}

#[tokio::test]
async fn add_member_duplicate_409() {
    let app = app();
    let creator = make_user(&app, "A", "a@example.com").await;
    let (status, body) = req(
        app.clone(),
        "POST",
        "/teams",
        Some(&format!(r#"{{"name":"T","created_by":{creator}}}"#)),
    )
    .await;
    assert_eq!(status, 201);
    let team_id = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    // Creator is already a member, so adding them again conflicts.
    let (status, body) = req_as(
        app,
        "POST",
        &format!("/teams/{team_id}/members"),
        Some(&format!(r#"{{"user_id":{creator}}}"#)),
        creator,
    )
    .await;
    assert_eq!(status, 409);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "already a member");
}

#[tokio::test]
async fn add_member_bad_team_404() {
    let app = app();
    let actor = make_user(&app, "A", "a@example.com").await;
    let (status, body) = req_as(
        app,
        "POST",
        "/teams/9999/members",
        Some(r#"{"user_id":1}"#),
        actor,
    )
    .await;
    assert_eq!(status, 404);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "team not found");
}

#[tokio::test]
async fn add_member_bad_user_404() {
    let app = app();
    let creator = make_user(&app, "A", "a@example.com").await;
    let (status, body) = req(
        app.clone(),
        "POST",
        "/teams",
        Some(&format!(r#"{{"name":"T","created_by":{creator}}}"#)),
    )
    .await;
    assert_eq!(status, 201);
    let team_id = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    let (status, body) = req_as(
        app,
        "POST",
        &format!("/teams/{team_id}/members"),
        Some(r#"{"user_id":9999}"#),
        creator,
    )
    .await;
    assert_eq!(status, 404);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "user not found");
}

#[tokio::test]
async fn add_member_without_identity_401() {
    let app = app();
    let creator = make_user(&app, "A", "a@example.com").await;
    let target = make_user(&app, "B", "b@example.com").await;
    let (status, body) = req(
        app.clone(),
        "POST",
        "/teams",
        Some(&format!(r#"{{"name":"T","created_by":{creator}}}"#)),
    )
    .await;
    assert_eq!(status, 201);
    let team_id = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    let (status, body) = req(
        app,
        "POST",
        &format!("/teams/{team_id}/members"),
        Some(&format!(r#"{{"user_id":{target}}}"#)),
    )
    .await;
    assert_eq!(status, 401);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "missing X-User-Id header");
}

#[tokio::test]
async fn add_member_nonmember_forbidden_403() {
    let app = app();
    let creator = make_user(&app, "A", "a@example.com").await;
    let outsider = make_user(&app, "B", "b@example.com").await;
    let target = make_user(&app, "C", "c@example.com").await;
    let (status, body) = req(
        app.clone(),
        "POST",
        "/teams",
        Some(&format!(r#"{{"name":"T","created_by":{creator}}}"#)),
    )
    .await;
    assert_eq!(status, 201);
    let team_id = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    let (status, body) = req_as(
        app,
        "POST",
        &format!("/teams/{team_id}/members"),
        Some(&format!(r#"{{"user_id":{target}}}"#)),
        outsider,
    )
    .await;
    assert_eq!(status, 403);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "not a member of this team");
}
