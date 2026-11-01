use axum::body::Body;
use axum::http::Request;
use notes::db;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

struct TempDatabase(PathBuf);

impl TempDatabase {
    fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before Unix epoch")
            .as_nanos();
        let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
        let filename = format!(
            "notes-fault-{}-{timestamp}-{unique_id}.db",
            std::process::id()
        );
        Self(std::env::temp_dir().join(filename))
    }

    fn as_string(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for TempDatabase {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

async fn req_as(
    app: axum::Router,
    method: &str,
    path: &str,
    body: Option<&str>,
    actor_id: i64,
) -> (u16, Vec<u8>) {
    let body = match body {
        Some(b) => Body::from(b.to_owned()),
        None => Body::empty(),
    };
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .header("X-User-Id", actor_id.to_string());
    let response = app.oneshot(request.body(body).unwrap()).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, bytes.to_vec())
}

async fn make_user(app: &axum::Router, name: &str, email: &str) -> i64 {
    let request = Request::builder()
        .method("POST")
        .uri("/users")
        .header(axum::http::header::CONTENT_TYPE, "application/json");
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(format!(
                    r#"{{"name":"{name}","email":"{email}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(status, 201);
    serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap()
}

// The access check (Box 7.2) promises that a storage failure surfaces as a
// 500 rather than failing closed to a 404. This test breaks the share query
// for real: a second connection to the same file-backed database drops the
// note_shares table, so the next non-owner access check errors in SQLite.
#[tokio::test]
async fn access_check_storage_failure_returns_500() {
    let temp = TempDatabase::new();
    let path = temp.as_string();

    let conn = db::new_file(&path).expect("open file database");
    let app = notes::router(notes::AppState {
        db: Arc::new(Mutex::new(conn)),
    });

    let owner = make_user(&app, "Owner", "owner@example.com").await;
    let outsider = make_user(&app, "Outsider", "outsider@example.com").await;

    let (status, body) = req_as(
        app.clone(),
        "POST",
        "/notes",
        Some(r#"{"title":"T","body":"B"}"#),
        owner,
    )
    .await;
    assert_eq!(status, 201);
    let note_id = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    // Baseline: with storage intact the outsider simply has no access.
    let (status, _) = req_as(app.clone(), "GET", &format!("/notes/{note_id}"), None, outsider).await;
    assert_eq!(status, 404);

    // Break the share query out from under the running service.
    let saboteur = db::new_file(&path).expect("open second connection");
    saboteur
        .execute_batch("DROP TABLE note_shares")
        .expect("drop note_shares");

    // The access check now errors: it must surface as a 500 JSON error,
    // not collapse into the no-access 404.
    let (status, body) = req_as(app.clone(), "GET", &format!("/notes/{note_id}"), None, outsider).await;
    assert_eq!(status, 500);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["error"].as_str().is_some_and(|s| !s.is_empty()));

    // The owner path short-circuits before the share query and still works.
    let (status, _) = req_as(app, "GET", &format!("/notes/{note_id}"), None, owner).await;
    assert_eq!(status, 200);
}
