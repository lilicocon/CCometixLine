#!/usr/bin/env node
const { spawnSync } = require('child_process');
const fs = require('fs');
const identity = require('../scripts/identity');

function run(binaryPath) {
  const result = spawnSync(binaryPath, process.argv.slice(2), {
    stdio: 'inherit',
    shell: false
  });
  identity.forwardSpawnResult(result);
}

// 1. Priority: Use ~/.claude/ccline/ccline if exists
const claudePath = identity.claudeBinaryPath();

if (fs.existsSync(claudePath)) {
  run(claudePath);
}

// 2. Fallback: Use npm package binary
const platformKey = identity.detectPlatformKey();
const packageName = identity.platformPackageName(platformKey);
if (!packageName) {
  console.error(`Error: Unsupported platform ${platformKey}`);
  console.error('Supported platforms: darwin (x64/arm64), linux (x64/arm64), win32 (x64)');
  console.error(`Please visit ${identity.repoUrl()} for manual installation`);
  process.exit(1);
}

const binaryName = identity.binaryFileName();
const binaryPath = identity.resolvePackageBinary(packageName, binaryName);

if (!binaryPath) {
  console.error(`Error: Binary not found at ${identity.nestedBinaryPath(packageName, binaryName)}`);
  console.error('This might indicate a failed installation or unsupported platform.');
  console.error(`Please try reinstalling: npm install -g ${identity.mainName}`);
  console.error(`Expected package: ${packageName}`);
  process.exit(1);
}

run(binaryPath);
