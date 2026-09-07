import { readFileSync, appendFileSync } from 'node:fs';

const readJson = (file) => JSON.parse(readFileSync(file, 'utf8'));
const pkg = readJson('package.json');
const npmLock = readJson('package-lock.json');
const cargo = readFileSync('src-tauri/Cargo.toml', 'utf8');
const cargoLock = readFileSync('src-tauri/Cargo.lock', 'utf8');
const versions = {
  'package.json': pkg.version,
  'package-lock.json': npmLock.version,
  'package-lock.json root package': npmLock.packages[''].version,
  'tauri.conf.json': readJson('src-tauri/tauri.conf.json').version,
  'Cargo.toml': cargo.match(/\[package\][\s\S]*?^version = "([^"]+)"/m)?.[1],
  'Cargo.lock': cargoLock.match(/\[\[package\]\]\nname = "multiservices-senegal"\nversion = "([^"]+)"/)?.[1],
};
if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(pkg.version)) {
  throw new Error('Use a stable release version such as 0.3.0.');
}
for (const [file, version] of Object.entries(versions)) {
  if (version !== pkg.version) throw new Error(`${file}: ${version} does not match ${pkg.version}.`);
}
const tag = process.argv[2] ?? `v${pkg.version}`;
if (tag !== `v${pkg.version}`) throw new Error(`Tag ${tag} does not match version ${pkg.version}.`);
if (process.env.GITHUB_OUTPUT) {
  appendFileSync(process.env.GITHUB_OUTPUT, `version=${pkg.version}\ntag=${tag}\n`);
}
console.log(`Release versions verified: ${tag}`);
