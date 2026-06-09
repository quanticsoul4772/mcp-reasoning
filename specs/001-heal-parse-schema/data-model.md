# Phase 1 Data Model: Self-Healing of Parse/Schema Failures

Three persisted entities (SQLite, via the existing SI storage layer). Field types are conceptual.

## DefectRecord

A detected failure of the server's own output.

| Field | Type | Notes |
|---|---|---|
| id | id | primary key |
| component | string | originating tool + mode (e.g. `reasoning_linear`/`linear`) |
| failure_class | enum | `Parse` \| `Schema` \| `Drift` |
| trigger_input_hash | string | content hash of the offending input (raw input not stored in cleartext if sensitive) |
| trigger_excerpt | string | bounded, redacted excerpt for diagnosis |
| first_seen | timestamp | |
| last_seen | timestamp | |
| occurrences | int | recurrence counter |
| status | enum | `Observed` \| `Recurring` \| `Proposed` \| `Resolved` \| `DriftRouted` |

Validation rules: `occurrences ≥ 1`; a record transitions `Observed → Recurring` when
`occurrences ≥ N` (D2); `failure_class = Drift` records never enter the repair path (FR-012).

State transitions:
`Observed → Recurring → Proposed → Resolved` (happy path);
`Observed/Recurring → DriftRouted` (D3); `Proposed → Recurring` (proposal rejected, re-queue).

## FixProposal

A candidate change for a recurring DefectRecord.

| Field | Type | Notes |
|---|---|---|
| id | id | primary key |
| defect_id | id | FK → DefectRecord |
| branch | string | generated branch name |
| change_summary | string | what the fix does |
| reproducing_test_ref | string | path/name of the generated test |
| grounded | bool | test demonstrably failed on unpatched code (D4); must be true to open a PR |
| suite_green | bool | full `cargo test` passed with the fix |
| quality_green | bool | fmt + clippy + rustc passed |
| pr_url | string | `gh` PR URL once opened |
| review_status | enum | `Proposed` \| `Approved` \| `Rejected` |

Validation rules: a PR is opened only if `grounded = true` (FR-006); a proposal is admissible only if
`grounded ∧ suite_green ∧ quality_green` (FR-008); `review_status` is set by the operator, never the
loop (FR-007); the proposal may not modify acceptance/measurement files (D6).

## KnowledgeEntry

An accepted defect→fix→test mapping, reused on recurrence.

| Field | Type | Notes |
|---|---|---|
| id | id | primary key |
| failure_signature | string | `(component, failure_class)` key |
| fix_summary | string | |
| test_ref | string | the reproducing test that guards the class |
| accepted_at | timestamp | |

Validation rules: keyed uniquely by `failure_signature`; on a new DefectRecord whose signature
matches an entry, the loop references the entry and does not re-diagnose (FR-011, SC-006).
