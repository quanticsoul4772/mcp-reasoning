# Memory Tools Implementation Plan

**Date**: 2026-03-02
**Target Version**: v0.2.0
**Timeline**: 2-3 weeks

---

## Overview

Implement 4 tools to expose existing memory capabilities:

1. `reasoning_list_sessions` - List all past sessions with summaries
2. `reasoning_resume` - Load full context from a session
3. `reasoning_search` - Semantic search over past reasoning
4. `reasoning_relate` - Show relationships between sessions

**Key Insight**: All data already exists in SQLite. Just need query/access layer.

---

## Implementation Order (Dependency-Driven)

### Phase 1: Foundation (Days 1-3)

**Tool**: `reasoning_list_sessions`

**Why First**:

- Simplest to implement
- No dependencies
- Establishes API patterns
- Provides immediate value

**Implementation**:

```rust
// src/modes/memory/list.rs
pub async fn list_sessions(
    storage: &SqliteStorage,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<SessionSummary>, ModeError> {
    // Query sessions table
    // Aggregate thought counts
    // Get first thought as preview
    // Return sorted by updated_at desc
}
```

**SQL Query**:

```sql
SELECT
    s.id,
    s.created_at,
    s.updated_at,
    COUNT(t.id) as thought_count,
    (SELECT content FROM thoughts WHERE session_id = s.id ORDER BY created_at LIMIT 1) as first_thought
FROM sessions s
LEFT JOIN thoughts t ON s.id = t.session_id
GROUP BY s.id
ORDER BY s.updated_at DESC
LIMIT ? OFFSET ?;
```

**API Schema**:

```json
{
  "name": "reasoning_list_sessions",
  "inputSchema": {
    "properties": {
      "limit": { "type": "integer", "default": 20 },
      "offset": { "type": "integer", "default": 0 },
      "mode_filter": { "type": "string" }
    }
  },
  "outputSchema": {
    "properties": {
      "sessions": {
        "type": "array",
        "items": {
          "properties": {
            "session_id": { "type": "string" },
            "created_at": { "type": "string" },
            "updated_at": { "type": "string" },
            "thought_count": { "type": "integer" },
            "preview": { "type": "string" },
            "primary_mode": { "type": "string" }
          }
        }
      },
      "total": { "type": "integer" },
      "has_more": { "type": "boolean" }
    }
  }
}
```

**Testing**:

- [ ] Create test session with thoughts
- [ ] Verify pagination works
- [ ] Test mode_filter
- [ ] Test empty database
- [ ] Test limit edge cases

**Deliverable**: Agent can see all past sessions with basic info

---

### Phase 2: Context Loading (Days 4-6)

**Tool**: `reasoning_resume`

**Why Second**:

- Builds on list_sessions
- High immediate value
- Moderate complexity

**Implementation**:

```rust
// src/modes/memory/resume.rs
pub async fn resume_session(
    storage: &SqliteStorage,
    session_id: &str,
    include_checkpoints: bool,
) -> Result<SessionContext, ModeError> {
    // Load session metadata
    // Load all thoughts chronologically
    // Load branches if tree mode was used
    // Load latest checkpoint if requested
    // Format as resumable context
}
```

**SQL Queries**:

```sql
-- Main session
SELECT * FROM sessions WHERE id = ?;

-- All thoughts
SELECT id, parent_id, mode, content, confidence, created_at
FROM thoughts
WHERE session_id = ?
ORDER BY created_at;

-- Latest checkpoint
SELECT id, name, description, state
FROM checkpoints
WHERE session_id = ?
ORDER BY created_at DESC
LIMIT 1;

-- Branches (if tree mode)
SELECT id, content, score, status
FROM branches
WHERE session_id = ?;
```

**API Schema**:

```json
{
  "name": "reasoning_resume",
  "inputSchema": {
    "properties": {
      "session_id": { "type": "string" },
      "include_checkpoints": { "type": "boolean", "default": true },
      "compress": { "type": "boolean", "default": false }
    },
    "required": ["session_id"]
  },
  "outputSchema": {
    "properties": {
      "session_id": { "type": "string" },
      "created_at": { "type": "string" },
      "context": {
        "type": "object",
        "properties": {
          "summary": { "type": "string" },
          "thought_chain": { "type": "array" },
          "key_conclusions": { "type": "array" },
          "last_mode": { "type": "string" }
        }
      },
      "checkpoint": { "type": "object" },
      "continuation_suggestions": { "type": "array" }
    }
  }
}
```

