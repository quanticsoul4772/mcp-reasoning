#!/bin/bash
# install.sh - One-command installation for macOS/Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.sh | bash

set -e

REPO="quanticsoul4772/mcp-reasoning"
VERSION="${1:-latest}"

echo "🔧 MCP Reasoning Server Installer"
echo "=================================="
echo ""

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)
    if [ "$ARCH" = "x86_64" ]; then
      TARGET="x86_64-unknown-linux-gnu"
    else
      echo "❌ Unsupported architecture: $ARCH"
      echo "Supported: x86_64"
      exit 1
    fi
    ;;
  darwin)
    if [ "$ARCH" = "x86_64" ]; then
      TARGET="x86_64-apple-darwin"
    elif [ "$ARCH" = "arm64" ]; then
      TARGET="aarch64-apple-darwin"
    else
      echo "❌ Unsupported architecture: $ARCH"
      echo "Supported: x86_64, arm64"
      exit 1
    fi
    ;;
  *)
    echo "❌ Unsupported OS: $OS"
    echo "Supported: Linux, macOS"
    echo "For Windows, use: install.ps1"
    exit 1
    ;;
esac

echo "📦 Detected: $OS ($ARCH) → $TARGET"
echo ""

# Download latest release
echo "⬇️  Downloading binary..."
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/$REPO/releases/latest/download/${TARGET}.tar.gz"
else
  URL="https://github.com/$REPO/releases/download/$VERSION/${TARGET}.tar.gz"
fi

TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

if command -v curl &> /dev/null; then
  curl -fsSL "$URL" -o release.tar.gz
elif command -v wget &> /dev/null; then
  wget -q "$URL" -O release.tar.gz
else
  echo "❌ Neither curl nor wget found. Please install one of them."
  exit 1
fi

tar -xzf release.tar.gz
chmod +x mcp-reasoning
mv mcp-reasoning "$INSTALL_DIR/"

cd - > /dev/null
rm -rf "$TEMP_DIR"

echo "✅ Binary installed to: $INSTALL_DIR/mcp-reasoning"
echo ""

# Add to PATH if needed
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
  echo "⚠️  $INSTALL_DIR is not in your PATH"
  echo ""
  echo "Add to PATH by adding this line to your shell profile:"

  if [ -f "$HOME/.zshrc" ]; then
    echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
    echo "  source ~/.zshrc"
  elif [ -f "$HOME/.bashrc" ]; then
    echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
    echo "  source ~/.bashrc"
  else
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
  fi
  echo ""
fi

# Verify installation
echo "Verifying installation..."
if "$INSTALL_DIR/mcp-reasoning" --version > /dev/null 2>&1; then
  echo "✅ Installation verified"
else
  echo "⚠️  Installation may have issues. Try running:"
  echo "   $INSTALL_DIR/mcp-reasoning --version"
fi

echo ""
echo "✨ Installation complete!"
echo ""
echo "Next steps:"
echo "1. Get an Anthropic API key: https://console.anthropic.com/"
echo "2. Configure Claude Desktop:"
echo "   Run: $INSTALL_DIR/mcp-reasoning --help"
echo ""
echo "Quick test:"
echo "  export ANTHROPIC_API_KEY=your-key-here"
echo "  $INSTALL_DIR/mcp-reasoning --health"
echo ""
echo "Documentation: https://github.com/$REPO"
