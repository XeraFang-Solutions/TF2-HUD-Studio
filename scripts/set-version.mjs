#!/usr/bin/env node
// Writes the app version everywhere it's declared.
//
//   node scripts/set-version.mjs                 -> version from ./VERSION (stable release)
//   node scripts/set-version.mjs --edge          -> VERSION + "-edge.g<short commit hash>" (cutting edge)
//   node scripts/set-version.mjs 1.2.3           -> explicit version
//
// Cutting-edge versions keep the stable number as their base so they sort correctly, and add the commit hash
// as a semver pre-release tag. The "g" prefix (as in `git describe`) keeps the tag valid even when a hash is
// all digits with a leading zero, which semver forbids.
import { readFileSync, writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const base = readFileSync(join(root, 'VERSION'), 'utf8').trim();
const SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/;

if (!/^\d+\.\d+\.\d+$/.test(base)) {
  console.error(`VERSION must be MAJOR.MINOR.PATCH, got "${base}"`);
  process.exit(1);
}

let version = base;
if (args.includes('--edge')) {
  const sha = (process.env.GITHUB_SHA || execSync('git rev-parse HEAD', { cwd: root }).toString()).trim().slice(0, 7);
  version = `${base}-edge.g${sha}`;
} else if (args[0] && !args[0].startsWith('-')) {
  version = args[0];
}
if (!SEMVER.test(version)) {
  console.error(`"${version}" is not a valid semantic version`);
  process.exit(1);
}

const edit = (rel, fn) => {
  const p = join(root, rel);
  const before = readFileSync(p, 'utf8');
  const after = fn(before);
  if (after === before && !before.includes(version)) throw new Error(`could not set version in ${rel}`);
  writeFileSync(p, after);
  console.log(`  ${rel}`);
};

console.log(`Setting version ${version}`);
edit('apps/desktop/src-tauri/tauri.conf.json', (s) => {
  const j = JSON.parse(s);
  j.version = version;
  return JSON.stringify(j, null, 2) + '\n';
});
edit('apps/desktop/package.json', (s) => {
  const j = JSON.parse(s);
  j.version = version;
  return JSON.stringify(j, null, 2) + '\n';
});
edit('apps/desktop/src-tauri/Cargo.toml', (s) => s.replace(/^version = ".*"$/m, `version = "${version}"`));

// Expose the result to GitHub Actions steps.
if (process.env.GITHUB_OUTPUT) writeFileSync(process.env.GITHUB_OUTPUT, `version=${version}\n`, { flag: 'a' });