**Compression Option**:
If `compress: true`, use Claude to summarize:

```
Given this reasoning chain of {N} thoughts, compress to key insights:
- What was being reasoned about?
- What approach was used?
- What conclusions were reached?
- What uncertainties remain?
```

**Testing**:

- [ ] Resume session with linear thoughts
- [ ] Resume session with tree branches
- [ ] Resume with checkpoint
- [ ] Resume non-existent session (error handling)
- [ ] Test compression feature
- [ ] Verify context completeness

**Deliverable**: Agent can fully resume any past session with context

---

### Phase 3: Semantic Search (Days 7-12)

**Tool**: `reasoning_search`

**Why Third**:

- Most complex (needs embeddings)
- High value once working
- Requires new infrastructure

**Implementation Options**:

#### Option A: Claude Embeddings API (Recommended)

```rust
// src/modes/memory/search.rs
pub async fn search_sessions(
    storage: &SqliteStorage,
    client: &AnthropicClient,
    query: &str,
    limit: u32,
) -> Result<Vec<SearchResult>, ModeError> {
    // 1. Get embedding for query
    let query_embedding = get_embedding(client, query).await?;

    // 2. Get all session embeddings from cache
    let session_embeddings = load_session_embeddings(storage).await?;

    // 3. Compute cosine similarity
    let similarities = compute_similarities(&query_embedding, &session_embeddings);

    // 4. Sort and return top N
    let results = similarities.top_k(limit);

    // 5. Load full session data for results
    load_session_details(storage, results).await
}
```

#### Option B: SQLite-VSS Extension

```rust
// Requires building with sqlite-vss
// Adds vector search directly in SQLite
```

**New Database Tables**:

```sql
-- Session embeddings cache
CREATE TABLE IF NOT EXISTS session_embeddings (
    session_id TEXT PRIMARY KEY,
    embedding_json TEXT NOT NULL,  -- JSON array of floats
    content_hash TEXT NOT NULL,     -- To detect changes
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Embedding generation queue
CREATE TABLE IF NOT EXISTS embedding_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, processing, completed, failed
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);
```

**Embedding Generation Strategy**:

```rust
// Generate embeddings lazily
async fn ensure_embeddings(
    storage: &SqliteStorage,
    client: &AnthropicClient,
) -> Result<(), ModeError> {
    // Find sessions without embeddings
    let missing = storage.sessions_without_embeddings().await?;

    for session_id in missing {
        // Get session summary
        let summary = create_session_summary(storage, &session_id).await?;

        // Get embedding from Claude
        let embedding = client.get_embedding(&summary).await?;

        // Cache in database
        storage.store_embedding(&session_id, &embedding).await?;
    }

    Ok(())
}
```

**API Schema**:

```json
{
  "name": "reasoning_search",
  "inputSchema": {
    "properties": {
      "query": { "type": "string" },
      "limit": { "type": "integer", "default": 5 },
      "min_similarity": { "type": "number", "default": 0.7 },
      "mode_filter": { "type": "string" }
    },
    "required": ["query"]
  },
  "outputSchema": {
    "properties": {
      "results": {
        "type": "array",
        "items": {
          "properties": {
            "session_id": { "type": "string" },
            "similarity_score": { "type": "number" },
            "preview": { "type": "string" },
            "created_at": { "type": "string" },
            "primary_mode": { "type": "string" }
          }
        }
      },
      "embeddings_cached": { "type": "integer" },
      "embeddings_generated": { "type": "integer" }
    }
  }
}
```

**Testing**:

- [ ] Generate embeddings for test sessions
- [ ] Verify cosine similarity calculations
- [ ] Test semantic matching (not just keyword)
- [ ] Test with no results
- [ ] Test mode_filter
- [ ] Performance test with 100+ sessions
- [ ] Test embedding cache invalidation

