# Documentation Index

Welcome to the MCP Reasoning Server documentation!

## 🚀 Getting Started

- [Main README](../README.md) - Project overview and quick start
- [Development Guide](guides/DEVELOPMENT.md) - Setup and development workflow
- [Contributing Guide](guides/CONTRIBUTING.md) - How to contribute

## 📖 Reference Documentation

### Architecture & Design

- [Architecture](reference/ARCHITECTURE.md) - System design and tool overview
- [API Specification](reference/API_SPECIFICATION.md) - Complete tool schemas (15 tools)
- [Implementation Details](reference/IMPLEMENTATION_DETAILS.md) - Technical implementation
- [Tool Reference](reference/TOOL_REFERENCE.md) - API usage guide with examples

### Patterns & Best Practices

- [Lessons Learned](reference/LESSONS_LEARNED.md) - Architectural decisions and patterns
- [Mode Pattern](reference/MODE_PATTERN.md) - Template for implementing new modes

## 📝 Development Guides

- [Development](guides/DEVELOPMENT.md) - Environment setup, build commands, debugging
- [Testing](guides/TESTING.md) - Testing strategies and best practices
- [Pre-commit Setup](guides/PRE_COMMIT_SETUP.md) - Git hooks and code quality
- [Contributing](guides/CONTRIBUTING.md) - Contribution workflow and guidelines

## 🎨 Design Documents

### Core Systems

- [Self-Improvement System](design/SELF_IMPROVEMENT.md) - 4-phase optimization loop
- [Metadata Enrichment](design/METADATA_ENRICHMENT.md) - Timing predictions & tool suggestions
- [Tiered Timeouts](design/TIERED_TIMEOUTS.md) - Timeout management system

### Future Enhancements

- [Silent Failures Fix Plan](design/SILENT_FAILURES_FIX_PLAN.md) - Planned improvements
- [Tool Composition & Error Guidance Plan](design/TOOL_COMPOSITION_AND_ERROR_GUIDANCE_PLAN.md) - Future features

## ⚙️ Operations

- [Branch Protection](operations/BRANCH_PROTECTION.md) - Git workflow and branch rules
- [Cleanup Notes](operations/CLEANUP_NOTES.md) - Maintenance history

## 📚 Archive

Historical implementation plans and completed design documents are in [archive/](archive/).

---

## Quick Links by Task

### I want to

**...understand the system:**
→ Start with [Architecture](reference/ARCHITECTURE.md)

**...use the tools:**
→ See [Tool Reference](reference/TOOL_REFERENCE.md) for examples

**...contribute code:**
→ Read [Contributing Guide](guides/CONTRIBUTING.md) first

**...set up development:**
→ Follow [Development Guide](guides/DEVELOPMENT.md)

**...implement a new mode:**
→ Use [Mode Pattern](reference/MODE_PATTERN.md) template

**...understand design decisions:**
→ Check [Lessons Learned](reference/LESSONS_LEARNED.md)

---

## Documentation Structure

```
docs/
├── README.md (you are here)
├── reference/          # Architecture, API specs, patterns
├── guides/             # How-to guides for development
├── design/             # Design documents and plans
├── operations/         # Operations and maintenance
└── archive/            # Historical documents
```

---

## Contributing to Documentation

Documentation improvements are welcome! Please:

1. Keep files under 500 lines when possible
2. Use clear headings and structure
3. Include code examples where helpful
4. Update this index when adding new docs
5. Follow the [Contributing Guide](guides/CONTRIBUTING.md)

---

**Last Updated:** 2026-03-01
**Documentation Version:** 2.0 (Reorganized)
