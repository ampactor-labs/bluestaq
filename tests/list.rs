use std::collections::HashSet;
use std::time::Duration;

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

async fn make_team(app: &axum::Router, creator_id: i64) -> i64 {
    let body = format!(r#"{{"name":"Team","created_by":{creator_id}}}"#);
    let (status, response) = req(app.clone(), "POST", "/teams", None, Some(&body)).await;
    assert_eq!(status, 201);
    serde_json::from_slice::<serde_json::Value>(&response).unwrap()["id"]
        .as_i64()
        .unwrap()
}

async fn add_member(app: &axum::Router, team_id: i64, user_id: i64) {
    let body = format!(r#"{{"user_id":{user_id}}}"#);
    let (status, _) = req(
        app.clone(),
        "POST",
        &format!("/teams/{team_id}/members"),
        None,
        Some(&body),
    )
    .await;
    assert_eq!(status, 201);
}

async fn make_note(app: &axum::Router, user_id: i64, title: &str, team_id: Option<i64>) -> i64 {
    let user_header = user_id.to_string();
    let body = match team_id {
        Some(team_id) => format!(r#"{{"title":"{title}","team_id":{team_id}}}"#),
        None => format!(r#"{{"title":"{title}"}}"#),
    };
    let (status, response) = req(
        app.clone(),
        "POST",
        "/notes",
        Some(&user_header),
        Some(&body),
    )
    .await;
    assert_eq!(status, 201);
    serde_json::from_slice::<serde_json::Value>(&response).unwrap()["id"]
        .as_i64()
        .unwrap()
}

struct Fixture {
    app: axum::Router,
    owner_id: i64,
    alpha_id: i64,
    beta_id: i64,
    gamma_id: i64,
    delta_id: i64,
}

async fn setup() -> Fixture {
    let state = notes::AppState::in_memory();
    let app = notes::router(state.clone());
    let owner_id = make_user(&app, "A", "a@example.com").await;
    let other_id = make_user(&app, "B", "b@example.com").await;

    let alpha_id = make_note(&app, owner_id, "Alpha one", None).await;
    let beta_id = make_note(&app, owner_id, "Beta two", None).await;
    let gamma_id = make_note(&app, other_id, "Gamma", None).await;

    let team_id = make_team(&app, owner_id).await;
    add_member(&app, team_id, other_id).await;
    let delta_id = make_note(&app, other_id, "Delta", Some(team_id)).await;

    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO note_shares (note_id, user_id, permission) VALUES (?1, ?2, 'read')",
        params![gamma_id, owner_id],
    )
    .unwrap();

    Fixture {
        app,
        owner_id,
        alpha_id,
        beta_id,
        gamma_id,
        delta_id,
    }
}

fn note_ids(body: &[u8]) -> Vec<i64> {
    serde_json::from_slice::<Vec<serde_json::Value>>(body)
        .unwrap()
        .iter()
        .map(|note| note["id"].as_i64().unwrap())
        .collect()
}

#[tokio::test]
async fn list_returns_accessible_notes_once_each() {
    let fixture = setup().await;
    let owner_id = fixture.owner_id.to_string();

    let (status, body) = req(fixture.app, "GET", "/notes", Some(&owner_id), None).await;
    assert_eq!(status, 200);
    let ids = note_ids(&body);
    assert_eq!(ids.len(), 4);
    assert_eq!(
        ids.into_iter().collect::<HashSet<_>>(),
        HashSet::from([
            fixture.alpha_id,
            fixture.beta_id,
            fixture.gamma_id,
            fixture.delta_id,
        ])
    );
}

#[tokio::test]
async fn patched_note_moves_to_front_of_list() {
    let fixture = setup().await;
    std::thread::sleep(Duration::from_millis(1100));
    let owner_id = fixture.owner_id.to_string();
    let patch = format!(r#"{{"title":"Alpha updated","version":1}}"#);
    let (status, _) = req(
        fixture.app.clone(),
        "PATCH",
        &format!("/notes/{}", fixture.alpha_id),
        Some(&owner_id),
        Some(&patch),
    )
    .await;
    assert_eq!(status, 200);

    let (status, body) = req(fixture.app, "GET", "/notes", Some(&owner_id), None).await;
    assert_eq!(status, 200);
    assert_eq!(note_ids(&body)[0], fixture.alpha_id);
}

#[tokio::test]
async fn search_is_ascii_case_insensitive() {
    let fixture = setup().await;
    let owner_id = fixture.owner_id.to_string();

    let (status, body) = req(fixture.app, "GET", "/notes?q=ALPH", Some(&owner_id), None).await;
    assert_eq!(status, 200);
    assert_eq!(note_ids(&body), vec![fixture.alpha_id]);
}

#[tokio::test]
async fn search_excludes_owned_nonmatching_notes() {
    let fixture = setup().await;
    let owner_id = fixture.owner_id.to_string();

    let (status, body) = req(fixture.app, "GET", "/notes?q=zzz", Some(&owner_id), None).await;
    assert_eq!(status, 200);
    assert!(note_ids(&body).is_empty());
}

#[tokio::test]
async fn search_excludes_shared_nonmatching_notes() {
    let fixture = setup().await;
    let owner_id = fixture.owner_id.to_string();

    let (status, body) = req(fixture.app, "GET", "/notes?q=alp", Some(&owner_id), None).await;
    assert_eq!(status, 200);
    assert_eq!(note_ids(&body), vec![fixture.alpha_id]);
}

#[tokio::test]
async fn concurrent_patches_with_same_version_have_one_winner() {
    let fixture = setup().await;
    let path = format!("/notes/{}", fixture.alpha_id);
    let app_a = fixture.app.clone();
    let app_b = fixture.app;
    let owner_a = fixture.owner_id.to_string();
    let owner_b = owner_a.clone();

    let first = tokio::spawn(async move {
        req(
            app_a,
            "PATCH",
            &path,
            Some(&owner_a),
            Some(r#"{"title":"First winner","version":1}"#),
        )
        .await
        .0
    });
    let path = format!("/notes/{}", fixture.alpha_id);
    let second = tokio::spawn(async move {
        req(
            app_b,
            "PATCH",
            &path,
            Some(&owner_b),
            Some(r#"{"title":"Second winner","version":1}"#),
        )
        .await
        .0
    });

    let statuses = [first.await.unwrap(), second.await.unwrap()];
    assert!(statuses.contains(&200));
    assert!(statuses.contains(&409));
    assert_ne!(statuses[0], statuses[1]);
}
