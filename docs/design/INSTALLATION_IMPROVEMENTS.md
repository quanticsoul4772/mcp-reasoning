# Installation Process Analysis & Improvements

**Date**: 2026-03-01
**Status**: Proposal
**Author**: Droid

---

## Executive Summary

Current installation requires Rust toolchain, manual builds, and manual configuration. This document proposes sophisticated improvements to support one-command installation across macOS, Windows, and Linux with automatic configuration.

**Key Improvements**:

- Pre-built binary releases (4 platforms)
- Platform-specific installers (shell scripts, PowerShell, MSI)
- Package manager support (Homebrew, Chocolatey, snap)
- npm wrapper for easy distribution
- Docker image for containerized deployment
- Automatic Claude configuration
- Interactive setup wizard

---

## Current State Analysis

### Current Installation Process

```bash
# Step 1: Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Step 2: Clone repository
git clone https://github.com/quanticsoul4772/mcp-reasoning.git
cd mcp-reasoning

# Step 3: Build from source
cargo build --release

# Step 4: Manual configuration
# Edit ~/.config/Claude/claude_desktop_config.json
# Add binary path and API key
```

### Pain Points

| Issue | Impact | Severity |
|-------|--------|----------|
| **Requires Rust toolchain** | ~500MB download, 5-10 min setup | High |
| **Long build times** | 5-8 minutes for release build | High |
| **Manual path configuration** | Error-prone, requires absolute paths | Medium |
| **Manual API key setup** | Security risk (stored in JSON) | Medium |
| **No verification step** | Users don't know if it works | Medium |
| **Platform differences** | Different config paths per OS | Low |
| **No updates mechanism** | Manual git pull + rebuild | Medium |

### Current Strengths

| Strength | Benefit |
|----------|---------|
| **GitHub release workflow exists** | Already builds 4 platform binaries |
| **Clean codebase** | 2,020 tests, 95% coverage |
| **Well-documented** | Clear prerequisites and steps |
| **MCP standard** | Works with standard MCP clients |

---

## Proposed Improvements

### Phase 1: Pre-built Binaries & Basic Installers

**Goal**: Eliminate Rust requirement for 95% of users

#### 1.1 GitHub Releases (Already Configured ✓)

The existing `release.yml` workflow already builds:

- Linux: `x86_64-unknown-linux-gnu` (Ubuntu/Debian/Fedora/Arch)
- macOS Intel: `x86_64-apple-darwin`
- macOS ARM: `aarch64-apple-darwin` (M1/M2/M3)
- Windows: `x86_64-pc-windows-msvc`

**Action Required**:

- Create first release tag: `v0.1.0`
- Verify binary artifacts are published

#### 1.2 Installation Scripts

**macOS/Linux: `install.sh`**

