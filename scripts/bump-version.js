#!/usr/bin/env node
/**
 * Sincroniza a versão nos três arquivos que a Tauri exige estar em sincronia:
 *   - package.json
 *   - src-tauri/Cargo.toml
 *   - src-tauri/tauri.conf.json
 *
 * Uso:
 *   node scripts/bump-version.js 0.2.0
 *
 * Após rodar, commit + tag:
 *   git add -A && git commit -m "chore: bump version to v0.2.0"
 *   git tag v0.2.0 && git push && git push --tags
 */

import { readFileSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const newVersion = process.argv[2];
if (!newVersion || !/^\d+\.\d+\.\d+$/.test(newVersion)) {
  console.error('Uso: node scripts/bump-version.js <major.minor.patch>');
  console.error('Exemplo: node scripts/bump-version.js 0.2.0');
  process.exit(1);
}

// ── package.json ──────────────────────────────────────────────────────────────
const pkgPath = resolve(root, 'package.json');
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
const oldPkgVersion = pkg.version;
pkg.version = newVersion;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
console.log(`package.json:          ${oldPkgVersion} → ${newVersion}`);

// ── src-tauri/tauri.conf.json ─────────────────────────────────────────────────
const tauriConfPath = resolve(root, 'src-tauri/tauri.conf.json');
const tauriConf = JSON.parse(readFileSync(tauriConfPath, 'utf8'));
const oldTauriVersion = tauriConf.version;
tauriConf.version = newVersion;
writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n');
console.log(`tauri.conf.json:       ${oldTauriVersion} → ${newVersion}`);

// ── src-tauri/Cargo.toml ──────────────────────────────────────────────────────
const cargoPath = resolve(root, 'src-tauri/Cargo.toml');
let cargo = readFileSync(cargoPath, 'utf8');
const cargoVersionMatch = cargo.match(/^version\s*=\s*"([^"]+)"/m);
const oldCargoVersion = cargoVersionMatch ? cargoVersionMatch[1] : '?';
cargo = cargo.replace(/^(version\s*=\s*)"[^"]+"(\s*$)/m, `$1"${newVersion}"$2`);
writeFileSync(cargoPath, cargo);
console.log(`Cargo.toml:            ${oldCargoVersion} → ${newVersion}`);

console.log(`\nPróximos passos:`);
console.log(`  git add -A`);
console.log(`  git commit -m "chore: bump version to v${newVersion}"`);
console.log(`  git tag v${newVersion}`);
console.log(`  git push && git push --tags`);
console.log(`\nO GitHub Actions vai criar a release automaticamente.`);
