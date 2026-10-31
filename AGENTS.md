# AGENTS.md — Senior SWE assessment workspace

This repository supports a time-boxed software engineering assessment. The actual prompt is the source of truth for scope and delivery format.

## Standing instructions

- AI use is permitted for this assessment. Make decisions visible, verify generated work, and correct mistakes rather than treating output as authoritative.
- Written communication is graded. Every deliverable should explain assumptions, the chosen tradeoffs and their costs, and what remains for more time. A reviewer without session context should be able to follow it.
- Work in small, reviewable steps. Build the smallest useful slice, verify it, then extend. Do not leave code that the submitter cannot explain.
- Verify before trusting. Confirm that APIs and libraries exist, prefer standard tools and simple designs, and handle error paths as well as success paths.
- Keep momentum within the time budget. If an approach fails twice, explain why and switch. Avoid circular rechecking; when a task budget is spent, record the deferred work and move on.
- Represent experience and credentials accurately. Do not imply industry experience or a clearance that the submitter does not have. Treat classification vocabulary as concepts unless supported by direct experience.

## Execution rules

- Start only when the actual prompt is available. Produce the first useful artifact within 10 minutes or 5 tool calls, whichever comes first.
- If two consecutive turns make no file change, use the next turn to produce the smallest useful artifact.
- Do not build for an unseen prompt. Avoid speculative libraries, downloads, and infrastructure.
- Cap plans at 8 steps. Shrink scope rather than nesting subplans.
- Once a decision is recorded, do not reopen it unless new evidence contradicts it.
- Prefer simple, tested subsets over speculative completeness.
- Do not assume the submission format, language, or environment until the prompt states it.
- Before implementation, define the files in scope, first action, step cap, and verification command.
- Use a small example as a scale check: one prompt, one file, a few meaningful assertions, actually executed, followed by a brief report.

## Tandem operation: planner and writer

For multi-step work, use a planner/writer/reviewer handoff. The planner reads the prompt and repository and writes one implementation box in BOXES.md. The writer reads that box first, implements only its approved scope, and appends a receipt. The reviewer checks the diff against the box before the next box is approved.

- T1: Each box lists files, settled decisions, first action, step cap, and verification command.
- T2: The planner edits BOXES.md and does not edit source code.
- T3: Size boxes for review, ideally one file or one function.
- T4: If a box is ambiguous, record the ambiguity in the receipt and stop for review; do not widen scope.
- T5: A receipt records files changed, actual verification output, and ambiguities. Do not begin the next box without it.
- T6: The reviewer checks the diff against the approved box before the next gate.
- T7: The execution rules above apply to both planner and writer; each box stays within the eight-step plan limit.
- T8: The writer's first action for a box is to read BOXES.md, not rely on memory or assumptions.

## Assessment review rubric

Review work for:

1. End-to-end ownership: idea, design, code, tests, and production readiness.
2. Explicit tradeoffs: state the benefit, cost, deciding constraint, and revisit trigger.
3. Tests at the lowest level that catches the likely bug class.
4. Clear data ownership and module boundaries.
5. Production debugging judgment: observability, correlation, and mitigation before diagnosis.
6. Comments and write-ups that help another engineer understand the work.

For each major design choice, state the constraint, the selected option and alternative, what the choice buys and costs, why the cost is acceptable, and what would trigger revisiting it.

Public company and product research informed the framing of the README's design choices.

## Engineering quick reference

**HTTP**: GET read (200), POST create/act (201/202, not idempotent — add an idempotency key), PUT replace (200), PATCH partial, DELETE (204). Codes: 400 malformed, 401 unauthenticated, 403 not allowed, 404, 409 conflict, 422 invalid semantics, 429 rate limited, 500 server fault, 503 retry later. Cursor (keyset) pagination beats offset on large/growing collections. DB migrations ship expand-and-contract so every deploy is reversible.

**Testing**: pyramid — many unit, fewer integration, few e2e. Test boundaries (empty, one, max, past max), error paths, and irreversible operations first. Doubles: stub = canned answers, fake = working in-memory implementation (prefer fakes/stubs), mock = asserts the interaction itself. Arrange / Act / Assert, visibly separated. A flaky test is a system defect, never noise.

**Distributed failure patterns** (recognize on sight): retry storms (fix: backoff + jitter, retry budgets, circuit breakers), retried writes applied twice (idempotency keys), timeout mismatch (deadlines shrink as requests go deeper), cascading failure (bulkheads, load shedding), clock skew (monotonic clocks for durations, sequence numbers for ordering). Incident order: mitigate first (rollback, flag off, shed load), diagnose second, blameless postmortem third. First question in any incident: "What changed?"

**Architecture defaults**: modular monolith for a small team; extract a service only when teams or scaling genuinely diverge. Boundaries follow data ownership. Events are past-tense facts. Cross-service transactions are sagas with compensating actions, or a boundary redesign. Queues need a dead-letter queue and an ordering story. Cache-aside is the default cache; invalidation event first, TTL as safety net.

**Performance**: measure before optimizing; Amdahl's law caps the win. Report latency in percentiles (p50/p95/p99), never averages. Usual suspects in order: N+1 queries, missing index (read the plan first), chatty interfaces, unbounded work (no pagination), pool exhaustion. Anchors: memory ~100 ns, SSD ~100 µs, same-DC round trip ~0.5 ms, cross-continent ~100+ ms.

**Security by design**: least privilege, defense in depth, assume breach. Authorization checked server-side, per request, per object (IDOR is the classic failure). Parameterized queries always. Passwords: Argon2id/bcrypt, never fast hashes. JWTs: verify signature + exp every request; payload is encoded, not encrypted. Secrets in a manager, injected at runtime; a secret that touched git gets rotated, not just deleted. Audit log: who, what, when, which object; never log secrets or raw personal data. Encoding is not encryption.

## Working the question

1. Read the prompt twice. Identify the deliverable format, time budget, and evaluation criteria.
2. Record one concise assumption for each important ambiguity; proceed unless the outcome depends on clarification.
3. State the main design tradeoff before implementation.
4. Implement the smallest complete version, verify it, then extend.
5. Add tests for the most plausible bugs and important error paths.
6. Finish the deliverable with Assumptions, Tradeoffs, and With more time.
7. Re-read the prompt and confirm the deliverable matches it; review the code, tests, and final write-up.
