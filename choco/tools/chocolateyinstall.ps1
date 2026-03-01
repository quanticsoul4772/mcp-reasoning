$ErrorActionPreference = 'Stop'

$packageName = 'mcp-reasoning'
$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url64 = 'https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-pc-windows-msvc.zip'

$packageArgs = @{
  packageName   = $packageName
  unzipLocation = $toolsDir
  url64bit      = $url64
  checksum64    = 'PLACEHOLDER_SHA256'  # Will be filled after first release
  checksumType64= 'sha256'
}

Install-ChocolateyZipPackage @packageArgs

Write-Host "✅ mcp-reasoning installed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "1. Get an Anthropic API key: https://console.anthropic.com/"
Write-Host "2. Set environment variable:"
Write-Host "   [Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', 'your-key-here', 'User')"
Write-Host "3. Run health check:"
Write-Host "   mcp-reasoning --health"
Write-Host ""
Write-Host "Documentation: https://github.com/quanticsoul4772/mcp-reasoning"
