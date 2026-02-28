#!/usr/bin/env node

const fs = require('fs');
const { execSync } = require('child_process');

const bumpType = process.argv[2] || 'patch';

if (!['patch', 'minor', 'major'].includes(bumpType)) {
  console.error('Usage: node bump-version.js [patch|minor|major]');
  process.exit(1);
}

console.log(`Bumping ${bumpType} version...`);

// Bump package.json
execSync(`npm version ${bumpType} --no-git-tag-version`, { stdio: 'inherit' });

const newVersion = JSON.parse(fs.readFileSync('package.json', 'utf8')).version;
console.log(`New version: ${newVersion}`);

// Update Cargo.toml
let cargoToml = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
cargoToml = cargoToml.replace(/^version = ".*"/m, `version = "${newVersion}"`);
fs.writeFileSync('src-tauri/Cargo.toml', cargoToml);
console.log('✓ Updated Cargo.toml');

// Update tauri.conf.json
const tauriConf = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
tauriConf.version = newVersion;
fs.writeFileSync('src-tauri/tauri.conf.json', JSON.stringify(tauriConf, null, 2) + '\n');
console.log('✓ Updated tauri.conf.json');

// Update Cargo.lock
execSync('cd src-tauri && cargo update -p stockenboard', { stdio: 'inherit' });
console.log('✓ Updated Cargo.lock');

console.log('\nVersion bumped to', newVersion);
console.log('\nNext steps:');
console.log('  git add .');
console.log(`  git commit -m "chore: bump version to ${newVersion}"`);
console.log('  git push');
console.log('\nGitHub Actions will automatically create a release!');
