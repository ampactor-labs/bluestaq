use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

async fn request(
    app: axum::Router,
    method: &str,
    path: &str,
    user_id: Option<i64>,
    body: Option<String>,
) -> (u16, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(axum::http::header::CONTENT_TYPE, "application/json");
    if let Some(user_id) = user_id {
        builder = builder.header("x-user-id", user_id.to_string());
    }
    let body = body.map_or_else(Body::empty, Body::from);
    let response = app.oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, bytes.to_vec())
}

async fn create_user(app: &axum::Router) -> i64 {
    let body = serde_json::json!({
        "name": "Search user",
        "email": "search@example.com"
    })
    .to_string();
    let (status, response) = request(app.clone(), "POST", "/users", None, Some(body)).await;
    assert_eq!(status, 201);
    serde_json::from_slice::<serde_json::Value>(&response).unwrap()["id"]
        .as_i64()
        .unwrap()
}

async fn create_note(app: &axum::Router, user_id: i64, title: &str) -> i64 {
    let body = serde_json::json!({ "title": title }).to_string();
    let (status, response) =
        request(app.clone(), "POST", "/notes", Some(user_id), Some(body)).await;
    assert_eq!(status, 201);
    serde_json::from_slice::<serde_json::Value>(&response).unwrap()["id"]
        .as_i64()
        .unwrap()
}

fn note_ids(body: &[u8]) -> Vec<i64> {
    let mut ids: Vec<i64> = serde_json::from_slice::<Vec<serde_json::Value>>(body)
        .unwrap()
        .into_iter()
        .map(|note| note["id"].as_i64().unwrap())
        .collect();
    ids.sort_unstable();
    ids
}

#[tokio::test]
async fn search_treats_like_metacharacters_literally_and_empty_query_as_absent() {
    let app = notes::router(notes::AppState::in_memory());
    let user_id = create_user(&app).await;
    let percent_id = create_note(&app, user_id, "100% complete").await;
    let underscore_id = create_note(&app, user_id, "a_b report").await;
    let backslash_id = create_note(&app, user_id, r"folder\file").await;
    let ordinary_id = create_note(&app, user_id, "plain text").await;

    for (query, expected_id) in [
        ("%25", percent_id),
        ("%5F", underscore_id),
        ("%5C", backslash_id),
    ] {
        let (status, response) = request(
            app.clone(),
            "GET",
            &format!("/notes?q={query}"),
            Some(user_id),
            None,
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(note_ids(&response), vec![expected_id], "query {query}");
    }

    let (all_status, all_response) =
        request(app.clone(), "GET", "/notes", Some(user_id), None).await;
    assert_eq!(all_status, 200);
    let (empty_status, empty_response) =
        request(app, "GET", "/notes?q=", Some(user_id), None).await;
    assert_eq!(empty_status, 200);
    let all_ids = note_ids(&all_response);
    assert_eq!(
        all_ids,
        vec![percent_id, underscore_id, backslash_id, ordinary_id]
    );
    assert_eq!(note_ids(&empty_response), all_ids);
}