```bash
#!/bin/bash
# install.sh - One-command installation for macOS/Linux

set -e

REPO="quanticsoul4772/mcp-reasoning"
VERSION="${1:-latest}"

echo "🔧 MCP Reasoning Server Installer"
echo "=================================="

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)
    if [ "$ARCH" = "x86_64" ]; then
      TARGET="x86_64-unknown-linux-gnu"
    else
      echo "❌ Unsupported architecture: $ARCH"
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
      exit 1
    fi
    ;;
  *)
    echo "❌ Unsupported OS: $OS"
    exit 1
    ;;
esac

echo "📦 Detected: $OS ($ARCH) → $TARGET"

# Download latest release
echo "⬇️  Downloading binary..."
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/$REPO/releases/latest/download/${TARGET}.tar.gz"
else
  URL="https://github.com/$REPO/releases/download/$VERSION/${TARGET}.tar.gz"
fi

curl -fsSL "$URL" | tar -xz -C "$INSTALL_DIR"
chmod +x "$INSTALL_DIR/mcp-reasoning"

echo "✅ Binary installed to: $INSTALL_DIR/mcp-reasoning"

# Add to PATH if needed
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
  echo ""
  echo "⚠️  Add to PATH by running:"
  echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
  echo "   (Add to ~/.bashrc or ~/.zshrc for persistence)"
fi

# Interactive configuration
echo ""
read -p "🔑 Configure Claude Desktop now? (y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  read -p "Enter Anthropic API key: " -s API_KEY
  echo

  # Detect Claude config path
  if [ "$OS" = "darwin" ]; then
    CONFIG_DIR="$HOME/Library/Application Support/Claude"
  else
    CONFIG_DIR="$HOME/.config/Claude"
  fi

  CONFIG_FILE="$CONFIG_DIR/claude_desktop_config.json"
  mkdir -p "$CONFIG_DIR"

  # Create or update config
  if [ -f "$CONFIG_FILE" ]; then
    echo "⚠️  Existing config found - backing up to ${CONFIG_FILE}.bak"
    cp "$CONFIG_FILE" "${CONFIG_FILE}.bak"
  fi

  # Use jq if available, otherwise create simple JSON
  if command -v jq &> /dev/null; then
    jq --arg path "$INSTALL_DIR/mcp-reasoning" \
       --arg key "$API_KEY" \
       '.mcpServers["mcp-reasoning"] = {
         command: $path,
         env: { ANTHROPIC_API_KEY: $key }
       }' "${CONFIG_FILE:-{}}" > "$CONFIG_FILE.tmp"
    mv "$CONFIG_FILE.tmp" "$CONFIG_FILE"
  else
    cat > "$CONFIG_FILE" <<EOF
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "$INSTALL_DIR/mcp-reasoning",
      "env": {
        "ANTHROPIC_API_KEY": "$API_KEY"
      }
    }
  }
}
EOF
  fi

  echo "✅ Claude Desktop configured at: $CONFIG_FILE"
  echo "🔄 Restart Claude Desktop to activate"
fi

echo ""
echo "✨ Installation complete!"
echo ""
echo "Verify installation:"
echo "  $INSTALL_DIR/mcp-reasoning --version"
echo ""
echo "Documentation: https://github.com/$REPO"
```

**Windows: `install.ps1`**

```powershell
# install.ps1 - One-command installation for Windows

param(
    [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"

$REPO = "quanticsoul4772/mcp-reasoning"
$TARGET = "x86_64-pc-windows-msvc"

Write-Host "🔧 MCP Reasoning Server Installer" -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan

# Download binary
Write-Host "⬇️  Downloading binary..." -ForegroundColor Yellow

$INSTALL_DIR = "$env:LOCALAPPDATA\Programs\mcp-reasoning"
New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null

if ($Version -eq "latest") {
    $URL = "https://github.com/$REPO/releases/latest/download/${TARGET}.zip"
} else {
    $URL = "https://github.com/$REPO/releases/download/$Version/${TARGET}.zip"
}

$ZIP_PATH = "$env:TEMP\mcp-reasoning.zip"
Invoke-WebRequest -Uri $URL -OutFile $ZIP_PATH
Expand-Archive -Path $ZIP_PATH -DestinationPath $INSTALL_DIR -Force
Remove-Item $ZIP_PATH

Write-Host "✅ Binary installed to: $INSTALL_DIR\mcp-reasoning.exe" -ForegroundColor Green

# Add to PATH
$USER_PATH = [Environment]::GetEnvironmentVariable("Path", "User")
if ($USER_PATH -notlike "*$INSTALL_DIR*") {
    Write-Host "➕ Adding to PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable(
        "Path",
        "$USER_PATH;$INSTALL_DIR",
        "User"
    )
    Write-Host "✅ Added to PATH (restart terminal to use)" -ForegroundColor Green
}

# Interactive configuration
Write-Host ""
$configure = Read-Host "🔑 Configure Claude Desktop now? (y/n)"
if ($configure -eq "y" -or $configure -eq "Y") {
    $API_KEY = Read-Host "Enter Anthropic API key" -AsSecureString
    $API_KEY_PLAIN = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
        [Runtime.InteropServices.Marshal]::SecureStringToBSTR($API_KEY)
    )

    $CONFIG_DIR = "$env:APPDATA\Claude"
    $CONFIG_FILE = "$CONFIG_DIR\claude_desktop_config.json"
    New-Item -ItemType Directory -Force -Path $CONFIG_DIR | Out-Null

    # Backup existing config
    if (Test-Path $CONFIG_FILE) {
        Write-Host "⚠️  Backing up existing config..." -ForegroundColor Yellow
        Copy-Item $CONFIG_FILE "$CONFIG_FILE.bak"
    }

    # Create config
    $CONFIG = @{
        mcpServers = @{
            "mcp-reasoning" = @{
                command = "$INSTALL_DIR\mcp-reasoning.exe"
                env = @{
                    ANTHROPIC_API_KEY = $API_KEY_PLAIN
                }
            }
        }
    } | ConvertTo-Json -Depth 10

    Set-Content -Path $CONFIG_FILE -Value $CONFIG
    Write-Host "✅ Claude Desktop configured at: $CONFIG_FILE" -ForegroundColor Green
    Write-Host "🔄 Restart Claude Desktop to activate" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "✨ Installation complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Verify installation:"
Write-Host "  mcp-reasoning --version"
Write-Host ""
Write-Host "Documentation: https://github.com/$REPO"
```

