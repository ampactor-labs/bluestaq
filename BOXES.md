# Assessment boxes

Planner (Qwen3.8-27B session one, Build mode restricted to this file) writes boxes here. Writer (Qwen3.8-27B session two, fresh, Build mode) implements one gated box at a time and appends its receipt under the box; its first tool call on any box is a read of this file. Morgan gates every box. Rules: AGENTS.md [T1]-[T8].

Box format:

## Box n: [name]
- Files touched:
- Decisions made (signatures, edge cases, error behavior):
- First action:
- Step cap:
- Verification command:
- Gate: [ ] approved by Morgan

Receipt (writer appends):
- Files changed:
- Verification output:
- Ambiguity hit:

---

# Global decisions (made 2026-10-31, do not re-litigate)

- Language/scope: Rust, FULL scope. One crate at repo root, package name `notes`, lib target `notes`.
- Stack: axum + tokio + serde/serde_json + rusqlite (bundled). State = `Arc<Mutex<rusqlite::Connection>>` inside `AppState`. No sqlx, no ORM, no auth crates, no uuid, no chrono. Clone liberally; one ownership pattern only.
- **Pinned (verified resolvable against crates.io index 2026-10-31): axum 0.8.9, rusqlite 0.41.1, tokio 1.54.1, serde 1.0.229 (derive), serde_json 1.0.154, tower 0.5.3 (dev, util). If a pin fails to resolve, take the newest that resolves and say so in the receipt. axum 0.8.x path-parameter syntax: `{id}` (curly braces), used consistently in every box.**
- Timestamps: SQLite `datetime('now')` via column defaults.
- Identity: `X-User-Id` header on all /notes routes. Missing, non-integer, or nonexistent user = 401.
- Permission matrix: owner = read/update/delete/share. Write share = read+update. Read share = read. Team member (note's team_id is one of the user's teams) = read only. No access = 404. Has access but lacks the specific right = 403.
- Error shape everywhere: JSON `{"error": "..."}`.
- Tests: integration tests through the router (tower oneshot) against a fresh in-memory DB per test app.
- Port: 127.0.0.1:8080, fixed, no env override (boring; note in README).
- Listener (ratified deviation, 2026-10-31): `main.rs` binds `tokio::net::TcpListener::bind("127.0.0.1:8080").await` and calls `axum::serve(listener, notes::router(...))` — axum 0.8.9 has no `axum::net`/`from_std`; no box may reference it. Writer standing rule: if a box names an API the pinned version lacks and exactly one canonical replacement preserves the box's behavior, implement it and disclose in the receipt; two or more replacements that behave differently is a stop.

## Box 1: Cargo skeleton, /health, one passing test
- Files touched: `Cargo.toml` (new), `src/lib.rs` (new), `src/main.rs` (new), `tests/health.rs` (new), `.gitignore` (new), `README.md` (replace 1-line stub). Nothing else. No commit (Morgan commits).
- Decisions made:
  - `Cargo.toml` exactly:
    ```toml
    [package]
    name = "notes"
    version = "0.1.0"
    edition = "2021"

    [dependencies]
    axum = "0.8.9"
    rusqlite = { version = "0.41.1", features = ["bundled"] }
    serde = { version = "1.0.229", features = ["derive"] }
    serde_json = "1.0.154"
    tokio = { version = "1.54.1", features = ["macros", "rt-multi-thread", "net", "signal"] }

    [dev-dependencies]
    tower = { version = "0.5.3", features = ["util"] }
    ```
    Full dep list lands now so dependency resolution is proven in Box 1; unused-in-Box-1 deps are fine (no warnings for them).
  - `src/lib.rs`:
    ```rust
    pub fn router() -> axum::Router {
        axum::Router::new().route("/health", get(health))
    }
    pub(crate) async fn health() -> Json<serde_json::Value> {
        Json(serde_json::json!({"status": "ok"}))
    }
    ```
    (import `axum::{Json, routing::get}`; lib name is `notes`, tests call `notes::router()`.)
  - `src/main.rs`: `#[tokio::main]` — build std `TcpListener` on `127.0.0.1:8080`, `axum::serve(TcpListener::from_std(listener), notes::router())`. On bind failure: `eprintln!` the error and `std::process::exit(1)`; no panic path.
  - `tests/health.rs`: one `#[tokio::test]` — `notes::router().oneshot(Req: GET /health)` via `tower::ServiceExt::oneshot`; assert status 200 and body bytes exactly `{"status":"ok"}` (use `axum::body::to_bytes`).
  - `.gitignore`: exactly `/target` and `*.db`.
  - `README.md` skeleton: title "Notes — shared team note-taking REST service", one-line description (Rust, axum, SQLite), "Status: WIP — endpoint list, curl examples, design choices, and more-time notes land in Box 7." Placeholder headings only: `## Run`, `## Endpoints`, `## Design choices`, `## With more time` (empty bodies).
  - Edge: no other routes exist yet; axum's default 404 text is acceptable until Box 7.
