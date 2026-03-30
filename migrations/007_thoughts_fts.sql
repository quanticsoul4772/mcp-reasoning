-- Full-Text Search for Reasoning Sessions
-- Uses SQLite FTS5 to index thought content for semantic keyword search.
-- Replaces hash-based embedding similarity with BM25-ranked text search.

-- FTS5 virtual table: indexes thought content, groups by session
CREATE VIRTUAL TABLE IF NOT EXISTS thoughts_fts USING fts5(
    session_id UNINDEXED,
    content,
    content='thoughts',
    content_rowid='rowid'
);

-- Populate from existing data
INSERT INTO thoughts_fts(rowid, session_id, content)
SELECT rowid, session_id, content FROM thoughts;

-- Triggers to keep FTS5 index in sync with thoughts table
CREATE TRIGGER IF NOT EXISTS thoughts_fts_ai AFTER INSERT ON thoughts BEGIN
    INSERT INTO thoughts_fts(rowid, session_id, content)
    VALUES (new.rowid, new.session_id, new.content);
END;

CREATE TRIGGER IF NOT EXISTS thoughts_fts_ad AFTER DELETE ON thoughts BEGIN
    INSERT INTO thoughts_fts(thoughts_fts, rowid, session_id, content)
    VALUES ('delete', old.rowid, old.session_id, old.content);
END;

CREATE TRIGGER IF NOT EXISTS thoughts_fts_au AFTER UPDATE ON thoughts BEGIN
    INSERT INTO thoughts_fts(thoughts_fts, rowid, session_id, content)
    VALUES ('delete', old.rowid, old.session_id, old.content);
    INSERT INTO thoughts_fts(rowid, session_id, content)
    VALUES (new.rowid, new.session_id, new.content);
END;
