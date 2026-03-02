-- Agent System Migration
-- Adds tables for agent invocations, inter-agent messages, and discovered skill patterns

-- Agent invocation records (from AgentMetricsCollector)
CREATE TABLE IF NOT EXISTS agent_invocations (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    task TEXT NOT NULL,
    skill_id TEXT,
    team_id TEXT,
    latency_ms INTEGER NOT NULL,
    success INTEGER NOT NULL,
    confidence REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Agent messages (from AgentMailbox)
CREATE TABLE IF NOT EXISTS agent_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    from_agent TEXT NOT NULL,
    to_agent TEXT,
    content TEXT NOT NULL,
    message_type TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Discovered skill patterns (from SkillDiscovery)
CREATE TABLE IF NOT EXISTS discovered_skills (
    id TEXT PRIMARY KEY,
    tool_chain TEXT NOT NULL,
    occurrences INTEGER NOT NULL,
    avg_success_rate REAL NOT NULL,
    materialized INTEGER NOT NULL DEFAULT 0,
    skill_id TEXT,
    discovered_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_agent_invocations_agent ON agent_invocations(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_invocations_session ON agent_invocations(session_id);
CREATE INDEX IF NOT EXISTS idx_agent_invocations_created ON agent_invocations(created_at);
CREATE INDEX IF NOT EXISTS idx_agent_messages_session ON agent_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_agent_messages_from ON agent_messages(from_agent);
CREATE INDEX IF NOT EXISTS idx_agent_messages_to ON agent_messages(to_agent);
CREATE INDEX IF NOT EXISTS idx_discovered_skills_materialized ON discovered_skills(materialized);
