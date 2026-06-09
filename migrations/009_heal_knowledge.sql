-- Migration 009: Persist self-heal knowledge entries (spec 001, FR-011).
--
-- A KnowledgeEntry is an accepted defect->fix->test mapping, keyed by failure
-- signature `(component, failure_class)`. Persisting it lets the self-heal loop
-- recognize a previously-fixed failure class on a later run and skip
-- re-diagnosing it (SC-006), surviving restarts. Defect recurrence itself is
-- session-scoped (FR-003) and stays in process memory; only the durable
-- knowledge is persisted here.
CREATE TABLE IF NOT EXISTS heal_knowledge_entries (
    id                TEXT PRIMARY KEY,
    failure_signature TEXT    NOT NULL UNIQUE,
    fix_summary       TEXT    NOT NULL,
    test_ref          TEXT    NOT NULL,
    accepted_at       INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_heal_knowledge_signature
    ON heal_knowledge_entries (failure_signature);
