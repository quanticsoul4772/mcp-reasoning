-- Migration 008: Persist per-action-type self-improvement learning stats.
--
-- The Learner accumulates per-action-type effectiveness (attempts, successes,
-- a rolling average reward, and expected/actual improvement sums) that
-- guidance() feeds into the Analyzer to steer future proposals. Previously this
-- lived only in process memory and reset on every restart, so guidance started
-- blank until the loop re-accumulated.
--
-- Persisting the aggregates lets the effectiveness table survive restarts.
-- Per-lesson textual insights are intentionally NOT persisted (chosen tradeoff:
-- aggregated stats only) — guidance's recent-insights list re-warms at runtime.
CREATE TABLE IF NOT EXISTS si_action_type_stats (
    action_type      TEXT PRIMARY KEY,
    total_executions INTEGER NOT NULL DEFAULT 0,
    successful       INTEGER NOT NULL DEFAULT 0,
    avg_reward       REAL    NOT NULL DEFAULT 0.0,
    total_expected   REAL    NOT NULL DEFAULT 0.0,
    total_actual     REAL    NOT NULL DEFAULT 0.0,
    updated_at       TEXT    NOT NULL
);
