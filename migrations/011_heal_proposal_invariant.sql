-- Migration 011: persist the validation-invariant guard verdict on a fix proposal
-- (spec 002, US1). `weakens_invariant` (0/1) disqualifies admissibility so the
-- operator-accept path enforces it across restarts; `block_reason` is the
-- operator-visible explanation (FR-009). Executed statement-by-statement with
-- "duplicate column" tolerance (the table already exists from migration 010).
ALTER TABLE heal_fix_proposals ADD COLUMN weakens_invariant INTEGER NOT NULL DEFAULT 0;
ALTER TABLE heal_fix_proposals ADD COLUMN block_reason TEXT;
