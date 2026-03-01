# @mcp-reasoning/server

MCP server providing 15 structured reasoning tools for Claude Code and Claude Desktop.

This is an npm wrapper that downloads the pre-built binary for your platform.

## Installation

```bash
# Global install
npm install -g @mcp-reasoning/server

# Or use with npx (no install needed)
npx @mcp-reasoning/server --version
```

## Usage

### Command Line

```bash
# Show version
mcp-reasoning --version

# Run health checks
mcp-reasoning --health

# Start server (requires ANTHROPIC_API_KEY)
export ANTHROPIC_API_KEY=your-key-here
mcp-reasoning
```

### With Claude Desktop

Add to `claude_desktop_config.json`:

**macOS/Linux**: `~/.config/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "npx",
      "args": ["-y", "@mcp-reasoning/server"],
      "env": {
        "ANTHROPIC_API_KEY": "your-api-key-here"
      }
    }
  }
}
```

Or if installed globally:

```json
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "mcp-reasoning",
      "env": {
        "ANTHROPIC_API_KEY": "your-api-key-here"
      }
    }
  }
}
```

## Features

- **15 Reasoning Tools**: Linear, tree, divergent, graph, MCTS, counterfactual, and more
- **Session Persistence**: Save and restore reasoning state
- **Self-Improvement**: Automated optimization with circuit breaker safety
- **Streaming API**: Real-time progress notifications
- **High Performance**: Written in Rust, 95%+ test coverage

## Documentation

- [GitHub Repository](https://github.com/quanticsoul4772/mcp-reasoning)
- [API Documentation](https://github.com/quanticsoul4772/mcp-reasoning/blob/main/docs/reference/API_SPECIFICATION.md)
- [Installation Guide](https://github.com/quanticsoul4772/mcp-reasoning/blob/main/docs/guides/DEVELOPMENT.md)

## Supported Platforms

- macOS (Intel and Apple Silicon)
- Linux (x86_64)
- Windows (x86_64)

## License

MIT
