# Homebrew formula for mcp-reasoning
# Installation:
#   brew tap quanticsoul4772/mcp
#   brew install mcp-reasoning

class McpReasoning < Formula
  desc "MCP server providing structured reasoning capabilities for Claude"
  homepage "https://github.com/quanticsoul4772/mcp-reasoning"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-apple-darwin.tar.gz"
      sha256 "b0329f6030befba49b028918549d9e480aa0f8ff8344ed77bf77dbaef7afd367"
    elsif Hardware::CPU.arm?
      url "https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/aarch64-apple-darwin.tar.gz"
      sha256 "9925679c4f327aba3fbeacc497cc14ecece3b5095ca1a32f06b9502e9edda9ef"
    end
  end

  on_linux do
    url "https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-unknown-linux-gnu.tar.gz"
    sha256 "ad0a3d62b1300691ff150b938ba2c82a6681b29b05a1fbaaa42f4b520596fd8a"
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
