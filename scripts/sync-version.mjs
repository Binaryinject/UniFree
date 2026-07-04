import { readFileSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

// Read version from version.json (single source of truth)
const { version } = JSON.parse(readFileSync(resolve(root, 'version.json'), 'utf-8'));

// Update package.json
const pkgPath = resolve(root, 'package.json');
const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'));
pkg.version = version;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
console.log(`✓ package.json → ${version}`);

// Update package-lock.json
const lockPath = resolve(root, 'package-lock.json');
const lock = JSON.parse(readFileSync(lockPath, 'utf-8'));
lock.version = version;
if (lock.packages?.['']) lock.packages[''].version = version;
writeFileSync(lockPath, JSON.stringify(lock, null, 2) + '\n');
console.log(`✓ package-lock.json → ${version}`);

// Update Cargo.toml
const cargoPath = resolve(root, 'src-tauri', 'Cargo.toml');
let cargo = readFileSync(cargoPath, 'utf-8');
cargo = cargo.replace(/^version\s*=\s*".*"/m, `version = "${version}"`);
writeFileSync(cargoPath, cargo);
console.log(`✓ Cargo.toml → ${version}`);

// Update tauri.conf.json
const tauriPath = resolve(root, 'src-tauri', 'tauri.conf.json');
const tauri = JSON.parse(readFileSync(tauriPath, 'utf-8'));
tauri.version = version;
if (tauri.app?.windows?.[0]?.title) {
  tauri.app.windows[0].title = `UniFree v${version}`;
}
writeFileSync(tauriPath, JSON.stringify(tauri, null, 2) + '\n');
console.log(`✓ tauri.conf.json → ${version}`);

console.log(`\nVersion synced: ${version}`);
