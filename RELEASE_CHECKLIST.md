# Release Checklist - v0.1.0

**Date**: 2026-03-01
**Status**: Ready for Release

---

## Pre-Release Verification ✅

All items completed and verified:

- [x] All installation improvements implemented
- [x] CLI enhancements complete (--version, --health, --help)
- [x] 6 installation methods created
- [x] Documentation updated (README, CHANGELOG)
- [x] All changes committed and pushed to main
- [x] GitHub Actions workflows configured
- [x] Pre-commit hooks passing
- [x] All tests passing (2,020 tests)
- [x] Build succeeds (cargo build --release)

---

## Release Process

### Step 1: Create Release Tag

```bash
# On main branch
git tag -a v0.1.0 -m "Release v0.1.0 - Initial public release with comprehensive installation options"
git push origin v0.1.0
```

**What This Triggers:**

- GitHub Actions `release.yml` workflow starts
- Builds binaries for 4 platforms:
  - `x86_64-unknown-linux-gnu` (Linux)
  - `x86_64-apple-darwin` (macOS Intel)
  - `aarch64-apple-darwin` (macOS ARM/M1/M2/M3)
  - `x86_64-pc-windows-msvc` (Windows)
- Creates GitHub Release with artifacts
- Publishes `.tar.gz` and `.zip` archives

**Estimated Time**: 10-15 minutes

---

## Post-Release Tasks

### Step 2: Update SHA256 Checksums

After release artifacts are published, calculate checksums:

```bash
# Download all release artifacts
cd /tmp
wget https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-apple-darwin.tar.gz
wget https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/aarch64-apple-darwin.tar.gz
wget https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-unknown-linux-gnu.tar.gz
wget https://github.com/quanticsoul4772/mcp-reasoning/releases/download/v0.1.0/x86_64-pc-windows-msvc.zip

# Calculate SHA256
sha256sum *.tar.gz *.zip
```

**Update Files:**

1. `homebrew/mcp-reasoning.rb` - Replace `PLACEHOLDER_INTEL_SHA256`, `PLACEHOLDER_ARM_SHA256`, `PLACEHOLDER_LINUX_SHA256`
2. `choco/tools/chocolateyinstall.ps1` - Replace `PLACEHOLDER_SHA256`

**Commit:**

```bash
git add homebrew/mcp-reasoning.rb choco/tools/chocolateyinstall.ps1
git commit -m "chore: Update SHA256 checksums for v0.1.0 release"
git push origin main
```

---

### Step 3: Test All Installation Methods

**Test on fresh systems (VMs or containers):**

1. **macOS (Intel)**:

   ```bash
   curl -fsSL https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.sh | bash
   mcp-reasoning --version
   ```

2. **macOS (ARM/M1)**:

   ```bash
   brew tap quanticsoul4772/mcp
   brew install mcp-reasoning
   mcp-reasoning --version
   ```

3. **Linux**:

   ```bash
   curl -fsSL https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.sh | bash
   mcp-reasoning --health  # requires ANTHROPIC_API_KEY
   ```

4. **Windows**:

   ```powershell
   irm https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.ps1 | iex
   mcp-reasoning --version
   ```

5. **npm**:

   ```bash
   npx @mcp-reasoning/server --version
   ```

6. **Docker**:

   ```bash
   docker pull ghcr.io/quanticsoul4772/mcp-reasoning:latest
   docker run --rm ghcr.io/quanticsoul4772/mcp-reasoning:latest --version
   ```

**Expected Output:**

```
mcp-reasoning 0.1.0
Rust MCP server for structured reasoning
...
```

---

### Step 4: Publish npm Package

```bash
cd npm

# Login to npm (one-time)
npm login

# Publish to npm registry
npm publish --access public

# Verify
npm view @mcp-reasoning/server
```

**Test npm installation:**

```bash
# Global
npm install -g @mcp-reasoning/server
mcp-reasoning --version

# npx (no install)
npx @mcp-reasoning/server --version
```

---

### Step 5: Submit to Package Repositories

#### Homebrew Tap (Already Available)

The tap is already created at:

```bash
brew tap quanticsoul4772/mcp
brew install mcp-reasoning
```

#### Homebrew Core (Future - After Maturity)

**Requirements:**

- 75+ GitHub stars
- 30+ forks
- 30+ days since first release
- Notable project (actively maintained)

**When ready:**

1. Fork `homebrew/homebrew-core`
2. Add formula to `Formula/mcp-reasoning.rb`
3. Create PR with formula
4. Wait for review and approval

