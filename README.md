# Team Notes API

A Rust REST backend for notes shared among small teams. Users can create teams, add members, and share notes; team membership grants read access to notes in that team. Note endpoints identify the caller with X-User-Id. [VERIFY]

## Run

Prerequisite: stable Rust.

cargo run

The service listens at http://127.0.0.1:8080. It creates notes.db in the working directory on startup, reusing the file if it already exists. [VERIFY]

## Test

cargo test

The integration tests cover user and team flows, note access, sharing, and stale-version conflicts. [VERIFY]

## Example flow

The commands assume jq is installed to extract IDs. The request fields and response shapes shown here should be checked against the finished handlers. [VERIFY]

BASE_URL=http://127.0.0.1:8080

OWNER_ID=$(
  curl -fsS -X POST "$BASE_URL/users" \
    -H 'Content-Type: application/json' \
    -d '{"email":"morgan@example.test","name":"Morgan"}' |
    jq -r '.id'
)

MEMBER_ID=$(
  curl -fsS -X POST "$BASE_URL/users" \
    -H 'Content-Type: application/json' \
    -d '{"email":"sam@example.test","name":"Sam"}' |
    jq -r '.id'
)

ALEX_ID=$(
  curl -fsS -X POST "$BASE_URL/users" \
    -H 'Content-Type: application/json' \
    -d '{"email":"alex@example.test","name":"Alex"}' |
    jq -r '.id'
)

TEAM_ID=$(
  curl -fsS -X POST "$BASE_URL/teams" \
    -H 'Content-Type: application/json' \
    -d "{\"name\":\"Planning\",\"created_by\":$OWNER_ID}" |
    jq -r '.id'
)

curl -fsS -X POST "$BASE_URL/teams/$TEAM_ID/members" \
  -H 'Content-Type: application/json' \
  -d "{\"user_id\":$MEMBER_ID}"

NOTE_RESPONSE=$(
  curl -fsS -X POST "$BASE_URL/notes" \
    -H 'Content-Type: application/json' \
    -H "X-User-Id: $OWNER_ID" \
    -d "{\"title\":\"Launch checklist\",\"body\":\"Review the open items.\",\"team_id\":$TEAM_ID}"
)
NOTE_ID=$(printf '%s' "$NOTE_RESPONSE" | jq -r '.id')
NOTE_VERSION=$(printf '%s' "$NOTE_RESPONSE" | jq -r '.version')

curl -fsS -X POST "$BASE_URL/notes/$NOTE_ID/shares" \
  -H 'Content-Type: application/json' \
  -H "X-User-Id: $OWNER_ID" \
  -d "{\"user_id\":$ALEX_ID,\"permission\":\"read\"}"

curl -fsS "$BASE_URL/notes/$NOTE_ID" \
  -H "X-User-Id: $ALEX_ID"

curl -fsS -X PATCH "$BASE_URL/notes/$NOTE_ID" \
  -H 'Content-Type: application/json' \
  -H "X-User-Id: $OWNER_ID" \
  -d "{\"version\":$NOTE_VERSION,\"title\":\"Launch checklist — updated\"}"

curl -fsS -G "$BASE_URL/notes" \
  -H "X-User-Id: $OWNER_ID" \
  --data-urlencode 'q=launch'

## Endpoints

The authorization rules and success statuses below describe the intended contract and should be checked against the finished handlers. [VERIFY]

| Method | Path | Who can call it | Success |
|---|---|---|---|
| GET | /health | No authentication specified | 200 OK [VERIFY] |
| POST | /users | No authentication specified | 201 Created [VERIFY] |
| GET | /users/{id} | No authentication specified | 200 OK [VERIFY] |
| POST | /teams | No authentication specified; creator becomes a member | 201 Created [VERIFY] |
| GET | /teams/{id} | No authentication specified | 200 OK [VERIFY] |
| POST | /teams/{id}/members | No authentication specified | 201 Created [VERIFY] |
| GET | /teams/{id}/members | No authentication specified | 200 OK [VERIFY] |
| POST | /notes | Valid X-User-Id | 201 Created [VERIFY] |
| GET | /notes/{id} | Valid X-User-Id; owner, share recipient, or team member with access | 200 OK [VERIFY] |
| GET | /notes | Valid X-User-Id; returns notes the user can access | 200 OK [VERIFY] |
| PATCH | /notes/{id} | Valid X-User-Id; owner or write-share recipient | 200 OK [VERIFY] |
| DELETE | /notes/{id} | Valid X-User-Id; owner | 204 No Content [VERIFY] |
| POST | /notes/{id}/shares | Valid X-User-Id; owner | 201 Created [VERIFY] |
| GET | /notes/{id}/shares | Valid X-User-Id; owner | 200 OK [VERIFY] |
| DELETE | /notes/{id}/shares/{user_id} | Valid X-User-Id; owner | 204 No Content [VERIFY] |

Missing, invalid, or unknown user IDs on note endpoints return 401. A user who cannot access a note receives 404; a user who can see it but lacks the requested permission receives 403. Errors use JSON shaped like {"error":"..."} and may use 400, 401, 403, 404, 409, or 500. [VERIFY]

## Design choices

Given the service exists to be shared amongst several small teams, I chose authorization in the service with X-User-Id as a trusted identity seam over built-in authentication. What it buys: ownership, team membership, and share rules can be exercised without coupling them to an identity provider. What it costs: the service trusts whoever can set the header. That cost is acceptable because production would have a gateway set it after token validation. I would revisit if the service needed to validate identities independently. [VERIFY]

Given small teams and low write concurrency, I chose one Mutex-guarded SQLite connection over a client-server database and connection pool. What it buys: simple local setup and a small operational footprint. What it costs: database work is serialized through one connection. That cost is acceptable because the expected workload is small and local. I would revisit if write contention or availability requirements outgrew a single-process database. [VERIFY]

Given that concurrent edits must not silently overwrite one another, I chose optimistic locking with an explicit 409 conflict over last-write-wins. What it buys: a stale update is rejected instead of discarding a newer edit. What it costs: clients must handle conflicts and retry with the current version. That cost is acceptable because surfacing the conflict preserves user changes. I would revisit if collaborative editing required automatic merge behavior. [VERIFY]

## Assumptions

- The service is for small teams with low write concurrency.
- X-User-Id is trusted; in production, a gateway would set it after validating a token. [VERIFY]
- Search is a case-insensitive (ASCII) substring match on title and body, with no pagination. [VERIFY]

## With more time

- Add real authentication with OIDC or JWT validation at the gateway seam.
- Move to PostgreSQL with a connection pool.
- Add pagination to GET /notes.
- Allow share permissions to be upgraded without deleting and recreating the share.
- Add soft deletes and note history.
- Add full-text search with FTS5.

.github/workflows/ci.yml

name: ci

on:
  push:
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test
