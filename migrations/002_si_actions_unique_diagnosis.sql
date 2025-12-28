-- Migration: Add unique constraint on diagnosis_id in si_actions
-- This ensures one action per diagnosis (1:1 relationship)
-- Version: 2
--
-- Rationale:
-- - Prevents duplicate action execution for same diagnosis
-- - Ensures consistent state in learning records
-- - Enforces data integrity in the self-improvement cycle

-- Drop existing non-unique index
DROP INDEX IF EXISTS idx_si_actions_diagnosis;

-- Create unique index to enforce 1:1 diagnosis-to-action relationship
CREATE UNIQUE INDEX IF NOT EXISTS idx_si_actions_diagnosis_unique
ON si_actions(diagnosis_id);
