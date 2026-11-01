# Team Notes API

A Rust REST API for notes shared among small teams. Users can create teams, add members, and share notes. Team membership grants read access to notes in that team. Note endpoints identify the caller with the X-User-Id header.

## Run

Prerequisite: stable Rust.

```sh
cargo run
```

The service listens at http://127.0.0.1:8080. It creates notes.db in the working directory on startup and reuses it if it already exists. Delete notes.db to reset the local service data.

## Test

```sh
cargo test
```

The integration tests cover user and team flows, note access, sharing, search, and stale-version conflicts. GitHub Actions runs the same command on pushes and pull requests.

## Example flow

This example requires curl and jq. Each run creates users with a unique email suffix so it can be repeated against the persistent database.

```sh
set -eu
BASE_URL=http://127.0.0.1:8080
RUN_ID="$(date +%s)-$$"

OWNER_ID=$(
  curl -fsS -X POST "$BASE_URL/users" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"morgan-$RUN_ID@example.test\",\"name\":\"Morgan\"}" |
    jq -r ".id"
)

MEMBER_ID=$(
  curl -fsS -X POST "$BASE_URL/users" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"sam-$RUN_ID@example.test\",\"name\":\"Sam\"}" |
    jq -r ".id"
)

ALEX_ID=$(
  curl -fsS -X POST "$BASE_URL/users" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"alex-$RUN_ID@example.test\",\"name\":\"Alex\"}" |
    jq -r ".id"
)

OUTSIDER_ID=$(
  curl -fsS -X POST "$BASE_URL/users" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"outsider-$RUN_ID@example.test\",\"name\":\"Outsider\"}" |
    jq -r ".id"
)

TEAM_ID=$(
  curl -fsS -X POST "$BASE_URL/teams" \
    -H "Content-Type: application/json" \
    -d "{\"name\":\"Planning\",\"created_by\":$OWNER_ID}" |
    jq -r ".id"
)

curl -fsS -X POST "$BASE_URL/teams/$TEAM_ID/members" \
  -H "Content-Type: application/json" \
  -H "X-User-Id: $OWNER_ID" \
  -d "{\"user_id\":$MEMBER_ID}"

NOTE_RESPONSE=$(
  curl -fsS -X POST "$BASE_URL/notes" \
    -H "Content-Type: application/json" \
    -H "X-User-Id: $OWNER_ID" \
    -d "{\"title\":\"Launch checklist\",\"body\":\"Review the open items.\",\"team_id\":$TEAM_ID}"
)
NOTE_ID=$(printf "%s" "$NOTE_RESPONSE" | jq -r ".id")
NOTE_VERSION=$(printf "%s" "$NOTE_RESPONSE" | jq -r ".version")

curl -fsS -X POST "$BASE_URL/notes/$NOTE_ID/shares" \
  -H "Content-Type: application/json" \
  -H "X-User-Id: $OWNER_ID" \
  -d "{\"user_id\":$ALEX_ID,\"permission\":\"read\"}"

curl -fsS "$BASE_URL/notes/$NOTE_ID" \
  -H "X-User-Id: $ALEX_ID"

curl -fsS -X PATCH "$BASE_URL/notes/$NOTE_ID" \
  -H "Content-Type: application/json" \
  -H "X-User-Id: $OWNER_ID" \
  -d "{\"version\":$NOTE_VERSION,\"title\":\"Launch checklist — updated\"}"

curl -fsS -G "$BASE_URL/notes" \
  -H "X-User-Id: $OWNER_ID" \
  --data-urlencode "q=launch"

# Expected error: HTTP 404 because the outsider has no access to this note.
curl -sS -w "HTTP %{http_code}\n" "$BASE_URL/notes/$NOTE_ID" \
  -H "X-User-Id: $OUTSIDER_ID"

# Expected error: HTTP 403 because Alex has read access but cannot edit.
curl -sS -w "HTTP %{http_code}\n" -X PATCH "$BASE_URL/notes/$NOTE_ID" \
  -H "Content-Type: application/json" \
  -H "X-User-Id: $ALEX_ID" \
  -d "{\"version\":$NOTE_VERSION,\"title\":\"Alex edit\"}"

# Expected error: HTTP 409 because version 1 is stale after the owner update.
curl -sS -w "HTTP %{http_code}\n" -X PATCH "$BASE_URL/notes/$NOTE_ID" \
  -H "Content-Type: application/json" \
  -H "X-User-Id: $OWNER_ID" \
  -d "{\"version\":$NOTE_VERSION,\"title\":\"Stale owner edit\"}"

printf "Restart-persistence user ID: %s\n" "$OWNER_ID"
```

## Endpoints

| Method | Path | Access | Success |
|---|---|---|---|
| GET | /health | No authentication required | 200 OK |
| POST | /users | No authentication required | 201 Created |
| GET | /users/{id} | No authentication required | 200 OK |
| POST | /teams | No authentication required; creator becomes a member | 201 Created |
| GET | /teams/{id} | No authentication required | 200 OK |
| POST | /teams/{id}/members | Valid X-User-Id of an existing team member | 201 Created |
| GET | /teams/{id}/members | No authentication required | 200 OK |
| POST | /notes | Valid X-User-Id | 201 Created |
| GET | /notes/{id} | Valid X-User-Id with owner, share, or team access | 200 OK |
| GET | /notes | Valid X-User-Id; lists accessible notes | 200 OK |
| PATCH | /notes/{id} | Owner or write-share recipient | 200 OK |
| DELETE | /notes/{id} | Owner | 204 No Content |
| POST | /notes/{id}/shares | Owner | 201 Created |
| GET | /notes/{id}/shares | Owner | 200 OK |
| DELETE | /notes/{id}/shares/{user_id} | Owner | 204 No Content |

All note and share endpoints require a valid X-User-Id. A missing, invalid, or unknown user ID returns 401. A user with no note access receives 404; a user who has access but lacks the requested permission receives 403. Application errors use JSON shaped like {"error":"..."}; responses may use 400, 401, 403, 404, 409, or 500.

## Tradeoffs

Given the service needs to support team sharing while keeping the example independent of an identity provider, I chose a trusted X-User-Id header over built-in authentication. What it buys: ownership, team membership, and share rules can be exercised through the API. What it costs: the service trusts whoever can set the header. That cost is acceptable because production would have a gateway set it after token validation. I would revisit if the service needed to validate identities independently.

Given small teams and low write concurrency, I chose one Mutex-guarded SQLite connection over a client-server database and connection pool. What it buys: simple local setup and a small operational footprint. What it costs: database work is serialized through one connection. That cost is acceptable because the expected workload is small and local. I would revisit if write contention or availability requirements outgrew a single-process database.

Given concurrent edits must not silently overwrite one another, I chose optimistic locking with an explicit 409 conflict over last-write-wins. What it buys: a stale update is rejected instead of discarding a newer edit. What it costs: clients must handle conflicts and retry with the current version. That cost is acceptable because surfacing the conflict preserves user changes. I would revisit if collaborative editing required automatic merge behavior.

## Assumptions

- The service is for small teams with low write concurrency.
- X-User-Id is trusted; in production, a gateway would set it after validating a token.
- Search uses SQLite LIKE, which is ASCII case-insensitive; it matches substrings in title and body. An empty q means no search filter, and the list is not paginated.

## With more time

- Add real authentication with OIDC or JWT validation at the gateway seam.
- Move to PostgreSQL with a connection pool.
- Add pagination to GET /notes.
- Allow share permissions to be upgraded without deleting and recreating the share.
- Add soft deletes and note history.
- Add full-text search with FTS5.
