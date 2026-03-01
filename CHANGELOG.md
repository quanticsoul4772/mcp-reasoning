# Changelog

All notable changes to the MCP Reasoning Server will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
