#!/usr/bin/env node
/**
 * Synchronizes version across all config files.
 *
 * Usage:
 *   node scripts/bump-version.mjs 0.2.0        # Set specific version
 *   node scripts/bump-version.mjs patch         # 0.1.0 → 0.1.1
 *   node scripts/bump-version.mjs minor         # 0.1.0 → 0.2.0
 *   node scripts/bump-version.mjs major         # 0.1.0 → 1.0.0
 *
 * Files updated:
 *   - package.json
 *   - package-lock.json
 *   - src-tauri/Cargo.toml
 *   - src-tauri/tauri.conf.json
 */

import { readFileSync, writeFileSync } from 'fs';
import { resolve, join } from 'path';

const ROOT = resolve(import.meta.dirname, '..');

// ─── Read current version ──────────────────────────────────────────────────────
const pkgPath = join(ROOT, 'package.json');
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
const current = pkg.version;

// ─── Determine new version ─────────────────────────────────────────────────────
const arg = process.argv[2];
if (!arg) {
  console.log(`Current version: ${current}`);
  console.log('Usage: node scripts/bump-version.mjs <patch|minor|major|x.y.z>');
  process.exit(0);
}

function bump(version, type) {
  const [major, minor, patch] = version.split('.').map(Number);
  switch (type) {
    case 'patch': return `${major}.${minor}.${patch + 1}`;
    case 'minor': return `${major}.${minor + 1}.0`;
    case 'major': return `${major + 1}.0.0`;
    default: return null;
  }
}

const newVersion = bump(current, arg) || arg;

// Validate semver format
if (!/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(newVersion)) {
  console.error(`Invalid version: "${newVersion}". Use semver format (e.g., 1.2.3)`);
  process.exit(1);
}

console.log(`\x1b[1mBumping version: ${current} → ${newVersion}\x1b[0m\n`);

// ─── 1. package.json ───────────────────────────────────────────────────────────
pkg.version = newVersion;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
console.log(`  ✓ package.json`);

// ─── 2. package-lock.json ──────────────────────────────────────────────────────
const lockPath = join(ROOT, 'package-lock.json');
try {
  const lock = JSON.parse(readFileSync(lockPath, 'utf8'));
  lock.version = newVersion;
  if (lock.packages && lock.packages['']) {
    lock.packages[''].version = newVersion;
  }
  writeFileSync(lockPath, JSON.stringify(lock, null, 2) + '\n');
  console.log(`  ✓ package-lock.json`);
} catch {
  console.log(`  ⚠ package-lock.json (not found, skipped)`);
}

// ─── 3. src-tauri/Cargo.toml ───────────────────────────────────────────────────
const cargoPath = join(ROOT, 'src-tauri', 'Cargo.toml');
let cargo = readFileSync(cargoPath, 'utf8');
cargo = cargo.replace(
  /^version\s*=\s*"[^"]+"/m,
  `version = "${newVersion}"`
);
writeFileSync(cargoPath, cargo);
console.log(`  ✓ src-tauri/Cargo.toml`);

// ─── 4. src-tauri/tauri.conf.json ──────────────────────────────────────────────
const tauriConfPath = join(ROOT, 'src-tauri', 'tauri.conf.json');
const tauriConf = JSON.parse(readFileSync(tauriConfPath, 'utf8'));
tauriConf.version = newVersion;
writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n');
console.log(`  ✓ src-tauri/tauri.conf.json`);

console.log(`\n\x1b[32m✓ All files updated to v${newVersion}\x1b[0m`);