- First action: write `Cargo.toml`.
- Step cap: 7 (Cargo.toml, lib.rs, main.rs, tests/health.rs, .gitignore, README.md, `cargo test`).
- Verification command: `cargo test` in repo root — expect `test result: ok. 1 passed`.
- Gate: [ ] approved by Morgan

Receipt (writer appends):
- Files changed: `Cargo.toml` (new, verbatim per box), `src/lib.rs` (new, verbatim per box + `use axum::{Json, routing::get};`), `src/main.rs` (new), `tests/health.rs` (new), `.gitignore` (new, exactly `/target` + `*.db`), `README.md` (replaced 1-line stub with box's skeleton).
- Verification output: `cargo test` in repo root:
  ```
  Running tests/health.rs (target/debug/deps/health-e8993e72a17ca554)
  running 1 test
  test health_returns_ok ... ok
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  ```
  (lib/bin unit-test harnesses: 0 tests, ok; doc-tests: 0, ok.)
- Ambiguity hit: one API deviation, forced by the pinned dependency, disclosed per orders: the box said "build std `TcpListener` ... `axum::serve(TcpListener::from_std(listener), ...)`", but axum 0.8.9 has no `axum::net`/`from_std` and `std::net::TcpListener` does not implement `axum::serve::Listener` (compile errors E0277/E0433 confirmed). `src/main.rs` therefore uses `tokio::net::TcpListener::bind("127.0.0.1:8080").await` (tokio `net` feature already in the box's Cargo.toml), same bind address, same eprintln + `exit(1)` on bind failure, plus eprintln + `exit(1)` on `axum::serve` error (no panic path, per box). Suggest planner notes this for future boxes.

---

## Box 2: Schema, DB layer, AppState, error type
- Files touched: `src/db.rs` (new), `src/error.rs` (new), `src/lib.rs` (modify: add `AppState`, change `router` signature), `tests/health.rs` (modify: build `AppState`).
- Decisions made:
  - `src/db.rs`:
    - `pub const SCHEMA: &str` = the global-decisions schema verbatim: `PRAGMA foreign_keys = ON;` then `users(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, email TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL DEFAULT (datetime('now')))`; `teams(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')))`; `team_members(team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, PRIMARY KEY(team_id,user_id))`; `notes(id INTEGER PRIMARY KEY AUTOINCREMENT, owner_id INTEGER NOT NULL REFERENCES users(id), team_id INTEGER REFERENCES teams(id), title TEXT NOT NULL, body TEXT NOT NULL DEFAULT '', version INTEGER NOT NULL DEFAULT 1, created_at/updated_at TEXT NOT NULL DEFAULT (datetime('now')))`; `note_shares(note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE, user_id INTEGER NOT NULL REFERENCES users(id), permission TEXT NOT NULL CHECK(permission IN ('read','write')), PRIMARY KEY(note_id,user_id))`.
    - `pub fn new_in_memory() -> rusqlite::Connection`: open `:memory:`, `pragma_update(None, "foreign_keys", "ON")`, execute the CREATE TABLEs from `SCHEMA` (PRAGMA set via pragma_update, DDL via `execute_batch`).
    - `pub fn new_file(path: &str) -> rusqlite::Result<Connection>`: open path, same pragma + DDL. Error → `AppError::Internal`.
    - Unit test inside db.rs: `new_in_memory()` yields exactly the 5 tables in `sqlite_master`.
  - `src/error.rs`: `pub enum AppError { BadRequest(String), Unauthorized, NotFound(String), Conflict(String), Internal(String) }` with `impl IntoResponse`: 400/401/404/409/500, each body JSON `{"error": "<msg>"}`.
  - JSON body handling (used from Box 3 on): handlers take `json: Result<axum::Json<T>, axum::JsonRejection>`; any rejection maps to `AppError::BadRequest("malformed JSON body".into())`. Decided here so Box 3+ is uniform.
  - `src/lib.rs`: `#[derive(Clone)] pub struct AppState { pub db: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>> }` with `impl AppState { pub fn in_memory() -> Self }` (wraps `db::new_in_memory`); `pub fn router(state: AppState) -> axum::Router` — `/health` unchanged, router built with `.with_state(state)`.
  - `tests/health.rs`: build `AppState::in_memory()`, `notes::router(state).oneshot(...)`, same asserts as Box 1.
  - Edge: note_shares cascade with their note; team_members cascade with team or user (decided by amendment 2026-10-31: deleting a shared note must not 500). notes.owner_id and notes.team_id stay default NO ACTION; record as assumption in Box 7.
- First action: write `src/db.rs` (SCHEMA const first).
- Step cap: 5 (db.rs, error.rs, lib.rs edit, health.rs edit, `cargo test`).
- Verification command: `cargo test` — expect 2 passed (health + schema unit test).
- Gate: [ ] approved by Morgan (draft — finalized when Box 1 receipt is reviewed)

Receipt (writer appends):
- Files changed:
- Verification output:
- Ambiguity hit:

---

## Box 3: Users, teams, membership + tests
- Files touched: `src/users.rs` (new), `src/teams.rs` (new), `src/db.rs` (add `is_unique`), `src/lib.rs` (register routes), `tests/users.rs` (new), `tests/teams.rs` (new).
- Decisions made:
  - Path-param convention (all id routes, this box onward): `{id}` extracted as `String` via `Path<String>`, parsed to `i64` by hand; parse failure → 404 `{"error":"not found"}`. This keeps the JSON error shape uniform (no axum rejection text) and treats non-numeric ids as nonexistent.
  - Response shapes (serde derive): `users.rs`: `pub struct UserOut { id: i64, name: String, email: String, created_at: String }`. `teams.rs`: `pub struct TeamOut { id: i64, name: String, created_at: String }`.
  - Handler signature pattern (uniform): `pub async fn create_user(State(s): State<AppState>, json: Result<axum::Json<NewUser>, axum::JsonRejection>) -> Result<(StatusCode, Json<UserOut>), AppError>`. `JsonRejection` → `AppError::BadRequest("malformed JSON body")` per Box 2.
  - `src/db.rs`: add `pub fn is_unique(e: &rusqlite::Error) -> bool { matches!(e, rusqlite::Error::UniqueViolation(_)) }`.
  - POST /users: body `{name?, email?}` as `Option`s. Either missing, or empty after `trim()` → 400 `"name and email are required"`. Store trimmed values. UniqueViolation → 409 `"email already registered"`. Success 201 + `UserOut`.
  - GET /users/{id}: 200 `UserOut` | 404 `"user not found"`.
  - POST /teams: body `{name?, created_by?}`; name missing/empty-after-trim → 400 `"name is required"`; created_by missing → 400 `"created_by is required"`; creator not a row in `users` → 404 `"creator not found"`. Insert team, then `INSERT INTO team_members(team_id, user_id) VALUES (?, ?)` for the creator (one lock, two statements). 201 + `TeamOut`.
  - GET /teams/{id}: 200 `TeamOut` | 404 `"team not found"`.
  - POST /teams/{id}/members: body `{user_id?}`; missing → 400 `"user_id is required"`; team missing → 404 `"team not found"`; user missing → 404 `"user not found"`; already a member (UniqueViolation on the PK) → 409 `"already a member"`. Success 201 + body `{"team_id": N, "user_id": M}`.
  - GET /teams/{id}/members: 200 JSON array of `UserOut` joined from `team_members`, `ORDER BY user_id`; team missing → 404.
  - No `X-User-Id` on these routes (open setup surface).
  - Tests: tower oneshot, fresh `AppState::in_memory()` per test. Each test file carries its own ~15-line helper (deliberate duplication): `async fn req(app: axum::Router, method: &str, path: &str, body: Option<&str>) -> (u16, bytes::Bytes)` built with `http` types via `axum::http::Request`, `tower::ServiceExt::oneshot`, `axum::body::to_bytes`. `tests/users.rs`: 201-echo (id > 0, fields, created_at non-empty); missing name 400; empty email 400; duplicate email 409; get 200; get 9999 → 404; get `/users/abc` → 404. `tests/teams.rs`: create 201 + creator is sole member; missing name 400; creator not found 404; get 200; get 9999 404; add member 201 (body echoes ids; members now 2); duplicate add 409; add with bad team 404; add with bad user 404.
- First action: write `src/users.rs`.
- Step cap: 8 (users.rs, teams.rs, db.rs edit, lib.rs routes, tests/users.rs, tests/teams.rs, `cargo test`, one fix pass if needed).
- Verification command: `cargo test` in repo root — expect 17 passed, 0 failed.
- Gate: [ ] approved by Morgan

Receipt (writer appends):
- Files changed:
- Verification output:
- Ambiguity hit:
