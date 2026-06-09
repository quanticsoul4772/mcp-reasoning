-- Migration 010: Persist self-heal fix proposals (spec 001, US3).
--
-- A FixProposal is the outcome of the propose-PR pipeline for one recurring
-- defect: a branch, a reproducing test, the validation verdicts, an optional PR
-- URL, and an operator review state. Persisting it lets the operator approve or
-- reject a proposal produced in an earlier cycle/session, and lets the loop
-- enforce "merge requires admissible AND operator-approved" across restarts
-- (FR-008/FR-009). The loop can never self-approve: review_status starts
-- 'proposed' and only an operator override moves it to 'approved'/'rejected'.
CREATE TABLE IF NOT EXISTS heal_fix_proposals (
    id                   TEXT PRIMARY KEY,
    defect_id            TEXT    NOT NULL,
    failure_signature    TEXT    NOT NULL,
    branch               TEXT    NOT NULL,
    change_summary       TEXT    NOT NULL,
    reproducing_test_ref TEXT    NOT NULL,
    grounded             INTEGER NOT NULL,
    suite_green          INTEGER NOT NULL,
    quality_green        INTEGER NOT NULL,
    pr_url               TEXT,
    review_status        TEXT    NOT NULL,
    created_at           INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_heal_fix_proposals_defect
    ON heal_fix_proposals (defect_id);

CREATE INDEX IF NOT EXISTS idx_heal_fix_proposals_review
    ON heal_fix_proposals (review_status);
