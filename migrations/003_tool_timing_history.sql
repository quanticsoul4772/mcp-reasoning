-- Tool timing history for duration predictions
CREATE TABLE IF NOT EXISTS tool_timing_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool_name TEXT NOT NULL,
    mode_name TEXT,
    duration_ms INTEGER NOT NULL,
    complexity_score INTEGER NOT NULL,
    timestamp INTEGER NOT NULL
);

-- Index for efficient lookups
CREATE INDEX IF NOT EXISTS idx_timing_lookup 
    ON tool_timing_history(tool_name, mode_name, timestamp);

-- Index for cleanup of old data
CREATE INDEX IF NOT EXISTS idx_timing_timestamp 
    ON tool_timing_history(timestamp);
