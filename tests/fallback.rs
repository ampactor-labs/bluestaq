use axum::http::Request;
use tower::ServiceExt;

#[tokio::test]
async fn unknown_route_returns_json_not_found() {
    let app = notes::router(notes::AppState::in_memory());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/not-a-route")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body.as_ref(), br#"{"error":"not found"}"#);
}
