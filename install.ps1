# install.ps1 - One-command installation for Windows
# Usage: irm https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.ps1 | iex

param(
    [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"

$REPO = "quanticsoul4772/mcp-reasoning"
$TARGET = "x86_64-pc-windows-msvc"

Write-Host "🔧 MCP Reasoning Server Installer" -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan
Write-Host ""

# Check architecture
if ([System.Environment]::Is64BitOperatingSystem) {
    Write-Host "📦 Detected: Windows (x86_64)" -ForegroundColor Green
} else {
    Write-Host "❌ Unsupported: 32-bit Windows" -ForegroundColor Red
    Write-Host "This application requires 64-bit Windows."
    exit 1
}

Write-Host ""

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

try {
    Invoke-WebRequest -Uri $URL -OutFile $ZIP_PATH -UseBasicParsing
    Expand-Archive -Path $ZIP_PATH -DestinationPath $INSTALL_DIR -Force
    Remove-Item $ZIP_PATH
} catch {
    Write-Host "❌ Download failed: $_" -ForegroundColor Red
    exit 1
}

Write-Host "✅ Binary installed to: $INSTALL_DIR\mcp-reasoning.exe" -ForegroundColor Green
Write-Host ""

# Add to PATH
$USER_PATH = [Environment]::GetEnvironmentVariable("Path", "User")
if ($USER_PATH -notlike "*$INSTALL_DIR*") {
    Write-Host "➕ Adding to PATH..." -ForegroundColor Yellow
    try {
        [Environment]::SetEnvironmentVariable(
            "Path",
            "$USER_PATH;$INSTALL_DIR",
            "User"
        )
        Write-Host "✅ Added to PATH" -ForegroundColor Green
        Write-Host "⚠️  Restart your terminal for PATH changes to take effect" -ForegroundColor Yellow
    } catch {
        Write-Host "⚠️  Could not add to PATH automatically" -ForegroundColor Yellow
        Write-Host "Add manually: $INSTALL_DIR"
    }
} else {
    Write-Host "✅ Already in PATH" -ForegroundColor Green
}

Write-Host ""

# Verify installation
Write-Host "Verifying installation..." -ForegroundColor Yellow
try {
    & "$INSTALL_DIR\mcp-reasoning.exe" --version | Out-Null
    Write-Host "✅ Installation verified" -ForegroundColor Green
} catch {
    Write-Host "⚠️  Installation may have issues" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "✨ Installation complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "1. Get an Anthropic API key: https://console.anthropic.com/"
Write-Host "2. Set environment variable:"
Write-Host "   `$env:ANTHROPIC_API_KEY = 'your-key-here'" -ForegroundColor White
Write-Host "3. Run health check:"
Write-Host "   mcp-reasoning --health" -ForegroundColor White
Write-Host ""
Write-Host "For Claude Desktop configuration:"
Write-Host "  Edit: $env:APPDATA\Claude\claude_desktop_config.json"
Write-Host ""
Write-Host "Documentation: https://github.com/$REPO"