**Deliverable**: Agent can find relevant past reasoning semantically

---

### Phase 4: Relationship Mapping (Days 13-15)

**Tool**: `reasoning_relate`

**Why Last**:

- Builds on all previous tools
- Most complex analysis
- Enhancement rather than core feature

**Implementation**:

```rust
// src/modes/memory/relate.rs
pub async fn relate_sessions(
    storage: &SqliteStorage,
    session_id: Option<String>,
    depth: u32,
) -> Result<RelationshipGraph, ModeError> {
    if let Some(id) = session_id {
        // Find relationships for specific session
        analyze_session_relationships(storage, &id, depth).await
    } else {
        // Build full relationship graph
        analyze_all_relationships(storage).await
    }
}

async fn analyze_session_relationships(
    storage: &SqliteStorage,
    session_id: &str,
    depth: u32,
) -> Result<RelationshipGraph, ModeError> {
    let mut graph = RelationshipGraph::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back((session_id.to_string(), 0));

    while let Some((current_id, current_depth)) = queue.pop_front() {
        if current_depth > depth || visited.contains(&current_id) {
            continue;
        }
        visited.insert(current_id.clone());

        // Find related sessions
        let related = find_related_sessions(storage, &current_id).await?;

        for (related_id, relationship_type, strength) in related {
            graph.add_edge(&current_id, &related_id, relationship_type, strength);

            if current_depth < depth {
                queue.push_back((related_id, current_depth + 1));
            }
        }
    }

    Ok(graph)
}
```

**Relationship Detection Logic**:

```rust
enum RelationshipType {
    ContinuesFrom,      // Session B explicitly resumes session A
    SimilarTopic,       // High embedding similarity
    SharedMode,         // Both use same reasoning mode
    TemporallyAdjacent, // Created within short time window
    CommonConclusion,   // Arrive at similar conclusions
}

async fn find_related_sessions(
    storage: &SqliteStorage,
    session_id: &str,
) -> Result<Vec<(String, RelationshipType, f64)>, StorageError> {
    let mut relationships = Vec::new();

    // Check for explicit continuations (metadata)
    relationships.extend(find_explicit_continuations(storage, session_id).await?);

    // Check embedding similarity
    relationships.extend(find_similar_sessions(storage, session_id, 0.75).await?);

    // Check shared modes
    relationships.extend(find_mode_related(storage, session_id).await?);

    // Check temporal proximity
    relationships.extend(find_temporal_neighbors(storage, session_id).await?);

    Ok(relationships)
}
```

**API Schema**:

```json
{
  "name": "reasoning_relate",
  "inputSchema": {
    "properties": {
      "session_id": { "type": "string" },
      "depth": { "type": "integer", "default": 2 },
      "min_strength": { "type": "number", "default": 0.5 },
      "include_types": {
        "type": "array",
        "items": { "type": "string" }
      }
    }
  },
  "outputSchema": {
    "properties": {
      "nodes": {
        "type": "array",
        "items": {
          "properties": {
            "session_id": { "type": "string" },
            "preview": { "type": "string" },
            "created_at": { "type": "string" }
          }
        }
      },
      "edges": {
        "type": "array",
        "items": {
          "properties": {
            "from_session": { "type": "string" },
            "to_session": { "type": "string" },
            "relationship_type": { "type": "string" },
            "strength": { "type": "number" }
          }
        }
      },
      "clusters": {
        "type": "array",
        "items": {
          "properties": {
            "sessions": { "type": "array" },
            "common_theme": { "type": "string" }
          }
        }
      }
    }
  }
}
```

**Testing**:

- [ ] Test single session relationships
- [ ] Test depth traversal
- [ ] Test relationship type filtering
- [ ] Test clustering algorithm
- [ ] Test with isolated sessions
- [ ] Performance test with large graphs

**Deliverable**: Agent can navigate knowledge graph of reasoning history

---

## Module Structure

