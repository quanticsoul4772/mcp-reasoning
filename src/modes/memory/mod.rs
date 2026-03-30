//! Memory access tools for querying past reasoning sessions.
//!
//! This module provides tools to expose existing memory capabilities:
//! - `list_sessions` - List all past sessions with summaries
//! - `resume_session` - Load full context from a past session
//! - `search_sessions` - Semantic search over reasoning history
//! - `relate_sessions` - Show relationships between sessions

mod cluster;
mod embeddings;
mod list;
mod relate;
mod resume;
mod search;
mod types;

pub use list::list_sessions;
pub use relate::relate_sessions;
pub use resume::resume_session;
pub use search::search_sessions;
pub use types::{
    RelationshipGraph, RelationshipType, SearchResult, SessionContext, SessionSummary,
};
