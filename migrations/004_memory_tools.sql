-- Memory Tools Migration
-- Adds tables for session embeddings and relationships

-- Session embeddings cache
CREATE TABLE IF NOT EXISTS session_embeddings (
    session_id TEXT PRIMARY KEY,
    embedding_json TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Embedding generation queue
CREATE TABLE IF NOT EXISTS embedding_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    processed_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Session relationships (cached)
CREATE TABLE IF NOT EXISTS session_relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_session_id TEXT NOT NULL,
    to_session_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    strength REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (from_session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (to_session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    UNIQUE(from_session_id, to_session_id, relationship_type)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_session_embeddings_session ON session_embeddings(session_id);
CREATE INDEX IF NOT EXISTS idx_embedding_queue_status ON embedding_queue(status);
CREATE INDEX IF NOT EXISTS idx_relationships_from ON session_relationships(from_session_id);
CREATE INDEX IF NOT EXISTS idx_relationships_to ON session_relationships(to_session_id);
