#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..', '..');
const mainTemplatePath = path.join(repoRoot, 'npm', 'main', 'package.json');
const mainTemplate = JSON.parse(fs.readFileSync(mainTemplatePath, 'utf8'));
const mainName = mainTemplate.name;

// Marks an --out-dir as ours, so re-runs may wipe it and nothing else
const OUT_MARKER = '.ccline-npm-publish';

const VERSION_RE = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

// platform key -> os, cpu, binary file name, description
const PLATFORMS = {
  'darwin-x64': {
    os: ['darwin'],
    cpu: ['x64'],
    binary: 'ccline',
    description: 'macOS x64 binary for CCometixLine',
  },
  'darwin-arm64': {
    os: ['darwin'],
    cpu: ['arm64'],
    binary: 'ccline',
    description: 'macOS ARM64 binary for CCometixLine',
  },
  'linux-x64': {
    os: ['linux'],
    cpu: ['x64'],
    binary: 'ccline',
    description: 'Linux x64 binary for CCometixLine',
  },
  'linux-x64-musl': {
    os: ['linux'],
    cpu: ['x64'],
    binary: 'ccline',
    description: 'Linux x64 static binary for CCometixLine (musl libc)',
  },
  'linux-arm64': {
    os: ['linux'],
    cpu: ['arm64'],
    binary: 'ccline',
    description: 'Linux ARM64 binary for CCometixLine',
  },
  'linux-arm64-musl': {
    os: ['linux'],
    cpu: ['arm64'],
    binary: 'ccline',
    description: 'Linux ARM64 binary (musl) for CCometixLine',
  },
  'win32-x64': {
    os: ['win32'],
    cpu: ['x64'],
    binary: 'ccline.exe',
    description: 'Windows x64 binary for CCometixLine',
  },
};

const PLATFORM_KEYS = Object.keys(PLATFORMS);

function usage() {
  return [
    'Usage: node npm/scripts/prepare-packages.js <version> [--bin-dir DIR] [--out-dir DIR] [--allow-missing]',
    '   or: GITHUB_REF=refs/tags/vX.Y.Z node npm/scripts/prepare-packages.js [--bin-dir DIR] [--out-dir DIR] [--allow-missing]',
  ].join('\n');
}

function fail(message) {
  console.error(`Error: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = {
    version: null,
    binDir: null,
    outDir: null,
    allowMissing: false,
  };

  const rest = argv.slice(2);
  for (let i = 0; i < rest.length; i++) {
    const arg = rest[i];
    if (arg === '--allow-missing') {
      args.allowMissing = true;
      continue;
    }
    if (arg === '--bin-dir' || arg === '--out-dir') {
      const value = rest[i + 1];
      if (!value || value.startsWith('-')) {
        fail(`${arg} requires a directory argument\n${usage()}`);
      }
      if (arg === '--bin-dir') {
        args.binDir = value;
      } else {
        args.outDir = value;
      }
      i += 1;
      continue;
    }
    if (arg.startsWith('-')) {
      fail(`Unknown option: ${arg}\n${usage()}`);
    }
    if (args.version) {
      fail(`Unexpected argument: ${arg}\n${usage()}`);
    }
    args.version = arg;
  }

  if (!args.version) {
    const ref = process.env.GITHUB_REF || '';
    const match = ref.match(/^refs\/tags\/v(.+)$/);
    if (match) {
      args.version = match[1];
    }
  }

  return args;
}

function sourceBinaryName(platformKey, binary) {
  return binary.endsWith('.exe') ? `ccline-${platformKey}.exe` : `ccline-${platformKey}`;
}

function sharedMetadata() {
  return {
    license: mainTemplate.license,
    author: mainTemplate.author,
    contributors: mainTemplate.contributors,
    repository: mainTemplate.repository,
    homepage: mainTemplate.homepage,
    bugs: mainTemplate.bugs,
  };
}

function copyDir(src, dest) {
  fs.mkdirSync(dest, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const from = path.join(src, entry.name);
    const to = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDir(from, to);
    } else if (entry.isSymbolicLink()) {
      fs.symlinkSync(fs.readlinkSync(from), to);
    } else {
      fs.copyFileSync(from, to);
    }
  }
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, JSON.stringify(value, null, 2) + '\n');
}

function platformReadme(platformKey) {
  const pkgName = `${mainName}-${platformKey}`;
  return [
    `# ${pkgName}`,
    '',
    `This package contains the ${platformKey} binary for ${mainName}.`,
    '',
    `Do not install it directly; install ${mainName} instead.`,
    '',
  ].join('\n');
}

