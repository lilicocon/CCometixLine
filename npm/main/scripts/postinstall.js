const fs = require('fs');
const path = require('path');
const os = require('os');
const identity = require('./identity');

if (process.env.CCLINE_SKIP_POSTINSTALL === '1') {
  process.exit(0);
}

// Silent mode detection
const silent = process.env.npm_config_loglevel === 'silent';

if (!silent) {
  console.log('🚀 Setting up CCometixLine for Claude Code...');
}

try {
  const platform = process.platform;
  const homeDir = os.homedir();
  const claudeDir = path.join(homeDir, '.claude', 'ccline');

  // Create directory
  fs.mkdirSync(claudeDir, { recursive: true });

  const platformKey = identity.detectPlatformKey();
  const packageName = identity.platformPackageName(platformKey);
  if (!packageName) {
    if (!silent) {
      console.log(`Platform ${platformKey} not supported for auto-setup`);
    }
    process.exit(0);
  }

  const binaryName = identity.binaryFileName();
  const targetPath = path.join(claudeDir, binaryName);
  const sourcePath = identity.resolveInstalledBinary(packageName, binaryName);

  if (!sourcePath) {
    if (!silent) {
      console.log('Binary package not installed, skipping Claude Code setup');
      console.log('The global ccline command will still work via npm');
    }
    process.exit(0);
  }

  // Copy or link the binary
  if (platform === 'win32') {
    // Windows: Copy file
    fs.copyFileSync(sourcePath, targetPath);
  } else {
    // Unix: Try hard link first, fallback to copy
    try {
      if (fs.existsSync(targetPath)) {
        fs.unlinkSync(targetPath);
      }
      fs.linkSync(sourcePath, targetPath);
    } catch {
      fs.copyFileSync(sourcePath, targetPath);
    }
    fs.chmodSync(targetPath, '755');
  }

  if (!silent) {
    console.log('✨ CCometixLine is ready for Claude Code!');
    console.log(`📍 Location: ${targetPath}`);
    console.log('🎉 You can now use: ccline --help');
  }
} catch (error) {
  // Silent failure - don't break installation
  if (!silent) {
    console.log('Note: Could not auto-configure for Claude Code');
    console.log('The global ccline command will still work.');
    console.log('You can manually copy ccline to ~/.claude/ccline/ if needed');
  }
}
