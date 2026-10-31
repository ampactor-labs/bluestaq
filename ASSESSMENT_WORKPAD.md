# Assessment Workpad & Cockpit

> **Candidate**: Morgan Espitia  
> **Role**: Senior Software Engineer, Bluestaq  
> **Target Platform**: TestGorilla (180 Minutes, 1 Question)  
> **Started At**: `[TBD: e.g. 21:05 MDT]`  
> **Submission Deadline**: `[TBD: Started At + 3h]`

> **Tandem roster (added 2026-10-31)**: PLANNER is Qwen3.8-27B in Build mode restricted by rule to one writable file, BOXES.md (OpenCode Plan mode cannot write repo files); it writes boxes and reviews diffs. WRITER is a second, fresh Qwen3.8-27B session in Build mode (the 8B failed the writer audition on 2026-10-31 and is sidecar-only); it implements one gated box at a time and appends its receipt under the box. Morgan gates every box and owns all submissions. Loni (chat) is the fast second brain for triage and the final review pass. Rhythm: while the writer implements box n, the planner drafts box n+1; review happens at the seam between boxes, not in one pass at the end.

---

## 1. Time Budget & Milestone Log

| Phase | Window | Target Milestone | Status / Timestamp |
| :--- | :--- | :--- | :--- |
| **01. Triage & Framing** | `00:00 - 00:15` | Read prompt twice. Fill Prompt & Deliverable Constraints. State assumptions. Pick language & architecture. | `[ ]` |
| **02. Walking Skeleton** | `00:15 - 00:45` | Smallest complete end-to-end slice working. Core types & entry point. First automated test passing. | `[ ]` |
| **03. Core Implementation** | `00:45 - 01:25` | Complete core domain logic. Unit tests for happy path & core business rules. | `[ ]` |
| **04. Error Paths & Edges** | `01:25 - 02:00` | Input validation, boundary values, malformed data handling, error classification, logging/correlation IDs. | `[ ]` |
| **05. Integration & Robustness** | `02:00 - 02:30` | End-to-end tests, stress/concurrency/boundary scenarios, edge cases caught. | `[ ]` |
| **06. Final Write-Up & Packaging** | `02:30 - 03:00` | Complete Assumptions, Tradeoffs, "With more time". Package deliverable (zip/git/text) & verify against rubric. | `[ ]` |

---

## 2. Prompt & Constraints (Raw Copy-Paste)

### The Prompt
```text
➡️ YOU CAN USE AI FOR THIS CHALLENGE!
Not all Bluestaq assessments allow AI use, but given this coding challenge's difficulty level and the time limit, we encourage you to use any tools or resources at your disposal, including AI tools.

➡️ Please submit a link to a version-controlled repository (GitHub, GitLab, etc.) that reviewers can access.

Your Task
You're building the backend for a note-taking service used and shared amongst several small teams. Users need somewhere to capture and work with their notes.

Deliver a runnable REST service and README. We will review your code for clarity, correctness, and the tradeoffs you made. You choose the language, framework, schema, endpoints, storage, and what's in scope. Be ready to defend those choices.

What to Submit
Submit a link to a version-controlled repository (GitHub, GitLab, etc.) that reviewers can access.
Include:
• All source code and tests
• A README that includes but is not limited to:
  • 2–3 design choices you spent the most time on, and why you made them
  • What you'd change, add, or stop doing if you had more time
• Your commit history
```

### Deliverable Format & Rules
- **Submission mechanism**: `[In-browser code / Git repo / Zip file / Architecture essay / Text box]`
- **Evaluation criteria / provided tests**: `[None / Unit test suite to satisfy / Performance benchmark]`
- **Dependencies / runtime limits**: `[Standard library only / Specific packages allowed / Offline / Network allowed]`

---

## 3. Assumptions Log
*Record each assumption as a single line as soon as an ambiguity is encountered:*
1. `Assumption 1: ...`
2. `Assumption 2: ...`

---

## 4. Architecture & Data Boundaries

- **Chosen Language**: `[Rust / Python / TS]` — *Justification: ...*
- **Data Ownership / Module Boundaries**:
  - `Module A`: owns ...
  - `Module B`: owns ...
- **Core Interfaces / Types**:
  - ...

---

## 5. Explicit Tradeoffs (Rubric Formula)

> Template:  
> *"Given [constraint], I chose [X] over [Y]. What it buys: [benefit]. What it costs: [cost]. That cost is acceptable because [reason]. I would revisit if [trigger]."*

### Tradeoff 1: Architecture / Data Model
- **Given**: 
- **I chose**: 
- **Over**: 
- **What it buys**: 
- **What it costs**: 
- **Why acceptable**: 
- **Revisit trigger**: 

### Tradeoff 2: Concurrency / Storage / Algorithmic
- **Given**: 
- **I chose**: 
- **Over**: 
- **What it buys**: 
- **What it costs**: 
- **Why acceptable**: 
- **Revisit trigger**: 

---

## 6. Testing Matrix (Arrange / Act / Assert)

| Category | Test Case | Target Bug Class / Scenario | Status |
| :--- | :--- | :--- | :--- |
| **Happy Path** | `test_nominal_flow` | Standard valid input produces expected output | `[ ]` |
| **Boundary** | `test_empty_input` | Empty dataset / zero-length inputs | `[ ]` |
| **Boundary** | `test_single_item` | Single item / off-by-one fencepost checks | `[ ]` |
| **Boundary** | `test_max_capacity` | Large / saturation limit / overflow bounds | `[ ]` |
| **Error Path** | `test_malformed_input` | Schema mismatch, invalid types, corrupted payload | `[ ]` |
| **Error Path** | `test_duplicate_or_conflict`| Duplicate keys, idempotency violations, concurrent update | `[ ]` |
| **Production** | `test_idempotency_or_recovery` | Repeated calls yield identical state; safe failure | `[ ]` |

---

## 7. Production Mindset & Operational Readiness

- [ ] **Observability**: Structured log events with operational context (correlation / request ID, object ID).
- [ ] **Data Integrity**: Input sanitization and domain validation separated from parsing.
- [ ] **Error Semantics**: Differentiated error types (client/validation error vs internal/system fault).
- [ ] **Graceful Degradation**: Clear failure messages; no panics/unhandled exceptions on bad input.

---

## 8. "With More Time" (Record as you go)
*Capture ideas and optimizations here immediately so you don't lose time implementing them now:*
1. 
2. 
3. 

---

## 9. Final Submission Checklist

- [ ] Does the solution strictly answer what was asked in the prompt?
- [ ] Are all test cases passing cleanly?
- [ ] Does the code have clean documentation and zero leftover debug prints/scratch code?
- [ ] Is the tradeoff template completed and clear?
- [ ] Are the 3 required wrap-up sections present: **Assumptions**, **Tradeoffs**, and **With More Time**?
- [ ] Zero unverified claims or defense/clearance implications?
