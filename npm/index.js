#!/usr/bin/env node

/**
 * @mcp-reasoning/server - npm wrapper
 *
 * This is a thin wrapper around the mcp-reasoning binary.
 * The actual binary is downloaded during postinstall.
 */

const { spawn } = require('child_process');
const path = require('path');

const platform = process.platform;
const binaryName = platform === 'win32' ? 'mcp-reasoning.exe' : 'mcp-reasoning';
const binaryPath = path.join(__dirname, 'bin', binaryName);

// Forward all arguments to the binary
const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  shell: false,
});

child.on('exit', (code) => {
  process.exit(code || 0);
});

child.on('error', (error) => {
  console.error('Failed to start mcp-reasoning:', error.message);
  process.exit(1);
});
