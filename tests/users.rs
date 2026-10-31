use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

async fn req(
    app: axum::Router,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> (u16, Vec<u8>) {
    let body = match body {
        Some(b) => Body::from(b.to_owned()),
        None => Body::empty(),
    };
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(body)
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, bytes.to_vec())
}

fn app() -> axum::Router {
    notes::router(notes::AppState::in_memory())
}

#[tokio::test]
async fn create_user_201_echo() {
    let (status, body) = req(
        app(),
        "POST",
        "/users",
        Some(r#"{"name":"Morgan","email":"morgan@example.com"}"#),
    )
    .await;
    assert_eq!(status, 201);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["id"].as_i64().unwrap() > 0);
    assert_eq!(v["name"], "Morgan");
    assert_eq!(v["email"], "morgan@example.com");
    assert!(!v["created_at"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn create_user_missing_name_400() {
    let (status, body) = req(app(), "POST", "/users", Some(r#"{"email":"a@b.c"}"#)).await;
    assert_eq!(status, 400);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "name and email are required");
}

#[tokio::test]
async fn create_user_empty_email_400() {
    let (status, body) = req(
        app(),
        "POST",
        "/users",
        Some(r#"{"name":"Morgan","email":"   "}"#),
    )
    .await;
    assert_eq!(status, 400);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "name and email are required");
}

#[tokio::test]
async fn create_user_duplicate_email_409() {
    let app = app();
    let (status, _) = req(app.clone(), "POST", "/users", Some(r#"{"name":"A","email":"dup@example.com"}"#)).await;
    assert_eq!(status, 201);
    let (status, body) = req(app, "POST", "/users", Some(r#"{"name":"B","email":"dup@example.com"}"#)).await;
    assert_eq!(status, 409);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "email already registered");
}

#[tokio::test]
async fn create_user_case_insensitive_email_duplicate_409() {
    let app = app();
    let (status, body) = req(
        app.clone(),
        "POST",
        "/users",
        Some(r#"{"name":"Uppercase","email":"User@X.com"}"#),
    )
    .await;
    assert_eq!(status, 201);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["email"], "user@x.com");

    let (status, body) = req(
        app,
        "POST",
        "/users",
        Some(r#"{"name":"Lowercase","email":"user@x.com"}"#),
    )
    .await;
    assert_eq!(status, 409);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "email already registered");
}

#[tokio::test]
async fn get_user_200() {
    let app = app();
    let (status, body) = req(app.clone(), "POST", "/users", Some(r#"{"name":"Morgan","email":"m@example.com"}"#)).await;
    assert_eq!(status, 201);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = v["id"].as_i64().unwrap();

    let (status, body) = req(app, "GET", &format!("/users/{id}"), None).await;
    assert_eq!(status, 200);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["id"], id);
    assert_eq!(v["name"], "Morgan");
    assert_eq!(v["email"], "m@example.com");
}

#[tokio::test]
async fn get_user_9999_404() {
    let (status, body) = req(app(), "GET", "/users/9999", None).await;
    assert_eq!(status, 404);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "user not found");
}

#[tokio::test]
async fn get_user_non_numeric_id_404() {
    let (status, body) = req(app(), "GET", "/users/abc", None).await;
    assert_eq!(status, 404);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "not found");
}