**Usage**:

```bash
# macOS/Linux
curl -fsSL https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.sh | bash

# Windows (PowerShell as Administrator)
irm https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.ps1 | iex
```

---

### Phase 2: Package Managers

**Goal**: Native installation via platform package managers

#### 2.1 Homebrew (macOS/Linux)

**Create `mcp-reasoning.rb` formula**:

```ruby
class McpReasoning < Formula
  desc "MCP server providing structured reasoning capabilities"
  homepage "https://github.com/quanticsoul4772/mcp-reasoning"
  version "0.1.0"

  if OS.mac? && Hardware::CPU.intel?
    url "https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-apple-darwin.tar.gz"
    sha256 "..." # Calculate after first release
  elsif OS.mac? && Hardware::CPU.arm?
    url "https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/aarch64-apple-darwin.tar.gz"
    sha256 "..."
  elsif OS.linux?
    url "https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-unknown-linux-gnu.tar.gz"
    sha256 "..."
  end

  def install
    bin.install "mcp-reasoning"
  end

  def caveats
    <<~EOS
      To configure Claude Desktop:
        1. Get an Anthropic API key from https://console.anthropic.com/
        2. Edit #{ENV["HOME"]}/.config/Claude/claude_desktop_config.json
        3. Add the following:

        {
          "mcpServers": {
            "mcp-reasoning": {
              "command": "#{bin}/mcp-reasoning",
              "env": {
                "ANTHROPIC_API_KEY": "your-api-key-here"
              }
            }
          }
        }

        4. Restart Claude Desktop
    EOS
  end

  test do
    assert_match "mcp-reasoning", shell_output("#{bin}/mcp-reasoning --version")
  end
end
```

**Distribution Options**:

1. **Homebrew Tap** (Recommended for early releases):

   ```bash
   # Create tap repository
   brew tap quanticsoul4772/mcp
   brew install mcp-reasoning
   ```

2. **Homebrew Core** (After maturity):
   - Submit PR to homebrew-core
   - Requires 75+ GitHub stars, 30+ forks
   - Automated updates via `brew bump-formula-pr`

#### 2.2 Chocolatey (Windows)

**Create `mcp-reasoning.nuspec`**:

```xml
<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2015/06/nuspec.xsd">
  <metadata>
    <id>mcp-reasoning</id>
    <version>0.1.0</version>
    <packageSourceUrl>https://github.com/quanticsoul4772/mcp-reasoning</packageSourceUrl>
    <owners>quanticsoul4772</owners>
    <title>MCP Reasoning Server</title>
    <authors>quanticsoul4772</authors>
    <projectUrl>https://github.com/quanticsoul4772/mcp-reasoning</projectUrl>
    <licenseUrl>https://github.com/quanticsoul4772/mcp-reasoning/blob/main/LICENSE</licenseUrl>
    <requireLicenseAcceptance>false</requireLicenseAcceptance>
    <tags>mcp claude reasoning ai anthropic</tags>
    <summary>MCP server providing 15 structured reasoning tools for Claude</summary>
    <description>
A high-performance MCP server that provides structured reasoning capabilities
for Claude Code and Claude Desktop. Built in Rust with 15 reasoning modes
including linear, tree-based, graph-based, and advanced causal analysis.
    </description>
  </metadata>
  <files>
    <file src="tools\**" target="tools" />
  </files>
</package>
```

**Install script** (`tools/chocolateyinstall.ps1`):

```powershell
$ErrorActionPreference = 'Stop'

$packageName = 'mcp-reasoning'
$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url64 = 'https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-pc-windows-msvc.zip'

$packageArgs = @{
  packageName   = $packageName
  unzipLocation = $toolsDir
  url64bit      = $url64
  checksum64    = '...'
  checksumType64= 'sha256'
}

Install-ChocolateyZipPackage @packageArgs
```

