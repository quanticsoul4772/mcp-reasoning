# Chocolatey Package Ready to Submit

The Chocolatey package is complete with verified SHA256 checksum. Ready for submission.

## Package Details

- Package ID: mcp-reasoning
- Version: 0.1.0
- Checksum: 014da2f50ca4651930a8fc8a3ed0db30ef725a87a1d619bdc1f3171686b58bdb (verified)
- Binary URL: <https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-pc-windows-msvc.zip>

## Package Structure

```
choco/
├── mcp-reasoning.nuspec (package metadata)
└── tools/
    ├── chocolateyinstall.ps1 (install script with checksum)
    └── chocolateyuninstall.ps1 (uninstall script)
```

## Submission Options

### Option 1: Command Line (if Chocolatey is installed)

```powershell
# Install Chocolatey first if needed
Set-ExecutionPolicy Bypass -Scope Process -Force
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))

# Pack and submit
cd choco
choco pack
choco push mcp-reasoning.0.1.0.nupkg --source https://push.chocolatey.org/ --api-key YOUR_API_KEY
```

### Option 2: Web Submission (No Chocolatey install required)

1. Get your API key from <https://community.chocolatey.org/account>
2. Create package locally (requires any Windows tool that can create .nupkg):
   - Or just zip the files and rename to .nupkg
3. Upload at <https://community.chocolatey.org/packages/upload>

### Option 3: Manual Package Creation

```powershell
# Create .nupkg manually (it's just a zip file)
cd C:\Development\Projects\MCP\project-root\mcp-servers\mcp-reasoning\choco
Compress-Archive -Path mcp-reasoning.nuspec,tools -DestinationPath mcp-reasoning.0.1.0.zip
Rename-Item mcp-reasoning.0.1.0.zip mcp-reasoning.0.1.0.nupkg
```

Then upload the .nupkg file at <https://community.chocolatey.org/packages/upload>

## After Submission

### Moderation Process

- Automated checks: 1-2 hours
- Manual review: 3-5 business days
- You'll receive email notifications

### Once Approved

Users can install with:

```powershell
choco install mcp-reasoning
```

### Auto-Updates

Users update with:

```powershell
choco upgrade mcp-reasoning
```

## Package Testing

After approval, verify installation:

```powershell
# Fresh install
choco install mcp-reasoning

# Verify binary
mcp-reasoning --version

# Check PATH
where.exe mcp-reasoning
```

## Status

- Package metadata: Complete
- Install script: Complete with verified SHA256
- Uninstall script: Complete
- Checksum verification: Enabled
- Binary URL: Valid (v0.1.0 release)

Ready to submit to Chocolatey community repository.

## Links

- Chocolatey Account: <https://community.chocolatey.org/account>
- Package Upload: <https://community.chocolatey.org/packages/upload>
- Package Guidelines: <https://docs.chocolatey.org/en-us/create/create-packages>
