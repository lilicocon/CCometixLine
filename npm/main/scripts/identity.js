const fs = require('fs');
const os = require('os');
const path = require('path');
const { execSync } = require('child_process');

const pkg = require('../package.json');
const mainName = pkg.name;
const packageRoot = path.join(__dirname, '..');

const SUPPORTED_PLATFORM_KEYS = [
  'darwin-x64',
  'darwin-arm64',
  'linux-x64',
  'linux-x64-musl',
  'linux-arm64',
  'linux-arm64-musl',
  'win32-x64',
];

function repoUrl() {
  const homepage = pkg.homepage && String(pkg.homepage).replace(/#readme$/, '');
  if (homepage) {
    return homepage;
  }
  const raw = pkg.repository && pkg.repository.url;
  if (!raw) {
    return '';
  }
  return String(raw).replace(/^git\+/, '').replace(/\.git$/, '');
}

function issuesUrl() {
  if (pkg.bugs && pkg.bugs.url) {
    return pkg.bugs.url;
  }
  const base = repoUrl();
  return base ? `${base}/issues` : '';
}

function binaryFileName(platform = process.platform) {
  return platform === 'win32' ? 'ccline.exe' : 'ccline';
}

function claudeBinaryPath() {
  return path.join(os.homedir(), '.claude', 'ccline', binaryFileName());
}

function getLibcInfo() {
  try {
    const lddOutput = execSync('ldd --version 2>/dev/null || echo ""', {
      encoding: 'utf8',
      timeout: 1000
    });

    // Check for musl explicitly
    if (lddOutput.includes('musl')) {
      return { type: 'musl' };
    }

    // Parse glibc version: "ldd (GNU libc) 2.35" format
    const match = lddOutput.match(/(?:GNU libc|GLIBC).*?(\d+)\.(\d+)/);
    if (match) {
      const major = parseInt(match[1]);
      const minor = parseInt(match[2]);
      return { type: 'glibc', major, minor };
    }

    // If we can't detect, default to musl for safety (more portable)
    return { type: 'musl' };
  } catch (e) {
    // If detection fails, default to musl (more portable)
    return { type: 'musl' };
  }
}

function detectPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;
  let platformKey = `${platform}-${arch}`;

  if (platform === 'linux') {
    const libcInfo = getLibcInfo();

    if (arch === 'arm64') {
      // ARM64 Linux: choose based on libc type and version
      if (libcInfo.type === 'musl' ||
          (libcInfo.type === 'glibc' && (libcInfo.major < 2 || (libcInfo.major === 2 && libcInfo.minor < 35)))) {
        platformKey = 'linux-arm64-musl';
      } else {
        platformKey = 'linux-arm64';
      }
    } else {
      // x64 Linux: choose based on libc type and version
      if (libcInfo.type === 'musl' ||
          (libcInfo.type === 'glibc' && (libcInfo.major < 2 || (libcInfo.major === 2 && libcInfo.minor < 35)))) {
        platformKey = 'linux-x64-musl';
      }
    }
  }

  return platformKey;
}

function canonicalPlatformKey(platformKey) {
  if (platformKey === 'win32-ia32') {
    return 'win32-x64';
  }
  return platformKey;
}

function platformPackageName(platformKey) {
  const key = canonicalPlatformKey(platformKey);
  if (!SUPPORTED_PLATFORM_KEYS.includes(key)) {
    return null;
  }
  return `${mainName}-${key}`;
}

function nestedBinaryPath(packageName, binaryName) {
  return path.join(packageRoot, 'node_modules', packageName, binaryName);
}

function resolvePackageBinary(packageName, binaryName) {
  try {
    const pkgJson = require.resolve(`${packageName}/package.json`);
    const candidate = path.join(path.dirname(pkgJson), binaryName);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  } catch (e) {
    // Fall back to the nested node_modules layout
  }

  const nested = nestedBinaryPath(packageName, binaryName);
  if (fs.existsSync(nested)) {
    return nested;
  }
  return null;
}

function findPnpmBinary(packageName, binaryName) {
  const currentPath = __dirname;
  const pnpmMatch = currentPath.match(/(.+\.pnpm)[\\/]([^\\//]+)[\\/]/);
  if (!pnpmMatch) {
    return null;
  }

  const pnpmRoot = pnpmMatch[1];
  const packageNameEncoded = packageName.replace('/', '+');

  try {
    const pnpmContents = fs.readdirSync(pnpmRoot);
    const packagePattern = new RegExp(`^${packageNameEncoded.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}@`);
    const matchingPackage = pnpmContents.find(dir => packagePattern.test(dir));

    if (matchingPackage) {
      const candidate = path.join(pnpmRoot, matchingPackage, 'node_modules', packageName, binaryName);
      if (fs.existsSync(candidate)) {
        return candidate;
      }
    }
  } catch (e) {
    // Fallback to other lookup strategies
  }
  return null;
}

function resolveInstalledBinary(packageName, binaryName) {
  return resolvePackageBinary(packageName, binaryName) || findPnpmBinary(packageName, binaryName);
}

function forwardSpawnResult(result) {
  if (result.error) {
    console.error(result.error.message || result.error);
    process.exit(1);
  }
  if (result.signal) {
    try {
      process.kill(process.pid, result.signal);
    } catch (e) {
      // If the signal cannot be delivered, still fail
    }
    process.exit(1);
  }
  process.exit(result.status);
}

module.exports = {
  mainName,
  pkg,
  packageRoot,
  SUPPORTED_PLATFORM_KEYS,
  repoUrl,
  issuesUrl,
  binaryFileName,
  claudeBinaryPath,
  detectPlatformKey,
  canonicalPlatformKey,
  platformPackageName,
  nestedBinaryPath,
  resolvePackageBinary,
  findPnpmBinary,
  resolveInstalledBinary,
  forwardSpawnResult,
};
