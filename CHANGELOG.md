# Changelog

All notable changes to the MCP Reasoning Server will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-06-10

The tool surface grew to **35 tools** (17 core reasoning, 7 self-improvement,
4 session management, 7 agent/team) and the self-improvement system went from
advisory scaffolding to a wired, measured, self-correcting loop — including a
self-heal path that proposes operator-reviewed PRs for its own recurring defects.

### Added

- **Self-heal propose-PR loop (specs 001 + 002)** — the server detects its own
  recurring parse/schema defects and can open operator-reviewed PRs that fix
  them. OFF by default (`SELF_HEAL_PROPOSE_ENABLED`, `SELF_HEAL_WORKSPACE`);
  it **never merges**. Spec 002 adds two safety guards: a fix that would weaken a
  validation/range/contract check is never admissible, and only stable-path code
  defects are propose-eligible (varied-input recurrence is held back;
  model-version-correlated spikes route to drift). A reproducing-test gate grounds
  every proposal; all `cargo`/`git`/`gh` side effects go through an injectable
  runner so tests never touch the real repo.
- **Self-improvement system, wired end-to-end** — `ThresholdAdjust` now drives
  real, tunable `Config` thresholds (reflection/MCTS quality, graph prune);
  recorded config overrides are applied to the live `Config` at startup (opt-in);
  each cycle's diagnosis and action outcomes are persisted as an audit trail;
  per-action-type learning stats survive restarts; the Learn loop feeds past
  lessons back into Analyze. New `reasoning_si_overrides` exposes advisory
  recommendations; the self-correcting suppression of low-success tool transitions
  is applied every cycle.
- **Streaming milestone progress over MCP** — the long-running modes (divergent,
  reflection, MCTS, counterfactual) deliver milestone progress as
  `notifications/progress`, sent when the client supplies a progress token.
- **Evaluation harness** — `eval::stats` statistical foundation, programmatic
  scorers + report, a real-mode solver/runner, and an opt-in eval binary with
  seed datasets; a real measurement sensor with a divergence tripwire (replacing
  fabricated signals), rewarding absolute measured improvement gated on the MDE.
- **Error enhancement** — `ErrorEnhancer` is wired into tool error responses with
  contextual recovery suggestions, example calls, real complexity metrics, and
  memory-tool-specific guidance.
- **Tool-chain tracking** — transition matrix (`reasoning_metrics query="chains"`)
  feeds `reasoning_meta` routing and data-driven next-tool suggestions; metadata
  enrichment and timing estimates extended across the tool set; automated
  anti-pattern detection routes low-success transitions into the SI monitor.

### Changed

- Documentation refreshed to the real surface: **35 tools**, ~64,000 lines of
  production Rust, **2,895+ tests** (95%+ coverage); the shipped server
  instructions, `lib.rs` rustdoc, README, CLAUDE.md, and ARCHITECTURE now agree.
- MSRV corrected and pinned to **1.94** (dependency-driven by sqlx 0.9), with an
  MSRV CI job and clippy `incompatible_msrv` to keep it honest.
- Performance: ~45% fewer allocations; lighter transition counts; LRU eviction on
  the in-memory session map.
- Tool list no longer ships oversized output schemas (−45% payload).

### Fixed

- Validation checks across detect/counterfactual/timeline no longer "cry wolf";
  causal-name matching is normalized; MCTS tuning params are wired into the prompt
  and backtrack/backprop coherence is verified.
- Thinking-budget labels and metadata report each mode's real budget.
- Streaming flushes its final 100% Complete milestone (previously dropped).

### Housekeeping

- Removed a stray root demo script and a stale committed package artifact;
  applied `clippy --fix` across test code; dropped a stale `dead_code` allow.

## [0.2.0] - 2026-06-03

### Added

- **Semantic session memory (Voyage AI)** — `reasoning_search` and `reasoning_relate`
  rank past sessions by meaning, not keywords, using `voyage-4` embeddings with
  cosine recall and a `rerank-2.5` cross-encoder. Replaces the BM25/keyword path.
  - Embedding cache (`session_embeddings`), keyed on content **and** model, treated
    as derived data that self-heals on a miss.
  - Background embedding worker drains an `embedding_queue` on an interval so the
    first search/relate after a write is already warm.
  - **Requires `VOYAGE_API_KEY`** — no keyword fallback; the tools return a clear
    config error when it is unset.
- **Embedding-grounded novelty for `reasoning_divergent`** — each perspective's
  `novelty_score` is computed from embedding distance to the others instead of being
  self-reported. `reasoning_divergent` now also requires `VOYAGE_API_KEY`.
- Environment: `VOYAGE_API_KEY` (gates search/relate/divergent) and `VOYAGE_MODEL`
  (default `voyage-4`).

### Changed

- **Relate graph hardened** — single-session traversal is depth-bounded and
  edge-capped (`MAX_GRAPH_EDGES`) with no dangling edges; SharedMode and
  TemporallyAdjacent edges now carry real strengths (mode-set Jaccard / temporal
  decay), so `min_strength` actually filters them.