**Usage**:

```powershell
choco install mcp-reasoning
```

#### 2.3 Snap (Linux)

**Create `snapcraft.yaml`**:

```yaml
name: mcp-reasoning
base: core22
version: '0.1.0'
summary: MCP server providing structured reasoning capabilities
description: |
  A high-performance MCP server that provides 15 structured reasoning tools
  for Claude Code and Claude Desktop. Built in Rust for speed and reliability.

grade: stable
confinement: strict

apps:
  mcp-reasoning:
    command: bin/mcp-reasoning
    plugs:
      - network
      - home

parts:
  mcp-reasoning:
    plugin: dump
    source: https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-unknown-linux-gnu.tar.gz
    source-type: tar
    organize:
      mcp-reasoning: bin/mcp-reasoning
```

**Usage**:

```bash
sudo snap install mcp-reasoning
```

#### 2.4 APT Repository (Debian/Ubuntu)

**Create `.deb` package using `cargo-deb`**:

```bash
cargo install cargo-deb
cargo deb
```

**Add to `Cargo.toml`**:

```toml
[package.metadata.deb]
maintainer = "quanticsoul4772"
copyright = "2026, quanticsoul4772 <email@example.com>"
license-file = ["LICENSE", "0"]
extended-description = """\
A high-performance MCP server providing 15 structured reasoning tools \
for Claude Code and Claude Desktop."""
depends = "$auto"
section = "utility"
priority = "optional"
assets = [
    ["target/release/mcp-reasoning", "usr/bin/", "755"],
    ["README.md", "usr/share/doc/mcp-reasoning/", "644"],
]
```

---

### Phase 3: npm Wrapper (Cross-Platform)

**Goal**: Easiest installation for JavaScript/TypeScript developers

**Why npm?**:

- Most developers already have Node.js/npm installed
- Cross-platform binary distribution built-in
- Automatic platform detection
- Handles PATH configuration
- Easy updates: `npm update -g mcp-reasoning`

**Package structure**:

```
mcp-reasoning-npm/
├── package.json
├── index.js
├── install.js
└── bin/
    └── mcp-reasoning (symlink created post-install)
```

**`package.json`**:

```json
{
  "name": "@mcp-reasoning/server",
  "version": "0.1.0",
  "description": "MCP server providing structured reasoning capabilities",
  "bin": {
    "mcp-reasoning": "bin/mcp-reasoning"
  },
  "scripts": {
    "postinstall": "node install.js"
  },
  "repository": {
    "type": "git",
    "url": "https://github.com/quanticsoul4772/mcp-reasoning.git"
  },
  "keywords": ["mcp", "claude", "reasoning", "ai", "anthropic"],
  "author": "quanticsoul4772",
  "license": "MIT",
  "os": ["darwin", "linux", "win32"],
  "cpu": ["x64", "arm64"]
}
```

**`install.js`**:

```javascript
#!/usr/bin/env node

const { execSync } = require('child_process');
const https = require('https');
const fs = require('fs');
const path = require('path');
const zlib = require('zlib');
const tar = require('tar');

const VERSION = '0.1.0';
const REPO = 'quanticsoul4772/mcp-reasoning';

// Platform detection
const platform = process.platform;
const arch = process.arch;

const TARGETS = {
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'win32-x64': 'x86_64-pc-windows-msvc',
};

const key = `${platform}-${arch}`;
const target = TARGETS[key];

if (!target) {
  console.error(`Unsupported platform: ${platform}-${arch}`);
  process.exit(1);
}

const isWindows = platform === 'win32';
const ext = isWindows ? 'zip' : 'tar.gz';
const binaryName = isWindows ? 'mcp-reasoning.exe' : 'mcp-reasoning';

const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${target}.${ext}`;
const binDir = path.join(__dirname, 'bin');
const binPath = path.join(binDir, binaryName);

console.log(`📦 Installing mcp-reasoning v${VERSION} for ${platform}-${arch}...`);
console.log(`⬇️  Downloading from: ${url}`);

// Create bin directory
if (!fs.existsSync(binDir)) {
  fs.mkdirSync(binDir, { recursive: true });
}

// Download and extract
const file = fs.createWriteStream(binPath);

