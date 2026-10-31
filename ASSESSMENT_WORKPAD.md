# Assessment Workpad

Use this document to plan a time-boxed engineering assessment and record the method used. Treat the submitted prompt as the source of truth. Remove unused notes and verify the final artifacts before sharing.

## 1. Time Budget and Milestones

This schedule allocates a 180-minute work budget. Adjust it only if the prompt specifies a different limit.

| Phase | Window | Target milestone |
| :--- | :--- | :--- |
| 1. Triage and framing | 00:00–00:15 | Read the prompt twice, identify constraints, state assumptions, choose an initial scope. |
| 2. Walking skeleton | 00:15–00:45 | Get one end-to-end slice and one automated check working. |
| 3. Core implementation | 00:45–01:25 | Implement the core behavior and tests for its main rules. |
| 4. Error paths and edges | 01:25–02:00 | Cover validation, boundaries, malformed input, and error classification. |
| 5. Integration and robustness | 02:00–02:30 | Verify integration behavior, concurrency, and high-risk edge cases. |
| 6. Write-up and packaging | 02:30–03:00 | Finish the README, verify claims, review the diff, and prepare the repository. |

## 2. Prompt and Constraints

### The prompt

~~~text
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
~~~

### Constraints to extract

Before implementation, identify the requested deliverable, evaluation criteria or supplied tests, runtime limits, and dependency constraints. Use only information stated in the prompt.

## 3. Assumptions Log

For each important ambiguity, record one concise assumption and proceed on it unless the answer would change the design materially. Keep facts and assumptions distinct.

## 4. Architecture and Data Boundaries

Record the chosen language and why it fits the prompt. Define which module owns each kind of data and behavior. Name the core interfaces and invariants that need to remain true.

## 5. Explicit Tradeoffs

For each major decision, record:

- The constraint that drove the choice.
- The selected option and the alternative.
- What the choice buys and what it costs.
- Why the cost is acceptable under the current constraint.
- What would trigger revisiting the decision.

## 6. Testing Matrix

Identify tests at the lowest level that catches each likely bug class.

| Category | Coverage to consider | Bug class |
| :--- | :--- | :--- |
| Happy path | One representative valid flow | Incorrect normal behavior |
| Boundary | Empty, single item, and upper-limit inputs | Off-by-one and limit errors |
| Error path | Malformed input and invalid types | Parsing and validation errors |
| Conflict | Duplicate keys and concurrent updates | Lost updates and uniqueness errors |
| Recovery | Repeated operations and safe failure | Duplicate effects and inconsistent state |

## 7. Production Mindset

Review operational context and request or object identifiers in logs. Separate input parsing from domain validation. Distinguish client errors from internal failures. Confirm that invalid input reaches a clear error response rather than a panic.

## 8. With More Time

Record only work deferred after the requested scope is complete. For each item, explain the constraint that deferred it and the condition that would justify revisiting it.

## 9. Final Submission Pass

1. Re-read the prompt and verify that the deliverable answers it.
2. Run the relevant tests and report their actual results.
3. Review error paths, documentation, and any live example flow.
4. Remove scratch files and debug output that are not part of the deliverable.
5. State assumptions and tradeoffs clearly, including their costs.
6. Remove unverified claims and ensure credentials or experience are represented accurately.