#### Chocolatey Community Repository

```bash
# Package the .nupkg
cd choco
choco pack

# Push to Chocolatey (requires API key)
choco push mcp-reasoning.0.1.0.nupkg --source https://push.chocolatey.org/

# Verify
choco search mcp-reasoning
```

**Note**: Chocolatey moderation can take 3-5 days for first package.

---

## Verification Checklist

After all post-release tasks:

- [ ] All 4 platform binaries available on GitHub Releases
- [ ] SHA256 checksums updated in Homebrew and Chocolatey configs
- [ ] install.sh works on macOS and Linux
- [ ] install.ps1 works on Windows
- [ ] Homebrew tap works (`brew install mcp-reasoning`)
- [ ] npm package published and working (`npx @mcp-reasoning/server`)
- [ ] Docker image available (`docker pull ghcr.io/...`)
- [ ] Chocolatey package submitted (pending moderation)
- [ ] All installation methods tested and verified
- [ ] Claude Desktop integration tested with at least 2 methods

---

## Rollback Plan

If critical issues are found post-release:

1. **Delete the tag:**

   ```bash
   git tag -d v0.1.0
   git push --delete origin v0.1.0
   ```

2. **Delete the GitHub Release:**
   - Go to <https://github.com/quanticsoul4772/mcp-reasoning/releases>
   - Delete the v0.1.0 release

3. **Fix issues on main branch**

4. **Create new release** (v0.1.1 or v0.2.0 depending on changes)

---

## Success Metrics

After release, monitor:

- **GitHub Releases**: Download counts for each platform
- **npm**: Weekly downloads (npmjs.com stats)
- **Docker**: Pull counts (GitHub Container Registry)
- **GitHub**: Stars, forks, issues, PRs
- **User Feedback**: Issues reported vs resolved

---

## Communication

### Release Announcement

**Channels:**

- GitHub Release Notes (auto-generated + custom)
- README.md updated badge (if applicable)
- Social media (optional)

**Template Release Notes:**

```markdown
# mcp-reasoning v0.1.0 - Initial Release

## 🎉 First Public Release

MCP Reasoning Server is now available with 6 easy installation methods!

### ✨ Highlights

- **15 Reasoning Tools**: Linear, tree, divergent, graph, MCTS, counterfactual, and more
- **6 Installation Methods**: One-command installers, npm, Homebrew, Chocolatey, Docker, build from source
- **30-Second Install**: No Rust required for end users
- **Production Ready**: 2,020 tests, 95%+ coverage, zero unsafe code

### 🚀 Quick Start

**macOS/Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.sh | bash
```

**Windows:**

```powershell
irm https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.ps1 | iex
```

**npm (all platforms):**

```bash
npx @mcp-reasoning/server --version
```

See [README](https://github.com/quanticsoul4772/mcp-reasoning#readme) for all installation methods.

### 📦 Platform Support

- macOS (Intel + Apple Silicon)
- Linux (x86_64)
- Windows (x86_64)
- Docker (linux/amd64)

### 🔗 Links

- [Documentation](https://github.com/quanticsoul4772/mcp-reasoning/blob/main/docs/README.md)
- [API Reference](https://github.com/quanticsoul4772/mcp-reasoning/blob/main/docs/reference/API_SPECIFICATION.md)
- [Installation Guide](https://github.com/quanticsoul4772/mcp-reasoning#installation)

---

**Full Changelog**: <https://github.com/quanticsoul4772/mcp-reasoning/blob/main/CHANGELOG.md>

```

---

## Timeline

**Immediate (< 1 hour):**
- Create and push v0.1.0 tag
- Wait for GitHub Actions to build binaries

**Within 24 hours:**
- Update SHA256 checksums
- Test all installation methods
- Publish npm package

**Within 1 week:**
- Submit to Chocolatey community
- Monitor for issues
- Respond to feedback

**Within 1 month:**
- Gather metrics
- Plan v0.2.0 based on feedback

---

## Notes

- **This is a pre-release**: While fully functional and tested, mark as "pre-release" on GitHub if desired
- **Semantic Versioning**: Follow semver strictly (MAJOR.MINOR.PATCH)
- **Breaking Changes**: Document clearly in CHANGELOG and migration guide
- **Support**: Respond to issues within 48 hours when possible

---

**Status**: ✅ Ready to create v0.1.0 release tag

**Next Action**: Execute Step 1 (create and push tag) to trigger release workflow