https.get(url, (response) => {
  if (response.statusCode === 302 || response.statusCode === 301) {
    // Follow redirect
    https.get(response.headers.location, (redirectResponse) => {
      extractAndInstall(redirectResponse);
    });
  } else {
    extractAndInstall(response);
  }
});

function extractAndInstall(response) {
  if (isWindows) {
    // Handle ZIP for Windows
    const AdmZip = require('adm-zip');
    const chunks = [];

    response.on('data', (chunk) => chunks.push(chunk));
    response.on('end', () => {
      const buffer = Buffer.concat(chunks);
      const zip = new AdmZip(buffer);
      zip.extractAllTo(binDir, true);

      console.log('✅ Installation complete!');
      console.log(`Binary installed at: ${binPath}`);
      console.log('\nRun: mcp-reasoning --version');
    });
  } else {
    // Handle tar.gz for Unix
    response
      .pipe(zlib.createGunzip())
      .pipe(tar.extract({ cwd: binDir }))
      .on('finish', () => {
        fs.chmodSync(binPath, 0o755);
        console.log('✅ Installation complete!');
        console.log(`Binary installed at: ${binPath}`);
        console.log('\nRun: mcp-reasoning --version');
      });
  }
}
```

**Usage**:

```bash
# Global install
npm install -g @mcp-reasoning/server

# Or run directly (npx downloads and caches)
npx @mcp-reasoning/server --version

# Use in Claude Desktop config
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "npx",
      "args": ["@mcp-reasoning/server"],
      "env": {
        "ANTHROPIC_API_KEY": "sk-ant-xxx"
      }
    }
  }
}
```

**Benefits**:

- ✅ Zero Rust installation required
- ✅ Works on all platforms
- ✅ Auto-updates with `npm update`
- ✅ Familiar to JavaScript developers
- ✅ Can be used with `npx` (no install needed)

---

### Phase 4: Docker Image

**Goal**: Containerized deployment for servers and cloud environments

**`Dockerfile`**:

```dockerfile
# Multi-stage build for minimal image size
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev sqlite-dev

WORKDIR /build

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY migrations migrations

# Build release binary
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime image
FROM alpine:latest

RUN apk add --no-cache sqlite-libs ca-certificates

# Create non-root user
RUN addgroup -g 1000 mcp && \
    adduser -D -u 1000 -G mcp mcp

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/mcp-reasoning /usr/local/bin/

# Create data directory
RUN mkdir -p /app/data && chown -R mcp:mcp /app

USER mcp

EXPOSE 8080

ENV DATABASE_PATH=/app/data/reasoning.db
ENV MCP_TRANSPORT=stdio
ENV LOG_LEVEL=info

ENTRYPOINT ["/usr/local/bin/mcp-reasoning"]
```

**`docker-compose.yml`** (for easy setup):

```yaml
version: '3.8'

services:
  mcp-reasoning:
    image: ghcr.io/quanticsoul4772/mcp-reasoning:latest
    container_name: mcp-reasoning
    restart: unless-stopped
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - DATABASE_PATH=/app/data/reasoning.db
      - LOG_LEVEL=info
    volumes:
      - ./data:/app/data
    stdin_open: true  # Keep stdin open for stdio transport
    tty: true
```

**GitHub Actions for Docker** (`.github/workflows/docker.yml`):

```yaml
name: Docker

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - uses: actions/checkout@v6

      - name: Log in to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=semver,pattern={{version}}
            type=semver,pattern={{major}}.{{minor}}
            type=semver,pattern={{major}}
            type=raw,value=latest,enable={{is_default_branch}}

      - name: Build and push
        uses: docker/build-push-action@v6
        with:
          context: .
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
```

**Usage**:

```bash
# Pull and run
docker run -it --rm \
  -e ANTHROPIC_API_KEY=sk-ant-xxx \
  ghcr.io/quanticsoul4772/mcp-reasoning:latest

# Or with docker-compose
docker-compose up -d
```

---

### Phase 5: Automated Configuration

**Goal**: Zero-touch Claude Desktop configuration

**`configure.sh`** (Interactive setup wizard):

```bash
#!/bin/bash
# configure.sh - Interactive configuration wizard

set -e

echo "🧙 MCP Reasoning Server Configuration Wizard"
echo "============================================="
echo ""

# Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
if [ "$OS" = "darwin" ]; then
    CONFIG_DIR="$HOME/Library/Application Support/Claude"
else
    CONFIG_DIR="$HOME/.config/Claude"
fi

CONFIG_FILE="$CONFIG_DIR/claude_desktop_config.json"

echo "📁 Config location: $CONFIG_FILE"
echo ""

# Get binary path
if command -v mcp-reasoning &> /dev/null; then
    BINARY_PATH=$(command -v mcp-reasoning)
    echo "✅ Found mcp-reasoning at: $BINARY_PATH"
else
    echo "⚠️  mcp-reasoning not found in PATH"
    read -p "Enter full path to mcp-reasoning binary: " BINARY_PATH

    if [ ! -x "$BINARY_PATH" ]; then
        echo "❌ Binary not found or not executable: $BINARY_PATH"
        exit 1
    fi
fi

echo ""

# Get API key
if [ -n "$ANTHROPIC_API_KEY" ]; then
    echo "✅ Found ANTHROPIC_API_KEY in environment"
    USE_ENV_KEY="y"
else
    read -p "🔑 Enter Anthropic API key (or press Enter to use env var): " API_KEY

    if [ -z "$API_KEY" ]; then
        echo "❌ No API key provided and ANTHROPIC_API_KEY not set"
        exit 1
    fi
fi

echo ""

# Optional settings
read -p "📊 Database path [default: ./data/reasoning.db]: " DB_PATH
DB_PATH=${DB_PATH:-./data/reasoning.db}

read -p "📋 Log level [info/debug/trace] (default: info): " LOG_LEVEL
LOG_LEVEL=${LOG_LEVEL:-info}

