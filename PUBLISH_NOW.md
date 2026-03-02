# Publishing Instructions - Ready to Execute

Both packages are ready. Follow these steps to publish.

## npm Package - Publish Now

### Step 1: Complete npm Login

A browser window should have opened with this URL:
https://www.npmjs.com/login?next=/login/cli/e17dd1e9-2ffa-4a32-be0b-06e855c61700

1. Complete the login in your browser
2. Wait for "Logged in" confirmation in terminal
3. Continue to Step 2

### Step 2: Publish to npm

```bash
cd npm
npm publish --access public
```

### Step 3: Verify Publication

```bash
npm view @mcp-reasoning/server
```

Or visit: https://www.npmjs.com/package/@mcp-reasoning/server

### Step 4: Test Installation

```bash
# Test npx (zero install)
npx @mcp-reasoning/server --version

# Test global install
npm install -g @mcp-reasoning/server
mcp-reasoning --version
```

---

## Chocolatey Package - Submit Now

### Option 1: Web Submission (Easiest)

1. Get your Chocolatey API key:
   - Visit https://community.chocolatey.org/account
   - Copy your API key

2. Create the package file:
   ```powershell
   cd choco
   # Create .nupkg (it's just a zip)
   Compress-Archive -Path mcp-reasoning.nuspec,tools -DestinationPath mcp-reasoning.0.1.0.zip
   Rename-Item mcp-reasoning.0.1.0.zip mcp-reasoning.0.1.0.nupkg
   ```

3. Upload at https://community.chocolatey.org/packages/upload

### Option 2: Command Line (if choco installed)

```powershell
cd choco
choco pack
choco push mcp-reasoning.0.1.0.nupkg --source https://push.chocolatey.org/ --api-key YOUR_API_KEY
```

### After Submission

- Automated checks: 1-2 hours
- Manual review: 3-5 business days
- Email notification when approved

---

## Quick Summary

**npm**: Login in browser → `cd npm && npm publish --access public`

**Chocolatey**: Get API key → Create .nupkg → Upload at https://community.chocolatey.org/packages/upload

Both packages are verified and ready to go.
