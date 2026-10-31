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
- Pre-decisions for later boxes (Codex findings + Morgan amendments, adopted 2026-10-31):
  - Box 6: list query must parenthesize the access disjunction — `WHERE (n.owner_id = ? OR share hit OR team hit) AND (title LIKE ? OR body LIKE ?)`; regression tests: an owned non-matching note and a shared non-matching note are both excluded from `?q=` results.
  - Box 7: `main.rs` switches from `AppState::in_memory()` to `db::new_file("notes.db")` (file-backed, persists across restarts; `notes.db` already gitignored). `new_file` startup error = `eprintln!` + `exit(1)`. Unknown-route JSON fallback is consistency polish. Smoke includes a restart-persistence check.
- Crate-path rule (Morgan, 2026-10-31): every crate type path a box names is verified with one grep in the vendored source (~/.cargo/registry/src/*/axum-0.8.9/) before the box is final. Verified: `axum::extract::rejection::JsonRejection` (src/extract/rejection.rs).
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
- Gate: [*] approved by Morgan

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
    - `pub fn new_in_memory() -> rusqlite::Connection`: open `:memory:`, `pragma_update(None, "foreign_keys", "ON")`, execute the CREATE TABLEs from `SCHEMA` (PRAGMA set via pragma_update, DDL via `execute_batch`). **Gate amendment (Morgan, 2026-10-31): all `CREATE TABLE` are `CREATE TABLE IF NOT EXISTS` — schema creation is idempotent so a file-backed database restarts cleanly; disclose in the receipt.**
    - `pub fn new_file(path: &str) -> rusqlite::Result<Connection>`: open path, same pragma + DDL. Error → `AppError::Internal`.
    - Unit test inside db.rs: `new_in_memory()` yields exactly the 5 tables in `sqlite_master`.
  - `src/error.rs`: `pub enum AppError { BadRequest(String), Unauthorized, NotFound(String), Conflict(String), Internal(String) }` with `impl IntoResponse`: 400/401/404/409/500, each body JSON `{"error": "<msg>"}`.
  - JSON body handling (used from Box 3 on): handlers take `json: Result<axum::Json<T>, axum::extract::rejection::JsonRejection>`; any rejection maps to `AppError::BadRequest("malformed JSON body".into())`. Decided here so Box 3+ is uniform.
  - `src/lib.rs`: `#[derive(Clone)] pub struct AppState { pub db: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>> }` with `impl AppState { pub fn in_memory() -> Self }` (wraps `db::new_in_memory`); `pub fn router(state: AppState) -> axum::Router` — `/health` unchanged, router built with `.with_state(state)`.
  - `tests/health.rs`: build `AppState::in_memory()`, `notes::router(state).oneshot(...)`, same asserts as Box 1.
  - Edge: note_shares cascade with their note; team_members cascade with team or user (decided by amendment 2026-10-31: deleting a shared note must not 500). notes.owner_id and notes.team_id stay default NO ACTION; record as assumption in Box 7.
- First action: write `src/db.rs` (SCHEMA const first).
- Step cap: 5 (db.rs, error.rs, lib.rs edit, health.rs edit, `cargo test`).
- Verification command: `cargo test` — expect 2 passed (health + schema unit test).
- Gate: [*] approved by Morgan (draft — finalized when Box 1 receipt is reviewed)

Receipt (writer appends):
- Files changed: `src/db.rs` (new), `src/error.rs` (new), `src/lib.rs` (modified: `AppState` + `router(state)`), `tests/health.rs` (modified: builds `AppState::in_memory()`), and — **box gap, forced, disclosed** — `src/main.rs` (2 lines: `let state = notes::AppState::in_memory();` passed to `notes::router(state)`). The box's `router` signature change makes `main.rs` uncompilable as-is, and `cargo test` (the verification command) builds the bin target; `main.rs` is not in the box's files list. `AppState` has no file-backed constructor in this box, so `in_memory()` is the only available choice; a later box presumably switches main to a file DB.
- Verification output: `cargo test` in repo root:
  ```
  Running unittests src/lib.rs (target/debug/deps/notes-5216898542882618)
  running 1 test
  test db::tests::in_memory_db_has_exactly_five_tables ... ok
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

  Running tests/health.rs (target/debug/deps/health-e8993e72a17ca554)
  running 1 test
  test health_returns_ok ... ok
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  (main.rs unit harness: 0 tests, ok; doc-tests: 0, ok.)
- Ambiguity hit: several, all disclosed:
  1. **Gate amendment applied (Morgan, 2026-10-31):** all 5 tables are `CREATE TABLE IF NOT EXISTS` so file-backed DBs restart cleanly; `new_file` re-applies the idempotent schema on open.
  2. Box's `axum::extract::rejection::JsonRejection` does not exist in axum 0.8.9 (no root re-export). Verified against the vendored crate source: canonical path is `axum::extract::rejection::JsonRejection`. Implemented with that path per the standing deviation rule. **Box 3 (line 120) also names `axum::extract::rejection::JsonRejection` — planner should fix that reference before gating Box 3.**
  3. `new_in_memory()` signature is non-Result per box; used `.expect(...)` on open/init (in-memory open failure is effectively impossible; no caller can recover anyway).
  4. Schema unit test filters `name NOT LIKE 'sqlite_%'` so "exactly 5 tables" is not tripped by SQLite's internal `sqlite_sequence` (created for AUTOINCREMENT); intent preserved, test cannot be flaky.
  5. `AppError::Unauthorized` body message unspecified by the box → `"unauthorized"` (only messageless variant; Box 3+ tests pin the other messages).
  6. Added `impl From<axum::extract::rejection::JsonRejection> for AppError` in `error.rs` as the uniform mechanism for the box's JSON-body decision (file already in scope).

---

## Box 3: Users, teams, membership + tests
- Files touched: `src/users.rs` (new), `src/teams.rs` (new), `src/db.rs` (add `is_unique`), `src/lib.rs` (register routes), `tests/users.rs` (new), `tests/teams.rs` (new).
- Decisions made:
  - Path-param convention (all id routes, this box onward): `{id}` extracted as `String` via `Path<String>`, parsed to `i64` by hand; parse failure → 404 `{"error":"not found"}`. This keeps the JSON error shape uniform (no axum rejection text) and treats non-numeric ids as nonexistent.
  - Response shapes (serde derive): `users.rs`: `pub struct UserOut { id: i64, name: String, email: String, created_at: String }`. `teams.rs`: `pub struct TeamOut { id: i64, name: String, created_at: String }`.
  - Handler signature pattern (uniform): `pub async fn create_user(State(s): State<AppState>, json: Result<axum::Json<NewUser>, axum::extract::rejection::JsonRejection>) -> Result<(StatusCode, Json<UserOut>), AppError>`. `JsonRejection` → `AppError::BadRequest("malformed JSON body")` per Box 2.
  - `src/db.rs`: add `pub fn is_unique(e: &rusqlite::Error) -> bool { matches!(e, rusqlite::Error::UniqueViolation(_)) }`.
  - POST /users: body `{name?, email?}` as `Option`s. Either missing, or empty after `trim()` → 400 `"name and email are required"`. Store trimmed values. Email is lowercased at ingest (`email.trim().to_lowercase()`), so duplicates differing only by case hit the UNIQUE constraint. UniqueViolation → 409 `"email already registered"`. Success 201 + `UserOut`.
  - GET /users/{id}: 200 `UserOut` | 404 `"user not found"`.
  - POST /teams: body `{name?, created_by?}`; name missing/empty-after-trim → 400 `"name is required"`; created_by missing → 400 `"created_by is required"`; creator not a row in `users` → 404 `"creator not found"`. Insert team, then `INSERT INTO team_members(team_id, user_id) VALUES (?, ?)` for the creator (one lock, two statements). 201 + `TeamOut`.
  - GET /teams/{id}: 200 `TeamOut` | 404 `"team not found"`.
  - POST /teams/{id}/members: body `{user_id?}`; missing → 400 `"user_id is required"`; team missing → 404 `"team not found"`; user missing → 404 `"user not found"`; already a member (UniqueViolation on the PK) → 409 `"already a member"`. Success 201 + body `{"team_id": N, "user_id": M}`.
  - GET /teams/{id}/members: 200 JSON array of `UserOut` joined from `team_members`, `ORDER BY user_id`; team missing → 404.
  - No `X-User-Id` on these routes (open setup surface).
  - Tests: tower oneshot, fresh `AppState::in_memory()` per test. Each test file carries its own ~15-line helper (deliberate duplication): `async fn req(app: axum::Router, method: &str, path: &str, body: Option<&str>) -> (u16, bytes::Bytes)` built with `http` types via `axum::http::Request`, `tower::ServiceExt::oneshot`, `axum::body::to_bytes`. `tests/users.rs`: 201-echo (id > 0, fields, created_at non-empty); missing name 400; empty email 400; duplicate email 409; case-insensitive duplicate 409 ("User@X.com" after "user@x.com"); get 200; get 9999 → 404; get `/users/abc` → 404. `tests/teams.rs`: create 201 + creator is sole member; missing name 400; creator not found 404; get 200; get 9999 404; add member 201 (body echoes ids; members now 2); duplicate add 409; add with bad team 404; add with bad user 404.
- First action: write `src/users.rs`.
- Step cap: 8 (users.rs, teams.rs, db.rs edit, lib.rs routes, tests/users.rs, tests/teams.rs, `cargo test`, one fix pass if needed).
- Verification command: `cargo test` in repo root — expect 18 passed, 0 failed.
- Gate: [*] approved by Morgan

Receipt (writer appends):
- Files changed: `src/users.rs`, `src/teams.rs`, `src/db.rs`, `src/lib.rs` (Box 3 route wiring was already present), `tests/users.rs`, and `tests/teams.rs`.
- Verification output: `cargo test` in repo root:
  ```
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
  test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
  test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  ```
  Total: 19 passed, 0 failed (schema, health, teams, users; 17 integration tests).
- Ambiguity hit: no unresolved ambiguity. Fixed inherited compile errors: `Connection::execute` returns `Result<usize, _>`, and rusqlite 0.41.1 has no `Error::UniqueViolation`; `db::is_unique` now checks `SqliteFailure` with `ErrorCode::ConstraintViolation` and the extended UNIQUE or PRIMARYKEY constraint code found in the vendored rusqlite/SQLite sources. Also lowercased trimmed email before insert and added the requested `User@X.com` then `user@x.com` conflict test. Changed user/team fetch handling so only `QueryReturnedNoRows` maps to NotFound and other SQLite errors map to Internal; existence checks likewise distinguish no row from query failure. The first compile exposed private request types in public handlers, so those types are public; test runs also exposed a borrowed router passed to a by-value helper and missing JSON Content-Type headers, both corrected in the Box 3 tests.
---

## Box 4: Notes CRUD (owner-only) + tests
- Files touched: `src/auth.rs` (new), `src/notes.rs` (new), `src/error.rs` (modify: add `Forbidden(String)` -> 403; change `Unauthorized` to `Unauthorized(String)` so auth.rs's three distinct 401 messages are expressible), `src/lib.rs` (register routes), `tests/notes.rs` (new).
- Decisions made:
  - Identity helper (no new deps — no async-trait crate): `src/auth.rs` defines `pub fn identity(state: &AppState, headers: &axum::http::HeaderMap) -> Result<i64, AppError>`. Reads `x-user-id`: missing → `Unauthorized` `"missing X-User-Id header"`; non-integer → `Unauthorized` `"invalid X-User-Id header"`; not a row in `users` → `Unauthorized` `"unknown user"`. Every /notes handler takes `headers: axum::http::HeaderMap` and calls this first.
  - Scope: owner-only in this box. Non-owner on any /notes/{id} route → 404 `"note not found"` (indistinguishable from missing, per spec's "does not exist = 404"). Box 5 generalizes to the full matrix.
  - `NoteOut` (serde derive): `{ id: i64, owner_id: i64, team_id: Option<i64>, title: String, body: String, version: i64, created_at: String, updated_at: String }` — `team_id` serializes as `null` when absent.
  - POST /notes: body `{ title: Option<String>, body: Option<String>, team_id: Option<i64> }`. title missing or empty-after-trim → 400 `"title is required"`. team_id present and team missing → 404 `"team not found"`. team_id present and owner not in `team_members` → 403 `"not a member of that team"`. body defaults to `""`. Insert with `version = 1`. 201 + `NoteOut`.
  - GET /notes/{id}: owner → 200 `NoteOut`; else/missing → 404.
  - PATCH /notes/{id}: body `{ title: Option<String>, body: Option<String>, version: Option<i64> }`. version missing → 400 `"version is required"`. Neither title nor body present → 400 `"title or body is required"`. title present but empty-after-trim → 400 `"title is required"`. body present may be `""` (valid). Non-owner or missing note → 404 `"note not found"`. Version mismatch → 409 `"version conflict"`. Apply as one `UPDATE notes SET <given fields>, version = version + 1, updated_at = datetime('now') WHERE id = ? AND owner_id = ? AND version = ?` (SET clause built from the fields present); 0 rows with existing+matching version is impossible under the single mutex, but if it happens → 409. 200 + fresh `NoteOut`.
  - DELETE /notes/{id}: owner → `StatusCode::NO_CONTENT`, empty body; else → 404. Note deletion cascades its shares (Box 2 schema).
  - Routes registered: `/notes` post; `/notes/{id}` get + patch + delete (separate `.route` calls, same path).
  - Tests (`tests/notes.rs`; tower oneshot; per-file `req` helper as in Box 3; `setup()` → fresh app, owner user, second user, team created by owner; all via router calls). TRIMMED to 8 (Morgan, 2026-10-31):
  1. create 201: version == 1, body == "", team_id null, title echoes
  2. identity: missing header 401, non-integer 401, nonexistent user 401 — three distinct messages, three asserts
  3. create with non-member team → 403
  4. get as non-owner 404 (also covers id 9999 → 404)
  5. patch happy path 200: v1→v2 is the assertion, title change visible on re-GET (assert only `updated_at >= created_at` — 1-second resolution)
  6. patch stale version 409
  7. patch without version 400
  8. delete as owner 204 (re-GET → 404)
- First action: write `src/auth.rs`.
- Step cap: 7 (auth.rs, notes.rs, lib.rs, tests/notes.rs, `cargo test`, two fix passes if needed).
- Verification command: `cargo test` in repo root — expect 26 passed, 0 failed.
- Gate: [ ] approved by Morgan (draft — finalized when Box 3 receipt is reviewed)

Receipt (writer appends):
- Files changed:
- Verification output:
- Ambiguity hit:

---

## Box 5a: Access control — helper + matrix enforcement + tests
- Files touched: `src/notes.rs` (modify). No route changes; `src/lib.rs` untouched.
- Decisions made:
  - `enum Perm { Owner, Write, Read }` (private to notes.rs). Resolution, in order: `note.owner_id == user` → Owner; else `note_shares` row for (note, user): `'write'` → Write, `'read'` → Read; else if `note.team_id` is Some and `user` is a member of that team → Read; else → 404.
  - Helper signature: `fn access(conn: &rusqlite::Connection, user: i64, note: &NoteRow) -> Perm` — operates on data fetched under the handler's lock. **Lock discipline (Codex rule): exactly one `state.db.lock()` per handler; the note fetch, the access check, and the mutation all run under the same guard.**
  - Enforcement: GET needs Read+; PATCH needs Write+; DELETE needs Owner. Has access, lacks right → `Forbidden` 403 `"insufficient permissions"`. No access → 404 `"note not found"` (identical to missing note — no info leak).
  - PATCH mutation changes: `WHERE id = ? AND version = ?` (owner_id removed — write-shares must be able to update). Everything else in the PATCH contract (400/409 semantics, version bump, updated_at) unchanged.
  - `NoteRow` stays a private struct in notes.rs (the 8 fields of the notes table).
  - Tests (`tests/matrix.rs`; tower oneshot; setup via router: owner, writer, reader, outsider; note created by owner with `team_id` NULL; `note_shares` rows for writer=`'write'`, reader=`'read'` by direct SQL on `state.db`). TRIMMED to 5 (Morgan, 2026-10-31):
  1. GET as outsider → 404
  2. PATCH as writer 200 (version 1→2 — the assertion)
  3. PATCH as reader → 403
  4. DELETE as writer → 403
  5. DELETE as owner 204 (re-GET → 404)
- First action: add `Perm` + `access` to `src/notes.rs`.
- Step cap: 5 (notes.rs, tests/matrix.rs, `cargo test`, two fix passes if needed).
- Verification command: `cargo test` in repo root — expect 31 passed, 0 failed.
- Gate: [ ] approved by Morgan (draft — finalized when Box 4 receipt is reviewed)

Receipt (writer appends):
- Files changed:
- Verification output:
- Ambiguity hit:

---

## Box 5b: Shares endpoints + tests
- Files touched: `src/shares.rs` (new), `src/lib.rs` (register routes), `tests/shares.rs` (new).
- Decisions made:
  - Routes: `POST /notes/{id}/shares`, `GET /notes/{id}/shares`, `DELETE /notes/{id}/shares/{user_id}`. Both `{id}` and `{user_id}` extracted as `String` → `i64`; parse failure → 404.
  - Owner-only: note missing or no access → 404 `"note not found"`; access but not Owner → 403 `"insufficient permissions"`. One lock per handler (Codex rule); access check + mutation under the same guard.
  - POST: body `{ user_id: Option<i64>, permission: Option<String> }`. Either missing → 400 `"user_id and permission are required"`. `permission` must be exactly `"read"` or `"write"` (case-sensitive) → else 400 `"permission must be 'read' or 'write'"`. Target not in `users` → 404 `"user not found"`. Target == owner → 409 `"already the owner"` (ratified). Already a `note_shares` row → 409 `"already shared"` (ratified). Success 201 + body `{"user_id": N, "permission": "..."}`.
  - GET: 200 + array of `{"user_id": i64, "permission": String}` (ids only, ratified) `ORDER BY user_id`.
  - DELETE: 204 `NO_CONTENT`, empty body, always — deleting a share that does not exist is a no-op (idempotent, ratified).
  - Tests (`tests/shares.rs`; tower oneshot; setup: owner, writer, reader, outsider, note owned by owner with `team_id` NULL). TRIMMED to 6 (Morgan, 2026-10-31):
  1. POST write share 201 (body echoes), then recipient PATCHes 200 (v1→v2)
  2. POST duplicate share → 409
  3. POST invalid permission (`"write "`) → 400
  4. POST owner as target → 409
  5. DELETE share 204; re-GET gone; DELETE again → 204 (idempotent)
  6. POST share as writer (non-owner) → 403
- First action: write `src/shares.rs`.
- Step cap: 7 (shares.rs, lib.rs, tests/shares.rs, `cargo test`, two fix passes if needed).
- Verification command: `cargo test` in repo root — expect 37 passed, 0 failed.
- Gate: [ ] approved by Morgan (draft — finalized when Box 5a receipt is reviewed)

Receipt (writer appends):
- Files changed:
- Verification output:
- Ambiguity hit:

## Box 6: List, search, optimistic-locking proof + tests
- Files touched: `src/notes.rs` (add list handler), `src/lib.rs` (register GET /notes), `tests/list.rs` (new).
- Decisions made:
  - Query params: `#[derive(serde::Deserialize)] struct ListParams { q: Option<String> }`; handler takes `Result<axum::extract::Query<ListParams>, axum::extract::rejection::QueryRejection>` (path verified in vendored axum 0.8.9, `src/extract/rejection.rs:97`); rejection -> `BadRequest("malformed query")`. Identity via `auth::identity` first; one lock for the query.
  - Access filter in SQL with `EXISTS` (no JOIN duplicates): `WHERE (n.owner_id = :uid OR EXISTS (SELECT 1 FROM note_shares ns WHERE ns.note_id = n.id AND ns.user_id = :uid) OR EXISTS (SELECT 1 FROM team_members tm WHERE tm.team_id = n.team_id AND tm.user_id = :uid))`. The parenthesization of the access disjunction is mandatory (Codex regression: unparenthesized, AND binds tighter than OR and the search filter applies only to the last branch).
  - Search: when `q` is present and non-empty, append `AND (n.title LIKE :pat ESCAPE '\' OR n.body LIKE :pat ESCAPE '\')`, where `pat = "%" + escaped + "%"` and escaping prefixes each of `\`, `%`, `_` in `q` with a backslash. `?q=` (empty) is treated as absent. SQLite LIKE is ASCII case-insensitive; that is the documented behavior.
  - Order: `ORDER BY n.updated_at DESC, n.id DESC` (`id` breaks the 1-second timestamp ties). No pagination — all accessible notes.
  - Response: 200 + JSON array of `NoteOut`.
- Tests (`tests/list.rs`; setup via router + direct SQL for the share row: users A and B; A owns "Alpha one" and "Beta two"; B owns "Gamma", shared to A by direct SQL; team T created by A, B added as member; B owns "Delta" with `team_id T`):
  1. list as A returns exactly the ids of {Alpha, Beta, Gamma, Delta}, no duplicates.
  2. order: `std::thread::sleep(1.1s)`, PATCH Alpha as A, assert Alpha is first.
  3. `?q=ALPH` returns only Alpha (case-insensitive match on an owned note).
  4. `?q=zzz` returns an empty array (owned non-matching notes are excluded — precedence regression).
  5. `?q=alp` returns only Alpha (shared non-matching note Gamma is excluded — precedence regression).
  6. concurrency: two tasks PATCH the same note with version 1; exactly one returns 200, the other 409.
- First action: add `ListParams` and the list handler to `src/notes.rs`.
- Step cap: 6 (`notes.rs`, `lib.rs`, `tests/list.rs`, `cargo test`, two fix passes).
- Verification command: `cargo test` — expect 43 passed, 0 failed.
- Gate: [ ] approved by Morgan

## Box 7: File-backed startup, route fallback, final README + smoke
- Files touched: `src/main.rs` (modify), `src/lib.rs` (modify), `tests/fallback.rs` (new), `README.md` (modify).
- Decisions made:
  - In `main.rs`, open the database with `notes::db::new_file("notes.db")` and construct `AppState` around that connection before starting the server. On startup error, `eprintln!` the error and call `std::process::exit(1)`. Keep the fixed `127.0.0.1:8080` listener behavior.
  - Add a router fallback for unknown routes that returns 404 JSON `{"error":"not found"}`, matching the established error shape. Registered routes and their existing error behavior stay unchanged.
  - Add one router integration test in `tests/fallback.rs`: GET an unknown path and assert status 404 and exact JSON body `{"error":"not found"}`.
  - `README.md` is a committed draft containing `[VERIFY]` markers. Check every marked claim against the finished code, correct any unsupported or stale claim, then remove every marker. Verify zero remain with a grep that reports no matches. Keep the README's example flow executable as written, including a unique email so it can be repeated without deleting `notes.db`. Include the required Assumptions, Tradeoffs, and With more time sections; state the tradeoff using the global template.
  - Live smoke: run the server, execute the README example flow verbatim, create a user, stop and restart the server, then GET that user's returned id and confirm the record persists. Do not delete `notes.db` during this check.
  - Run the final `cargo test` after the smoke.
- First action: change `src/main.rs` to initialize `AppState` with `db::new_file("notes.db")`.
- Step cap: 7 (main.rs, lib.rs, tests/fallback.rs, README.md verification/edit, zero-marker grep, live smoke including restart check, final cargo test).
- Verification command: `cargo test` — expect 44 passed, 0 failed; `grep -n '\[VERIFY\]' README.md` — expect no matches. Live smoke is as specified above.
- Gate: [ ] approved by Morgan
