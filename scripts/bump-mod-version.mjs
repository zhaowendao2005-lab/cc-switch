#!/usr/bin/env node
// Bump the app version to the upstream stable tag before creating a `mod-v*` release tag.
//
// Usage:
//   node scripts/bump-mod-version.mjs v4.0.28
//   node scripts/bump-mod-version.mjs 4.0.28

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const raw = process.argv[2];
if (!raw) {
  console.error("Usage: node scripts/bump-mod-version.mjs <tag|version>");
  process.exit(1);
}

const version = raw
  .replace(/^mod-v/, "")
  .replace(/^v/, "")
  .trim();

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(`Invalid version: ${raw} (expected vX.Y.Z or mod-vX.Y.Z)`);
  process.exit(1);
}

const changed = [];

function writeIfChanged(relPath, content) {
  const path = join(root, relPath);
  if (!existsSync(path)) {
    console.error(`Missing file: ${relPath}`);
    process.exit(1);
  }
  const before = readFileSync(path, "utf8");
  if (before !== content) {
    writeFileSync(path, content);
    changed.push(relPath);
  }
}

bumpTopLevelVersion("package.json");
bumpTopLevelVersion("src-tauri/tauri.conf.json");

function bumpTopLevelVersion(relPath) {
  const path = join(root, relPath);
  const before = readFileSync(path, "utf8");
  const pattern = /^(\s*"version"\s*:\s*")[^"]+(")/m;
  if (!pattern.test(before)) {
    console.error(`Could not find top-level version in ${relPath}`);
    process.exit(1);
  }
  writeIfChanged(relPath, before.replace(pattern, `$1${version}$2`));
}

// Cargo.toml: only the [package] version (the first `version = "..."`).
{
  const relPath = "src-tauri/Cargo.toml";
  const path = join(root, relPath);
  const before = readFileSync(path, "utf8");
  const lines = before.split("\n");
  let inPackage = false;
  let done = false;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (/^\[package\]\s*$/.test(line)) {
      inPackage = true;
      continue;
    }
    if (/^\[/.test(line)) {
      inPackage = false;
    }
    if (inPackage && !done && /^version\s*=\s*"/.test(line)) {
      lines[i] = `version = "${version}"`;
      done = true;
      break;
    }
  }
  if (!done) {
    console.error(`Could not find [package] version in ${relPath}`);
    process.exit(1);
  }
  writeIfChanged(relPath, lines.join("\n"));
}

// Cargo.lock: the cc-switch package entry.
{
  const relPath = "src-tauri/Cargo.lock";
  const path = join(root, relPath);
  const before = readFileSync(path, "utf8");
  const lines = before.split("\n");
  let inPackage = false;
  let done = false;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (line === "[[package]]") {
      inPackage = true;
      continue;
    }
    if (inPackage && /^name = "cc-switch"$/.test(line)) {
      for (let j = i + 1; j < lines.length && j < i + 5; j += 1) {
        if (/^version = "/.test(lines[j])) {
          lines[j] = `version = "${version}"`;
          done = true;
          break;
        }
      }
      break;
    }
  }
  if (!done) {
    console.error(`Could not find cc-switch entry in ${relPath}`);
    process.exit(1);
  }
  writeIfChanged(relPath, lines.join("\n"));
}

// What's-new summary is required by the release workflow.
{
  const dir = join(root, "src/whats-new");
  mkdirSync(dir, { recursive: true });
  const relPath = `src/whats-new/${version}.json`;
  if (!existsSync(join(root, relPath))) {
    writeFileSync(
      join(root, relPath),
      `${JSON.stringify({ version, items: [] }, null, 2)}\n`,
    );
    changed.push(relPath);
  }
}

if (changed.length === 0) {
  console.log(`Version already ${version}; nothing to change.`);
} else {
  console.log(`Version set to ${version}:`);
  for (const file of changed) console.log(`  - ${file}`);
}
