#!/usr/bin/env node
/**
 * Cross-platform build script for StockenBoard Web Server.
 *
 * Usage:
 *   node scripts/build-server.mjs              # Build for current platform (release)
 *   node scripts/build-server.mjs --dev        # Build for current platform (debug)
 *   node scripts/build-server.mjs --target <T> # Cross-compile for specific target
 *   node scripts/build-server.mjs --run        # Build (release) + run
 *   node scripts/build-server.mjs --dev --run  # Build (debug) + run
 *
 * Environment variables:
 *   SB_PORT      - Server port (default: 8080)
 *   SB_BIND      - Bind address (default: 0.0.0.0)
 *   SB_DATA_DIR  - Data directory (default: ./data)
 */

import { execSync, spawn } from 'child_process';
import { existsSync } from 'fs';
import { resolve, join } from 'path';
import { platform, arch } from 'os';

const ROOT = resolve(import.meta.dirname, '..');
const TAURI_DIR = join(ROOT, 'src-tauri');
const DIST_DIR = join(ROOT, 'dist');

// Parse args
const args = process.argv.slice(2);
const isDev = args.includes('--dev');
const shouldRun = args.includes('--run');
const targetIdx = args.indexOf('--target');
const crossTarget = targetIdx !== -1 ? args[targetIdx + 1] : null;

// Detect binary extension
const isWindows = platform() === 'win32';
const binaryExt = isWindows ? '.exe' : '';

// Build profile
const profile = isDev ? 'debug' : 'release';
const releaseFlag = isDev ? '' : '--release';

// Determine output path
const targetTriple = crossTarget || '';
const binaryPath = crossTarget
  ? join(TAURI_DIR, 'target', crossTarget, profile, `server${binaryExt}`)
  : join(TAURI_DIR, 'target', profile, `server${binaryExt}`);

function run(cmd, opts = {}) {
  console.log(`\x1b[36m> ${cmd}\x1b[0m`);
  execSync(cmd, { stdio: 'inherit', ...opts });
}

// ─── Step 1: Build frontend ────────────────────────────────────────────────────
console.log('\n\x1b[1m[1/2] Building frontend...\x1b[0m\n');
if (!existsSync(DIST_DIR) || !shouldRun) {
  run('npm run build', { cwd: ROOT });
} else {
  console.log('  dist/ exists, skipping frontend build (use without --run to force)');
}

// ─── Step 2: Build server binary ───────────────────────────────────────────────
console.log(`\n\x1b[1m[2/2] Building server binary (${profile})...\x1b[0m\n`);

const targetFlag = crossTarget ? `--target ${crossTarget}` : '';
const cargoCmd = `cargo build ${releaseFlag} --bin server --no-default-features --features server ${targetFlag}`.trim();
run(cargoCmd, { cwd: TAURI_DIR });

console.log(`\n\x1b[32m✓ Server binary: ${binaryPath}\x1b[0m`);
console.log(`\x1b[32m✓ Frontend dist:  ${DIST_DIR}\x1b[0m\n`);

// ─── Step 3 (optional): Run server ────────────────────────────────────────────
if (shouldRun) {
  console.log('\x1b[1mStarting server...\x1b[0m\n');

  const env = {
    ...process.env,
    SB_STATIC_DIR: DIST_DIR,
    SB_PORT: process.env.SB_PORT || '8080',
    SB_BIND: process.env.SB_BIND || '0.0.0.0',
    SB_DATA_DIR: process.env.SB_DATA_DIR || join(ROOT, 'data'),
  };

  const server = spawn(binaryPath, [], { env, stdio: 'inherit' });
  server.on('close', (code) => process.exit(code ?? 0));

  // Forward SIGINT/SIGTERM to server
  process.on('SIGINT', () => server.kill('SIGINT'));
  process.on('SIGTERM', () => server.kill('SIGTERM'));
}
