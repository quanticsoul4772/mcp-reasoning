#!/bin/bash
# configure.sh - Interactive configuration wizard for Claude Desktop
# Usage: ./configure.sh

set -e

echo "🧙 MCP Reasoning Server Configuration Wizard"
echo "============================================="
echo ""

# Detect OS and config path
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
  darwin)
    CONFIG_DIR="$HOME/Library/Application Support/Claude"
    ;;
  linux)
    CONFIG_DIR="$HOME/.config/Claude"
    ;;
  *)
    echo "❌ Unsupported OS: $OS"
    exit 1
    ;;
esac

CONFIG_FILE="$CONFIG_DIR/claude_desktop_config.json"

echo "📁 Config location: $CONFIG_FILE"
echo ""

# Check if mcp-reasoning is installed
BINARY_PATH=""
if command -v mcp-reasoning &> /dev/null; then
    BINARY_PATH=$(command -v mcp-reasoning)
    echo "✅ Found mcp-reasoning at: $BINARY_PATH"
else
    echo "⚠️  mcp-reasoning not found in PATH"
    echo ""
    read -p "Enter full path to mcp-reasoning binary: " BINARY_PATH

    if [ ! -x "$BINARY_PATH" ]; then
        echo "❌ Binary not found or not executable: $BINARY_PATH"
        echo ""
        echo "Install mcp-reasoning first:"
        echo "  curl -fsSL https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.sh | bash"
        exit 1
    fi
fi

echo ""

# Get API key
API_KEY=""
if [ -n "$ANTHROPIC_API_KEY" ]; then
    echo "✅ Found ANTHROPIC_API_KEY in environment"
    read -p "Use environment variable? (y/n): " USE_ENV
    if [[ "$USE_ENV" =~ ^[Yy]$ ]]; then
        API_KEY="$ANTHROPIC_API_KEY"
    fi
fi

if [ -z "$API_KEY" ]; then
    echo ""
    echo "Get your API key from: https://console.anthropic.com/"
    read -p "🔑 Enter Anthropic API key: " -s API_KEY
    echo ""

    if [ -z "$API_KEY" ]; then
        echo "❌ No API key provided"
        exit 1
    fi
fi

echo ""

# Optional settings
echo "Optional settings (press Enter for defaults):"
echo ""

read -p "📊 Database path [./data/reasoning.db]: " DB_PATH
DB_PATH=${DB_PATH:-./data/reasoning.db}

read -p "📋 Log level [info/debug/trace]: " LOG_LEVEL
LOG_LEVEL=${LOG_LEVEL:-info}

echo ""
echo "Summary:"
echo "--------"
echo "Binary:    $BINARY_PATH"
echo "API Key:   ${API_KEY:0:10}... (hidden)"
echo "Database:  $DB_PATH"
echo "Log Level: $LOG_LEVEL"
echo ""

read -p "Proceed with configuration? (y/n): " -n 1 -r
echo

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Configuration cancelled"
    exit 1
fi

# Create config directory
mkdir -p "$CONFIG_DIR"

# Backup existing config
if [ -f "$CONFIG_FILE" ]; then
    BACKUP="$CONFIG_FILE.backup.$(date +%Y%m%d_%H%M%S)"
    echo "📦 Backing up existing config to: $BACKUP"
    cp "$CONFIG_FILE" "$BACKUP"
fi

# Build config JSON
if command -v jq &> /dev/null; then
    # Use jq for proper JSON merging
    EXISTING_CONFIG=$(cat "$CONFIG_FILE" 2>/dev/null || echo '{}')

    echo "$EXISTING_CONFIG" | jq \
        --arg path "$BINARY_PATH" \
        --arg key "$API_KEY" \
        --arg db "$DB_PATH" \
        --arg log "$LOG_LEVEL" \
        '.mcpServers["mcp-reasoning"] = {
            command: $path,
            env: {
                ANTHROPIC_API_KEY: $key,
                DATABASE_PATH: $db,
                LOG_LEVEL: $log
            }
        }' > "$CONFIG_FILE"
else
    # Fallback: simple JSON (overwrites file)
    echo "⚠️  jq not found - creating new config (will overwrite existing)"
    cat > "$CONFIG_FILE" <<EOF
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "$BINARY_PATH",
      "env": {
        "ANTHROPIC_API_KEY": "$API_KEY",
        "DATABASE_PATH": "$DB_PATH",
        "LOG_LEVEL": "$LOG_LEVEL"
      }
    }
  }
}
EOF
fi

echo ""
echo "✅ Configuration complete!"
echo ""
echo "Next steps:"
echo "1. Restart Claude Desktop"
echo "2. Ask Claude to use reasoning tools:"
echo "   \"Use linear reasoning to analyze the trade-offs\""
echo ""
echo "Troubleshooting:"
if [ "$OS" = "darwin" ]; then
    echo "  View logs: ~/Library/Logs/Claude/mcp-server-mcp-reasoning.log"
else
    echo "  View logs: ~/.local/share/Claude/logs/mcp-server-mcp-reasoning.log"
fi
echo "  Test binary: $BINARY_PATH --health"
echo ""
echo "Documentation: https://github.com/quanticsoul4772/mcp-reasoning"
