import { copyFileSync, mkdirSync, readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';

const { version } = JSON.parse(readFileSync('package.json', 'utf8'));
const platform = `${process.platform === 'darwin' ? 'macos' : process.platform === 'win32' ? 'windows' : process.platform}-${process.arch}`;
if (!['macos-arm64', 'macos-x64', 'windows-x64'].includes(platform)) {
  throw new Error(`Unsupported release platform: ${platform}`);
}
if (process.env.RELEASE_PLATFORM && process.env.RELEASE_PLATFORM !== platform) {
  throw new Error(`Runner architecture ${platform} does not match ${process.env.RELEASE_PLATFORM}.`);
}
const windows = process.platform === 'win32';
const extension = windows ? '.exe' : '.dmg';
const directory = path.join('src-tauri', 'target', 'release', 'bundle', windows ? 'nsis' : 'dmg');
const matches = readdirSync(directory).filter(name => name.includes(`_${version}_`) && name.endsWith(extension));
if (matches.length !== 1) throw new Error(`Expected exactly one ${platform} installer for ${version}, found: ${matches.join(', ')}`);
const source = path.join(directory, matches[0]);
if (!statSync(source).size) throw new Error(`Empty installer: ${source}`);
mkdirSync('release-artifacts', { recursive: true });
const destination = path.join('release-artifacts', `Ker-Finance_${version}_${platform}${windows ? '-setup' : ''}${extension}`);
copyFileSync(source, destination);
console.log(`Release installer ready: ${destination}`);
