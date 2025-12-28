-- MCP Reasoning Server Database Schema
-- Version: 1

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    metadata TEXT -- JSON metadata
);

-- Thoughts table
CREATE TABLE IF NOT EXISTS thoughts (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    parent_id TEXT REFERENCES thoughts(id) ON DELETE SET NULL,
    mode TEXT NOT NULL,
    content TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.0,
    metadata TEXT, -- JSON metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Branches table (for tree mode)
CREATE TABLE IF NOT EXISTS branches (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    parent_branch_id TEXT REFERENCES branches(id) ON DELETE SET NULL,
    content TEXT NOT NULL,
    score REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'active', -- active, completed, abandoned
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Checkpoints table
CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    state TEXT NOT NULL, -- JSON serialized state
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Graph nodes table (for GoT mode)
CREATE TABLE IF NOT EXISTS graph_nodes (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    node_type TEXT NOT NULL DEFAULT 'thought', -- thought, aggregation, refinement
    score REAL,
    is_terminal INTEGER NOT NULL DEFAULT 0,
    metadata TEXT, -- JSON metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Graph edges table
CREATE TABLE IF NOT EXISTS graph_edges (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    from_node_id TEXT NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    to_node_id TEXT NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    edge_type TEXT NOT NULL DEFAULT 'continues', -- continues, aggregates, refines
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Metrics table
CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mode TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    latency_ms INTEGER NOT NULL,
    success INTEGER NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Self-improvement actions table
CREATE TABLE IF NOT EXISTS self_improvement_actions (
    id TEXT PRIMARY KEY,
    action_type TEXT NOT NULL,
    parameters TEXT NOT NULL, -- JSON
    status TEXT NOT NULL DEFAULT 'pending', -- pending, executing, completed, failed, rolled_back
    result TEXT, -- JSON
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- ============================================================================
-- Self-Improvement System Tables (DESIGN.md Section 14)
-- ============================================================================

-- Invocation records (fed by Monitor)
CREATE TABLE IF NOT EXISTS invocations (
    id TEXT PRIMARY KEY,
    tool_name TEXT NOT NULL,
    latency_ms INTEGER NOT NULL,
    success INTEGER NOT NULL,
    quality_score REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Diagnosis records (from Analyzer)
CREATE TABLE IF NOT EXISTS diagnoses (
    id TEXT PRIMARY KEY,
    trigger_type TEXT NOT NULL,
    trigger_json TEXT NOT NULL,
    severity TEXT NOT NULL,
    description TEXT NOT NULL,
    suspected_cause TEXT,
    suggested_action_json TEXT NOT NULL,
    action_rationale TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Action records (executed by Executor)
CREATE TABLE IF NOT EXISTS si_actions (
    id TEXT PRIMARY KEY,
    diagnosis_id TEXT NOT NULL REFERENCES diagnoses(id),
    action_type TEXT NOT NULL,
    action_json TEXT NOT NULL,
    outcome TEXT NOT NULL DEFAULT 'pending',
    pre_metrics_json TEXT NOT NULL,
    post_metrics_json TEXT,
    execution_time_ms INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Learning records (from Learner)
CREATE TABLE IF NOT EXISTS learnings (
    id TEXT PRIMARY KEY,
    action_id TEXT NOT NULL REFERENCES si_actions(id),
    reward_value REAL NOT NULL,
    reward_breakdown_json TEXT NOT NULL,
    confidence REAL NOT NULL,
    lessons_json TEXT,
    recommendations_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Config overrides (applied by Executor, read at startup)
CREATE TABLE IF NOT EXISTS config_overrides (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    applied_by_action TEXT REFERENCES si_actions(id),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Indexes
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_thoughts_session ON thoughts(session_id);
CREATE INDEX IF NOT EXISTS idx_thoughts_parent ON thoughts(parent_id);
CREATE INDEX IF NOT EXISTS idx_branches_session ON branches(session_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_session ON checkpoints(session_id);
CREATE INDEX IF NOT EXISTS idx_graph_nodes_session ON graph_nodes(session_id);
CREATE INDEX IF NOT EXISTS idx_graph_edges_session ON graph_edges(session_id);
CREATE INDEX IF NOT EXISTS idx_metrics_mode ON metrics(mode);
CREATE INDEX IF NOT EXISTS idx_metrics_created ON metrics(created_at);
CREATE INDEX IF NOT EXISTS idx_self_improvement_status ON self_improvement_actions(status);

-- Self-improvement indexes
CREATE INDEX IF NOT EXISTS idx_invocations_created_at ON invocations(created_at);
CREATE INDEX IF NOT EXISTS idx_invocations_tool ON invocations(tool_name);
CREATE INDEX IF NOT EXISTS idx_diagnoses_status ON diagnoses(status);
CREATE INDEX IF NOT EXISTS idx_si_actions_diagnosis ON si_actions(diagnosis_id);
CREATE INDEX IF NOT EXISTS idx_si_actions_outcome ON si_actions(outcome);
CREATE INDEX IF NOT EXISTS idx_learnings_action ON learnings(action_id);