- **voyage-4 similarity thresholds tuned to measured distributions** — search
  `min_similarity` 0.5 → 0.35, relate `min_strength` 0.5 → 0.6, cluster threshold
  0.8 → 0.72.
- `reasoning_decision` responses omit perspectives-only fields
  (`stakeholder_map`/`conflicts`/`alignments`) on non-perspectives operations.

### Added (earlier, 2026-03-01)

- **Performance Optimizations** (2026-03-01)
  - Added 38 SQL query constants across 7 storage modules
  - Implemented Vec pre-allocation in 5 mode modules
  - Added HashMap pre-allocation in metrics collection
  - Added enum `as_str()` methods to avoid `format!()` allocations
  - Result: ~45% reduction in string allocations, ~71% fewer vector reallocations

- **Documentation Reorganization** (2026-03-01)
  - Split 170KB DESIGN.md into 3 focused documents (Architecture, API, Implementation)
  - Created categorical directory structure (reference/, guides/, design/, operations/)
  - Added comprehensive documentation index (docs/README.md)
  - Created DOCUMENTATION_AUDIT.md with complete analysis

### Changed

- **Documentation Structure** (2026-03-01)
  - Reorganized 20 documentation files into 5 categories
  - Archived 10 completed implementation plans
  - Updated all internal documentation links
  - Improved navigation with quick links by task/role

### Fixed

- **Dependency Management** (2026-03-01)
  - Pinned getrandom to 0.4 to reduce duplication
  - Pinned hashbrown to 0.16 to reduce duplication
  - Documented remaining duplicate dependencies
  - Removed unused [patch.crates-io] section from Cargo.toml

## [0.1.0] - 2025-12-31

### Added

- **Core Reasoning Tools**
  - `reasoning_linear` - Sequential step-by-step reasoning
  - `reasoning_tree` - Branching exploration with multiple paths
  - `reasoning_divergent` - Multi-perspective creative reasoning
  - `reasoning_reflection` - Meta-cognitive reasoning analysis
  - `reasoning_checkpoint` - State management and backtracking
  - `reasoning_auto` - Automatic mode selection

- **Advanced Reasoning**
  - `reasoning_graph` - Graph-of-Thoughts with 8 operations
  - `reasoning_timeline` - Temporal reasoning with parallel paths
  - `reasoning_mcts` - Monte Carlo Tree Search exploration
  - `reasoning_counterfactual` - What-if causal analysis

- **Analysis Tools**
  - `reasoning_detect` - Cognitive bias and logical fallacy detection
  - `reasoning_decision` - Multi-criteria decision analysis
  - `reasoning_evidence` - Evidence assessment and Bayesian inference

- **Infrastructure**
  - `reasoning_preset` - Pre-configured workflow execution
  - `reasoning_metrics` - Usage statistics and observability

- **Metadata Enrichment System**
  - Timing predictions with confidence levels
  - Tool suggestions based on context
  - Workflow preset recommendations
  - Historical learning from execution data
  - Complexity analysis and tracking

- **Self-Improvement System**
  - 4-phase optimization loop (Monitor → Analyze → Execute → Learn)
  - Circuit breaker safety mechanism
  - Action allowlist and validation
  - Rollback capability
  - Automatic performance tuning

- **Storage & Persistence**
  - SQLite-based session persistence
  - Branch tracking for tree reasoning
  - Checkpoint system for state restoration
  - Metrics collection and historical data
  - Transaction support with rollback

- **Client Integration**
  - Claude Code integration via stdio transport
  - Claude Desktop integration via config
  - Environment-based configuration
  - Structured logging to stderr
  - Streaming API support

### Technical Details

- **Language**: Rust 1.75+
- **Testing**: 2,020 tests with 95%+ coverage
- **Architecture**: Direct Anthropic API integration
- **Transport**: MCP stdio protocol
- **Database**: SQLite with sqlx
- **Performance**: Zero unsafe code, no panics in production paths

### Documentation

- Complete API reference for all 15 tools
- Architecture and design documents
- Implementation guides and patterns
- Testing strategies and best practices
- Pre-commit hooks and code quality automation

---

## Version History

### Version Format

This project uses [Semantic Versioning](https://semver.org/):

- **MAJOR**: Incompatible API changes
- **MINOR**: Backwards-compatible functionality additions
- **PATCH**: Backwards-compatible bug fixes

### Release Process

1. Update CHANGELOG.md with changes
2. Update version in Cargo.toml
3. Create git tag: `git tag -a v0.1.0 -m "Release v0.1.0"`
4. Push tag: `git push origin v0.1.0`
5. GitHub Actions will build and release

---

## Links

- [Repository](https://github.com/quanticsoul4772/mcp-reasoning)
- [Documentation](docs/README.md)
- [Issues](https://github.com/quanticsoul4772/mcp-reasoning/issues)
- [Pull Requests](https://github.com/quanticsoul4772/mcp-reasoning/pulls)
