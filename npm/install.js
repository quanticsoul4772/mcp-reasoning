#!/usr/bin/env node

const https = require('https');
const http = require('http');
const fs = require('fs');
const path = require('path');
const zlib = require('zlib');
const { pipeline } = require('stream');
const { promisify } = require('util');

const streamPipeline = promisify(pipeline);

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
  console.error(`❌ Unsupported platform: ${platform}-${arch}`);
  console.error('Supported platforms:');
  Object.keys(TARGETS).forEach(k => console.error(`  - ${k}`));
  process.exit(1);
}

const isWindows = platform === 'win32';
const ext = isWindows ? 'zip' : 'tar.gz';
const binaryName = isWindows ? 'mcp-reasoning.exe' : 'mcp-reasoning';

const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${target}.${ext}`;
const binDir = path.join(__dirname, 'bin');
const archivePath = path.join(__dirname, `download.${ext}`);

console.log(`📦 Installing mcp-reasoning v${VERSION} for ${platform}-${arch}...`);
console.log(`⬇️  Downloading from: ${url}`);

// Create bin directory
if (!fs.existsSync(binDir)) {
  fs.mkdirSync(binDir, { recursive: true });
}

// Download file
async function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    const request = (url.startsWith('https') ? https : http).get(url, (response) => {
      // Follow redirects
      if (response.statusCode === 301 || response.statusCode === 302) {
        download(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
        return;
      }

      response.pipe(file);
      file.on('finish', () => {
        file.close(resolve);
      });
    });

    request.on('error', (err) => {
      fs.unlink(dest, () => reject(err));
    });

    file.on('error', (err) => {
      fs.unlink(dest, () => reject(err));
    });
  });
}

// Extract archive
async function extract(archivePath, destDir) {
  if (isWindows) {
    // Handle ZIP for Windows
    const AdmZip = require('adm-zip');
    const zip = new AdmZip(archivePath);
    zip.extractAllTo(destDir, true);
  } else {
    // Handle tar.gz for Unix
    const tar = require('tar');
    await tar.extract({
      file: archivePath,
      cwd: destDir,
    });
  }
}

// Main installation
(async () => {
  try {
    // Download
    await download(url, archivePath);
    console.log('✅ Download complete');

    // Extract
    console.log('📦 Extracting archive...');
    await extract(archivePath, binDir);
    fs.unlinkSync(archivePath);
    console.log('✅ Extraction complete');

    // Make executable (Unix only)
    if (!isWindows) {
      const binaryPath = path.join(binDir, binaryName);
      fs.chmodSync(binaryPath, 0o755);
    }

    console.log('');
    console.log('✅ Installation complete!');
    console.log('');
    console.log(`Binary installed at: ${path.join(binDir, binaryName)}`);
    console.log('');
    console.log('Run: mcp-reasoning --version');
    console.log('');
    console.log('Documentation: https://github.com/' + REPO);

  } catch (error) {
    console.error('');
    console.error('❌ Installation failed:', error.message);
    console.error('');
    console.error('Please report this issue at:');
    console.error(`  https://github.com/${REPO}/issues`);
    process.exit(1);
  }
})();
