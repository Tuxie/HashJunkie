#!/usr/bin/env node
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

const defaultRepoRoot =
  process.env.HASHJUNKIE_ROOT ?? dirname(dirname(fileURLToPath(import.meta.url)));

const cargoManifests = [
  "crates/hashjunkie/Cargo.toml",
  "crates/hashjunkie-cli/Cargo.toml",
  "crates/hashjunkie-napi/Cargo.toml",
  "crates/hashjunkie-wasm/Cargo.toml",
];

const packageManifests = [
  "npm/hashjunkie/package.json",
  "npm/hashjunkie-linux-x64-gnu/package.json",
  "npm/hashjunkie-linux-arm64-gnu/package.json",
  "npm/hashjunkie-darwin-x64/package.json",
  "npm/hashjunkie-darwin-arm64/package.json",
  "npm/hashjunkie-win32-x64-msvc/package.json",
];

const lockfile = "npm/bun.lock";

function pathOf(root, path) {
  return join(root, path);
}

export async function readVersion(root = defaultRepoRoot) {
  const version = (await readFile(pathOf(root, "VERSION"), "utf8")).trim();
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error(`VERSION must contain a semver value, got ${JSON.stringify(version)}`);
  }
  return version;
}

async function readPackageVersion(root, path) {
  const data = JSON.parse(await readFile(pathOf(root, path), "utf8"));
  return data.version;
}

async function syncPackageVersion(root, path, version) {
  const contents = await readFile(pathOf(root, path), "utf8");
  await writeFile(
    pathOf(root, path),
    contents.replace(/^  "version": "[^"]+",$/m, `  "version": "${version}",`),
  );
}

async function readCargoVersion(root, path) {
  const contents = await readFile(pathOf(root, path), "utf8");
  const match = contents.match(/^version\s*=\s*"([^"]+)"$/m);
  if (!match) {
    throw new Error(`${path}: missing package version`);
  }
  return match[1];
}

async function syncCargoVersion(root, path, version) {
  const contents = await readFile(pathOf(root, path), "utf8");
  await writeFile(
    pathOf(root, path),
    contents.replace(/^version\s*=\s*"([^"]+)"$/m, `version = "${version}"`),
  );
}

async function readBunLockVersions(root) {
  const contents = await readFile(pathOf(root, lockfile), "utf8");
  const versions = [];
  const re =
    /"hashjunkie(?:-[^"]+)?": \{\n\s+"name": "@perw\/hashjunkie(?:-[^"]+)?",\n\s+"version": "([^"]+)"/g;
  let match;
  while ((match = re.exec(contents)) !== null) {
    versions.push(match[1]);
  }
  if (versions.length !== packageManifests.length) {
    throw new Error(`${lockfile}: expected ${packageManifests.length} hashjunkie workspace versions, found ${versions.length}`);
  }
  return versions;
}

async function syncBunLock(root, version) {
  const contents = await readFile(pathOf(root, lockfile), "utf8");
  await writeFile(
    pathOf(root, lockfile),
    contents.replace(
      /("hashjunkie(?:-[^"]+)?": \{\n\s+"name": "@perw\/hashjunkie(?:-[^"]+)?",\n\s+"version": ")[^"]+(")/g,
      `$1${version}$2`,
    ),
  );
}

export async function checkVersions(root = defaultRepoRoot) {
  const version = await readVersion(root);
  const out = [];
  for (const path of cargoManifests) {
    const actual = await readCargoVersion(root, path);
    if (actual !== version) {
      out.push(`${path}: ${actual} != ${version}`);
    }
  }
  for (const path of packageManifests) {
    const actual = await readPackageVersion(root, path);
    if (actual !== version) {
      out.push(`${path}: ${actual} != ${version}`);
    }
  }
  const lockVersions = await readBunLockVersions(root);
  lockVersions.forEach((actual, index) => {
    if (actual !== version) {
      out.push(`${lockfile}: workspace version ${index + 1} is ${actual} != ${version}`);
    }
  });
  return out;
}

export async function syncVersions(root = defaultRepoRoot) {
  const version = await readVersion(root);
  for (const path of cargoManifests) {
    await syncCargoVersion(root, path, version);
  }
  for (const path of packageManifests) {
    await syncPackageVersion(root, path, version);
  }
  await syncBunLock(root, version);
}

async function main() {
  const command = process.argv[2] ?? "check";
  const version = await readVersion(defaultRepoRoot);

  if (command === "print") {
    process.stdout.write(`${version}\n`);
    return;
  }

  if (command === "sync") {
    await syncVersions(defaultRepoRoot);
    return;
  }

  if (command === "check") {
    const found = await checkVersions(defaultRepoRoot);
    if (found.length > 0) {
      console.error(`Version mismatch. VERSION is ${version}:`);
      for (const mismatch of found) {
        console.error(`- ${mismatch}`);
      }
      process.exitCode = 1;
    }
    return;
  }

  console.error("Usage: node scripts/version-sync.mjs [check|sync|print]");
  process.exitCode = 2;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
