use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn request(
    app: &Router,
    method: Method,
    path: &str,
    user_id: Option<i64>,
    payload: Option<Value>,
) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(user_id) = user_id {
        builder = builder.header("X-User-Id", user_id.to_string());
    }
    let request = match payload {
        Some(payload) => builder
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, body.to_vec())
}

fn value(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap()
}

async fn create_user(app: &Router, email: &str) -> i64 {
    let (status, body) = request(
        app,
        Method::POST,
        "/users",
        None,
        Some(json!({"name": "Test User", "email": email})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    value(&body)["id"].as_i64().unwrap()
}

struct Fixture {
    app: Router,
    owner_id: i64,
    recipient_id: i64,
    outsider_id: i64,
    note_id: i64,
}

async fn fixture() -> Fixture {
    let app = notes::router(notes::AppState::in_memory());
    let owner_id = create_user(&app, "owner@example.test").await;
    let recipient_id = create_user(&app, "recipient@example.test").await;
    let outsider_id = create_user(&app, "outsider@example.test").await;
    let (status, body) = request(
        &app,
        Method::POST,
        "/notes",
        Some(owner_id),
        Some(json!({"title": "Shared note"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let note_id = value(&body)["id"].as_i64().unwrap();
    Fixture {
        app,
        owner_id,
        recipient_id,
        outsider_id,
        note_id,
    }
}

async fn post_share(
    app: &Router,
    note_id: i64,
    owner_id: i64,
    user_id: i64,
    permission: &str,
) -> StatusCode {
    request(
        app,
        Method::POST,
        &format!("/notes/{note_id}/shares"),
        Some(owner_id),
        Some(json!({"user_id": user_id, "permission": permission})),
    )
    .await
    .0
}

#[tokio::test]
async fn write_share_allows_recipient_to_patch_note() {
    let fixture = fixture().await;
    let (status, body) = request(
        &fixture.app,
        Method::POST,
        &format!("/notes/{}/shares", fixture.note_id),
        Some(fixture.owner_id),
        Some(json!({"user_id": fixture.recipient_id, "permission": "write"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        value(&body),
        json!({"user_id": fixture.recipient_id, "permission": "write"})
    );

    let (status, body) = request(
        &fixture.app,
        Method::PATCH,
        &format!("/notes/{}", fixture.note_id),
        Some(fixture.recipient_id),
        Some(json!({"title": "Updated by recipient", "version": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value(&body)["version"], 2);
}

#[tokio::test]
async fn read_share_does_not_allow_recipient_to_patch_note() {
    let fixture = fixture().await;
    assert_eq!(
        post_share(
            &fixture.app,
            fixture.note_id,
            fixture.owner_id,
            fixture.recipient_id,
            "read",
        )
        .await,
        StatusCode::CREATED
    );
    let (status, _) = request(
        &fixture.app,
        Method::PATCH,
        &format!("/notes/{}", fixture.note_id),
        Some(fixture.recipient_id),
        Some(json!({"title": "Should be denied", "version": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn duplicate_share_returns_conflict() {
    let fixture = fixture().await;
    assert_eq!(
        post_share(
            &fixture.app,
            fixture.note_id,
            fixture.owner_id,
            fixture.recipient_id,
            "write",
        )
        .await,
        StatusCode::CREATED
    );
    assert_eq!(
        post_share(
            &fixture.app,
            fixture.note_id,
            fixture.owner_id,
            fixture.recipient_id,
            "read",
        )
        .await,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn invalid_permission_returns_bad_request() {
    let fixture = fixture().await;
    assert_eq!(
        post_share(
            &fixture.app,
            fixture.note_id,
            fixture.owner_id,
            fixture.recipient_id,
            "write ",
        )
        .await,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn sharing_with_note_owner_returns_conflict() {
    let fixture = fixture().await;
    assert_eq!(
        post_share(
            &fixture.app,
            fixture.note_id,
            fixture.owner_id,
            fixture.owner_id,
            "read",
        )
        .await,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn sharing_with_unknown_user_returns_not_found() {
    let fixture = fixture().await;
    let (status, body) = request(
        &fixture.app,
        Method::POST,
        &format!("/notes/{}/shares", fixture.note_id),
        Some(fixture.owner_id),
        Some(json!({"user_id": 999_999, "permission": "read"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(value(&body)["error"], "user not found");
}

#[tokio::test]
async fn only_note_readers_or_writers_receive_forbidden_on_share_post() {
    let fixture = fixture().await;
    assert_eq!(
        post_share(
            &fixture.app,
            fixture.note_id,
            fixture.owner_id,
            fixture.recipient_id,
            "read",
        )
        .await,
        StatusCode::CREATED
    );

    let (status, _) = request(
        &fixture.app,
        Method::POST,
        &format!("/notes/{}/shares", fixture.note_id),
        Some(fixture.recipient_id),
        Some(json!({"user_id": fixture.outsider_id, "permission": "write"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = request(
        &fixture.app,
        Method::POST,
        &format!("/notes/{}/shares", fixture.note_id),
        Some(fixture.outsider_id),
        Some(json!({"user_id": fixture.recipient_id, "permission": "write"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_share_is_idempotent_and_revokes_note_access() {
    let fixture = fixture().await;
    assert_eq!(
        post_share(
            &fixture.app,
            fixture.note_id,
            fixture.owner_id,
            fixture.recipient_id,
            "write",
        )
        .await,
        StatusCode::CREATED
    );
    let note_path = format!("/notes/{}", fixture.note_id);
    let (status, _) = request(
        &fixture.app,
        Method::GET,
        &note_path,
        Some(fixture.recipient_id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let share_path = format!("/notes/{}/shares/{}", fixture.note_id, fixture.recipient_id);
    let (status, body) = request(
        &fixture.app,
        Method::DELETE,
        &share_path,
        Some(fixture.owner_id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty());

    let (status, body) = request(
        &fixture.app,
        Method::DELETE,
        &share_path,
        Some(fixture.owner_id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty());

    let (status, _) = request(
        &fixture.app,
        Method::GET,
        &note_path,
        Some(fixture.recipient_id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn owner_can_list_shares_in_user_id_order_with_exact_response_shape() {
    let fixture = fixture().await;

    // Insert in descending id order so the response proves it is sorted.
    assert_eq!(
        post_share(
            &fixture.app,
            fixture.note_id,
            fixture.owner_id,
            fixture.outsider_id,
            "write",
        )
        .await,
        StatusCode::CREATED
    );
    assert_eq!(
        post_share(
            &fixture.app,
            fixture.note_id,
            fixture.owner_id,
            fixture.recipient_id,
            "read",
        )
        .await,
        StatusCode::CREATED
    );

    let (status, body) = request(
        &fixture.app,
        Method::GET,
        &format!("/notes/{}/shares", fixture.note_id),
        Some(fixture.owner_id),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        value(&body),
        json!([
            {"user_id": fixture.recipient_id, "permission": "read"},
            {"user_id": fixture.outsider_id, "permission": "write"}
        ])
    );
}

#[tokio::test]
async fn reader_cannot_list_note_shares() {
    let fixture = fixture().await;
    assert_eq!(
        post_share(
            &fixture.app,
            fixture.note_id,
            fixture.owner_id,
            fixture.recipient_id,
            "read",
        )
        .await,
        StatusCode::CREATED
    );

    let (status, body) = request(
        &fixture.app,
        Method::GET,
        &format!("/notes/{}/shares", fixture.note_id),
        Some(fixture.recipient_id),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(value(&body), json!({"error": "insufficient permissions"}));
}

#[tokio::test]
async fn outsider_cannot_list_note_shares() {
    let fixture = fixture().await;
    let (status, body) = request(
        &fixture.app,
        Method::GET,
        &format!("/notes/{}/shares", fixture.note_id),
        Some(fixture.outsider_id),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(value(&body), json!({"error": "note not found"}));
}

#[tokio::test]
async fn listing_shares_for_missing_note_returns_not_found() {
    let fixture = fixture().await;
    let (status, body) = request(
        &fixture.app,
        Method::GET,
        "/notes/999999/shares",
        Some(fixture.owner_id),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(value(&body), json!({"error": "note not found"}));
}