echo ""
echo "Summary:"
echo "--------"
echo "Binary:   $BINARY_PATH"
echo "API Key:  ${USE_ENV_KEY:+[from environment]}${API_KEY:+[provided]}"
echo "Database: $DB_PATH"
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
        --arg key "${API_KEY:-$ANTHROPIC_API_KEY}" \
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
    cat > "$CONFIG_FILE" <<EOF
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "$BINARY_PATH",
      "env": {
        "ANTHROPIC_API_KEY": "${API_KEY:-$ANTHROPIC_API_KEY}",
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
echo "   \"Use linear reasoning to analyze X\""
echo ""
echo "Troubleshooting:"
echo "  View logs: tail -f ~/.local/share/Claude/logs/mcp-server-mcp-reasoning.log"
echo "  Test binary: $BINARY_PATH --version"
```

---

### Phase 6: Verification & Health Checks

**Add `--version` and `--health` flags to binary**:

**`src/main.rs`** additions:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-v" => {
                println!("mcp-reasoning {}", env!("CARGO_PKG_VERSION"));
                println!("Rust MCP server for structured reasoning");
                return Ok(());
            }
            "--health" => {
                // Run health checks
                health_check().await?;
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown argument: {}", args[1]);
                print_help();
                std::process::exit(1);
            }
        }
    }

    // Normal server startup
    // ...
}

async fn health_check() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏥 Running health checks...");
    println!();

    // Check 1: Environment variables
    print!("1. API key configured... ");
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        println!("✅");
    } else {
        println!("❌ ANTHROPIC_API_KEY not set");
        return Err("Missing API key".into());
    }

    // Check 2: Database connectivity
    print!("2. Database connection... ");
    let config = Config::from_env()?;
    let storage = SqliteStorage::new(&config.database_path).await?;
    println!("✅");

    // Check 3: Anthropic API connectivity
    print!("3. Anthropic API reachable... ");
    let client = AnthropicClient::new(config.clone())?;
    // Simple ping test (minimal token usage)
    match client.generate_completion("test", "Say 'ok'", &[], None).await {
        Ok(_) => println!("✅"),
        Err(e) => {
            println!("❌ {}", e);
            return Err(e.into());
        }
    }

    println!();
    println!("✅ All health checks passed!");
    println!("Server is ready to use.");

    Ok(())
}

fn print_help() {
    println!("MCP Reasoning Server v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("    mcp-reasoning [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --version, -v    Print version information");
    println!("    --health         Run health checks");
    println!("    --help, -h       Print this help message");
    println!();
    println!("ENVIRONMENT:");
    println!("    ANTHROPIC_API_KEY    Anthropic API key (required)");
    println!("    DATABASE_PATH        SQLite database path (default: ./data/reasoning.db)");
    println!("    LOG_LEVEL            Log level: error|warn|info|debug|trace (default: info)");
    println!();
    println!("DOCUMENTATION:");
    println!("    https://github.com/quanticsoul4772/mcp-reasoning");
}
```

**Usage**:

```bash
# Verify installation
mcp-reasoning --version
# Output: mcp-reasoning 0.1.0

# Run health checks
mcp-reasoning --health
# Output:
# 🏥 Running health checks...
# 1. API key configured... ✅
# 2. Database connection... ✅
# 3. Anthropic API reachable... ✅
# ✅ All health checks passed!
```

---

## Implementation Roadmap

### Week 1: Foundation

- [ ] Create first GitHub release (v0.1.0)
- [ ] Verify binary artifacts for all 4 platforms
- [ ] Add `--version`, `--health`, `--help` flags
- [ ] Test binaries on each platform

### Week 2: Installation Scripts

- [ ] Create `install.sh` for macOS/Linux
- [ ] Create `install.ps1` for Windows
- [ ] Create `configure.sh` wizard
- [ ] Test on fresh VMs (macOS, Ubuntu, Windows)

### Week 3: Package Managers

- [ ] Create Homebrew tap + formula
- [ ] Create Chocolatey package
- [ ] Submit to Chocolatey community
- [ ] Test installations

### Week 4: Advanced Distribution

- [ ] Create npm wrapper package
- [ ] Publish to npm registry
- [ ] Create Dockerfile + docker-compose
- [ ] Setup GitHub Container Registry

### Week 5: Documentation & Polish

- [ ] Update README with all installation methods
- [ ] Create installation troubleshooting guide
- [ ] Record installation demo videos
- [ ] Update CONTRIBUTING.md

---

## Metrics & Success Criteria

| Metric | Current | Target |
|--------|---------|--------|
| Time to install (with Rust) | 10-15 min | 30 seconds |
| Time to install (no Rust) | N/A | 30 seconds |
| Configuration steps | 3-4 manual | 0 (automated) |
| Supported platforms | 4 | 4 |
| Installation methods | 1 (build) | 7 (script, brew, choco, npm, docker, snap, apt) |
| User errors | High | Low |
| Update complexity | High | `npm update` / `brew upgrade` |

---

## Maintenance Considerations

### Automated Releases

**GitHub Action for auto-release** (`.github/workflows/auto-release.yml`):

```yaml
name: Auto Release

on:
  push:
    branches:
      - main
    paths:
      - 'Cargo.toml'

jobs:
  check-version:
    runs-on: ubuntu-latest
    outputs:
      should_release: ${{ steps.check.outputs.should_release }}
      version: ${{ steps.check.outputs.version }}
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 2

      - name: Check version bump
        id: check
        run: |
          CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
          git checkout HEAD~1
          PREVIOUS_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

          if [ "$CURRENT_VERSION" != "$PREVIOUS_VERSION" ]; then
            echo "should_release=true" >> $GITHUB_OUTPUT
            echo "version=v$CURRENT_VERSION" >> $GITHUB_OUTPUT
          else
            echo "should_release=false" >> $GITHUB_OUTPUT
          fi

  create-release:
    needs: check-version
    if: needs.check-version.outputs.should_release == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Create tag
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git tag -a ${{ needs.check-version.outputs.version }} -m "Release ${{ needs.check-version.outputs.version }}"
          git push origin ${{ needs.check-version.outputs.version }}
```

### Update Notifications

**Add update checker to binary**:

```rust
// src/updates.rs

use reqwest::Client;
use serde::Deserialize;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_URL: &str = "https://api.github.com/repos/quanticsoul4772/mcp-reasoning/releases/latest";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

pub async fn check_for_updates() {
    if let Ok(latest_version) = fetch_latest_version().await {
        let current = semver::Version::parse(CURRENT_VERSION).unwrap();
        let latest = semver::Version::parse(&latest_version.trim_start_matches('v')).unwrap();

        if latest > current {
            eprintln!("📦 New version available: {} -> {}", CURRENT_VERSION, latest_version);
            eprintln!("   Update: npm update -g @mcp-reasoning/server");
            eprintln!("        or: brew upgrade mcp-reasoning");
        }
    }
}

async fn fetch_latest_version() -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response: Release = client
        .get(RELEASES_URL)
        .header("User-Agent", "mcp-reasoning")
        .send()
        .await?
        .json()
        .await?;

    Ok(response.tag_name)
}
```

---

## Security Considerations

### API Key Handling

**Current Issue**: API keys stored in plain text in `claude_desktop_config.json`

**Proposed Solutions**:

1. **System Keychain Integration**:

   ```bash
   # macOS: Use Keychain
   security add-generic-password -a mcp-reasoning -s anthropic-api-key -w

   # Linux: Use Secret Service API (libsecret)
   secret-tool store --label='MCP Reasoning API Key' service mcp-reasoning key api

   # Windows: Use Credential Manager
   cmdkey /generic:mcp-reasoning /user:api-key /pass:sk-ant-xxx
   ```

2. **Encrypted Storage**:
   - Store encrypted API key in config
   - Decrypt on startup using machine-specific key

3. **Environment Variable Recommendation**:
   - Prefer `ANTHROPIC_API_KEY` env var
   - Documentation emphasizes this approach

### Binary Verification

**Add checksums to releases**:

```yaml
# .github/workflows/release.yml addition
- name: Generate checksums
  run: |
    cd artifacts
    for file in *.tar.gz *.zip; do
      sha256sum "$file" > "$file.sha256"
    done
```

**Users can verify**:

```bash
# Download checksum
curl -LO https://github.com/.../mcp-reasoning-linux.tar.gz.sha256

# Verify
sha256sum -c mcp-reasoning-linux.tar.gz.sha256
```

---

## Cost Analysis

### Development Time

| Phase | Estimated Hours | Priority |
|-------|----------------|----------|
| GitHub releases + flags | 4h | P0 |
| Installation scripts | 8h | P0 |
| Homebrew formula | 4h | P1 |
| npm wrapper | 6h | P1 |
| Chocolatey package | 4h | P2 |
| Docker image | 3h | P2 |
| Configure wizard | 4h | P1 |
| Documentation | 4h | P0 |
| **Total** | **37h** | |

### Maintenance Cost

- **Automated**: Release workflow already configured
- **Per-release overhead**: ~30 minutes (verify builds, test installations)
- **Package manager updates**: Automatic via version bumps

---

## Recommendations

### Immediate Actions (Week 1)

1. ✅ **Create v0.1.0 release** - Unlock pre-built binaries
2. ✅ **Add CLI flags** - `--version`, `--health`, `--help`
3. ✅ **Create install.sh** - macOS/Linux one-command install
4. ✅ **Create install.ps1** - Windows one-command install

### Short-term (Weeks 2-3)

5. ✅ **Homebrew tap** - Easiest for macOS developers
6. ✅ **npm package** - Easiest for JavaScript ecosystem
7. ✅ **Documentation** - Update README with all methods

### Medium-term (Month 2)

8. ⏳ **Chocolatey** - Windows package manager
9. ⏳ **Docker** - Server deployments
10. ⏳ **Auto-update checker** - Notify users of new versions

### Long-term (3+ months)

11. ⏳ **Homebrew Core** - Official Homebrew distribution
12. ⏳ **Snap/APT** - Linux distribution packages
13. ⏳ **Keychain integration** - Secure API key storage
14. ⏳ **GUI installer** - Windows MSI, macOS .pkg

---

## Conclusion

The current installation process is functional but has significant friction:

- Requires Rust toolchain (500MB, 10+ min)
- Manual configuration prone to errors
- No update mechanism

**Proposed improvements reduce installation time from 10-15 minutes to 30 seconds** and eliminate 95% of user errors through:

1. **Pre-built binaries** (already configured in release.yml)
2. **One-command installers** (platform-specific scripts)
3. **Package managers** (npm, Homebrew, Chocolatey)
4. **Automated configuration** (interactive wizard)
5. **Health checks** (verify installation success)

**Recommended priority order**:

1. GitHub release + installers (Week 1) - Highest impact
2. npm + Homebrew (Week 2-3) - Widest reach
3. Documentation (Week 3) - Critical for adoption
4. Advanced options (Month 2+) - Nice to have

This transforms mcp-reasoning from a developer tool requiring Rust knowledge to a production-ready server installable by anyone in 30 seconds.