```
src/modes/memory/
├── mod.rs           # Public API + exports
├── list.rs          # reasoning_list_sessions
├── resume.rs        # reasoning_resume
├── search.rs        # reasoning_search
├── relate.rs        # reasoning_relate
├── embeddings.rs    # Embedding generation + caching
└── types.rs         # SessionSummary, SearchResult, RelationshipGraph
```

---

## Database Migrations

### Migration: 004_memory_tools.sql

```sql
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
```

---

## Testing Strategy

### Unit Tests

- [ ] Each query function tested independently
- [ ] Edge cases: empty DB, single session, many sessions
- [ ] Error handling: invalid IDs, DB errors
- [ ] Pagination logic
- [ ] Embedding calculation accuracy

### Integration Tests

- [ ] End-to-end: create session → list → resume → search
- [ ] Cross-tool: search results can be resumed
- [ ] Relationship detection accuracy
- [ ] Performance with 1000+ sessions

### Performance Benchmarks

- [ ] List 100 sessions: <100ms
- [ ] Resume session with 50 thoughts: <200ms
- [ ] Search with embeddings: <500ms
- [ ] Relate depth=3: <1000ms

---

## API Registration

```rust
// src/server/tools.rs

#[tool_router]
pub fn create_tool_router() -> ToolRouter {
    ToolRouter::new()
        // Existing tools...
        .tool(reasoning_linear_handler)
        .tool(reasoning_tree_handler)
        // ... other existing tools ...

        // NEW: Memory tools
        .tool(reasoning_list_sessions_handler)
        .tool(reasoning_resume_handler)
        .tool(reasoning_search_handler)
        .tool(reasoning_relate_handler)
}

// Handler implementations
async fn reasoning_list_sessions_handler(
    state: State<AppState>,
    params: ListSessionsParams,
) -> Result<ListSessionsResponse, ToolError> {
    let storage = &state.storage;
    list::list_sessions(storage, params.limit, params.offset).await
        .map_err(|e| ToolError::execution_error(e.to_string()))
}

// Similar handlers for resume, search, relate...
```

---

## Documentation Updates

### README.md

Add to tools table:

| Tool | Description | Operations |
|------|-------------|------------|
| `reasoning_list_sessions` | List past sessions | - |
| `reasoning_resume` | Resume past session | - |
| `reasoning_search` | Semantic search | - |
| `reasoning_relate` | Session relationships | - |

### docs/reference/TOOL_REFERENCE.md

Add full documentation for each tool with examples.

### CHANGELOG.md

```markdown
## [0.2.0] - 2026-03-XX

### Added
- Memory access tools for querying past reasoning
- `reasoning_list_sessions` - List all past sessions
- `reasoning_resume` - Load full context from session
- `reasoning_search` - Semantic search over reasoning
- `reasoning_relate` - Navigate reasoning knowledge graph
- Session embedding cache system
- Relationship detection and graph building
```

---

## Success Metrics

### Phase 1: List Sessions

- [ ] Tool works on first try
- [ ] Pagination handles 1000+ sessions
- [ ] Response time <100ms

### Phase 2: Resume

- [ ] Full context loaded accurately
- [ ] Checkpoint restoration works
- [ ] Compression reduces tokens by 70%+

### Phase 3: Search

- [ ] Semantic matching beats keyword search
- [ ] Top result relevant 80%+ of time
- [ ] Response time <500ms

### Phase 4: Relate

- [ ] Relationship detection accuracy >75%
- [ ] Graph navigation intuitive
- [ ] Performance acceptable (depth=3 <1s)

---

## Timeline Summary

| Phase | Days | Tool | Status |
|-------|------|------|--------|
| 1 | 1-3 | reasoning_list_sessions | Not Started |
| 2 | 4-6 | reasoning_resume | Not Started |
| 3 | 7-12 | reasoning_search | Not Started |
| 4 | 13-15 | reasoning_relate | Not Started |

**Total**: 15 working days (~3 weeks)

---

## Next Steps

1. Review this implementation plan
2. Create feature branch: `feature/memory-tools`
3. Start with Phase 1: reasoning_list_sessions
4. Iterate based on testing feedback
5. Document as we go
6. Release as v0.2.0

---

**Status**: Ready to implement
**Owner**: TBD
**Target**: v0.2.0
