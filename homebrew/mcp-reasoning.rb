# Homebrew formula for mcp-reasoning
# Installation:
#   brew tap quanticsoul4772/mcp
#   brew install mcp-reasoning

class McpReasoning < Formula
  desc "MCP server providing structured reasoning capabilities for Claude"
  homepage "https://github.com/quanticsoul4772/mcp-reasoning"
  version "0.3.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.3.0/x86_64-apple-darwin.tar.gz"
      sha256 "0635244d3e8e4eea7fad3d6b525093a5c6ad421fc8b3076cefd5275b87cc98e9"
    elsif Hardware::CPU.arm?
      url "https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.3.0/aarch64-apple-darwin.tar.gz"
      sha256 "cdd894bd623e545cdbfe05aeec6212c3e2bbf4015322eb98f1c07f9c6c73250c"
    end
  end

  on_linux do
    url "https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.3.0/x86_64-unknown-linux-gnu.tar.gz"
    sha256 "907d5ba4c591c36e20ade6fed70affdded6a1aace9f92c43b05e11b82d5a84cb"
  end

  def install
    bin.install "mcp-reasoning"
  end

  def caveats
    <<~EOS
      To configure Claude Desktop:

      1. Get an Anthropic API key from https://console.anthropic.com/

      2. Edit your Claude Desktop config:
         macOS: ~/Library/Application Support/Claude/claude_desktop_config.json
         Linux: ~/.config/Claude/claude_desktop_config.json

      3. Add this configuration:

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

      Or use the interactive configuration wizard:
         Run: curl -fsSL https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/configure.sh | bash

      Documentation: https://github.com/quanticsoul4772/mcp-reasoning
    EOS
  end

  test do
    assert_match "mcp-reasoning", shell_output("#{bin}/mcp-reasoning --version")
  end
end
