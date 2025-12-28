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

-- Indexes
CREATE INDEX IF NOT EXISTS idx_thoughts_session ON thoughts(session_id);
CREATE INDEX IF NOT EXISTS idx_thoughts_parent ON thoughts(parent_id);
CREATE INDEX IF NOT EXISTS idx_branches_session ON branches(session_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_session ON checkpoints(session_id);
CREATE INDEX IF NOT EXISTS idx_graph_nodes_session ON graph_nodes(session_id);
CREATE INDEX IF NOT EXISTS idx_graph_edges_session ON graph_edges(session_id);
CREATE INDEX IF NOT EXISTS idx_metrics_mode ON metrics(mode);
CREATE INDEX IF NOT EXISTS idx_metrics_created ON metrics(created_at);
CREATE INDEX IF NOT EXISTS idx_self_improvement_status ON self_improvement_actions(status);