function main() {
  const args = parseArgs(process.argv);
  if (!args.version) {
    fail(`Version not provided\n${usage()}`);
  }
  if (!VERSION_RE.test(args.version)) {
    fail(`Invalid version '${args.version}'. Expected X.Y.Z or X.Y.Z-prerelease`);
  }

  const version = args.version;
  const outDir = path.resolve(args.outDir || path.join(repoRoot, 'npm-publish'));
  const binDir = args.binDir ? path.resolve(args.binDir) : null;

  if (outDir === repoRoot) {
    fail('--out-dir must not be the repository root');
  }

  const licenseSrc = path.join(repoRoot, 'LICENSE');
  if (!fs.existsSync(licenseSrc)) {
    fail(`LICENSE not found at ${licenseSrc}`);
  }

  const missing = [];
  const skipped = [];
  const includedKeys = [];

  for (const platformKey of PLATFORM_KEYS) {
    const meta = PLATFORMS[platformKey];
    if (!binDir) {
      includedKeys.push(platformKey);
      continue;
    }
    const sourceName = sourceBinaryName(platformKey, meta.binary);
    const sourcePath = path.join(binDir, sourceName);
    if (fs.existsSync(sourcePath)) {
      includedKeys.push(platformKey);
    } else if (args.allowMissing) {
      skipped.push({ platformKey, sourcePath });
    } else {
      missing.push(sourcePath);
    }
  }

  if (missing.length) {
    console.error('Error: missing binaries:');
    for (const filePath of missing) {
      console.error(`  ${filePath}`);
    }
    process.exit(1);
  }

  if (skipped.length) {
    console.warn('WARNING: --allow-missing is set; skipping platforms with missing binaries:');
    for (const item of skipped) {
      console.warn(`  ${mainName}-${item.platformKey}  (missing ${item.sourcePath})`);
    }
  }

  if (fs.existsSync(outDir)) {
    const isOurs = fs.existsSync(path.join(outDir, OUT_MARKER));
    if (!isOurs && fs.readdirSync(outDir).length > 0) {
      fail(`--out-dir ${outDir} is not empty and was not created by this script; refusing to delete it`);
    }
    fs.rmSync(outDir, { recursive: true, force: true });
  }
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(path.join(outDir, OUT_MARKER), '');

  const generated = [];
  const meta = sharedMetadata();

  for (const platformKey of includedKeys) {
    const platform = PLATFORMS[platformKey];
    const targetDir = path.join(outDir, platformKey);
    fs.mkdirSync(targetDir, { recursive: true });

    const packageJson = {
      name: `${mainName}-${platformKey}`,
      version,
      description: platform.description,
      files: [platform.binary, 'LICENSE', 'README.md'],
      os: platform.os,
      cpu: platform.cpu,
      ...meta,
    };
    writeJson(path.join(targetDir, 'package.json'), packageJson);
    fs.copyFileSync(licenseSrc, path.join(targetDir, 'LICENSE'));
    fs.writeFileSync(path.join(targetDir, 'README.md'), platformReadme(platformKey));

    if (binDir) {
      const sourcePath = path.join(binDir, sourceBinaryName(platformKey, platform.binary));
      const destPath = path.join(targetDir, platform.binary);
      fs.copyFileSync(sourcePath, destPath);
      if (platformKey !== 'win32-x64') {
        fs.chmodSync(destPath, 0o755);
      }
    }

    generated.push(`${packageJson.name}@${version}`);
  }

  const mainSource = path.join(repoRoot, 'npm', 'main');
  const mainTarget = path.join(outDir, 'main');
  copyDir(mainSource, mainTarget);
  fs.copyFileSync(licenseSrc, path.join(mainTarget, 'LICENSE'));

  const mainPackageJson = JSON.parse(fs.readFileSync(path.join(mainTarget, 'package.json'), 'utf8'));
  mainPackageJson.version = version;
  mainPackageJson.optionalDependencies = {};
  for (const platformKey of includedKeys) {
    mainPackageJson.optionalDependencies[`${mainName}-${platformKey}`] = version;
  }
  writeJson(path.join(mainTarget, 'package.json'), mainPackageJson);
  generated.push(`${mainName}@${version}`);

  console.log(`Prepared ${generated.length} packages for ${version}:`);
  for (const name of generated) {
    console.log(`  ${name}`);
  }
  console.log(`Output: ${outDir}`);
}

try {
  main();
} catch (error) {
  fail(error && error.stack ? error.stack : String(error));
}
