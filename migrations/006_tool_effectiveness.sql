-- Tool effectiveness tracking table
-- Records aggregated effectiveness data per tool per context
CREATE TABLE IF NOT EXISTS tool_effectiveness (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool_name TEXT NOT NULL,
    context_tag TEXT NOT NULL,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    total_quality REAL NOT NULL DEFAULT 0.0,
    quality_sample_count INTEGER NOT NULL DEFAULT 0,
    total_latency_ms INTEGER NOT NULL DEFAULT 0,
    sample_count INTEGER NOT NULL DEFAULT 0,
    last_updated TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(tool_name, context_tag)
);

CREATE INDEX IF NOT EXISTS idx_tool_effectiveness_tool ON tool_effectiveness(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_effectiveness_context ON tool_effectiveness(context_tag);
CREATE INDEX IF NOT EXISTS idx_tool_effectiveness_tool_context ON tool_effectiveness(tool_name, context_tag);
