#!/usr/bin/env node
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

const platforms = [
  { key: "darwin-arm64", block: "on_arm" },
  { key: "darwin-x64", block: "on_intel" },
  { key: "linux-arm64-gnu", block: "on_arm" },
  { key: "linux-x64-gnu", block: "on_intel" },
];

async function sha256(path) {
  return createHash("sha256").update(await readFile(path)).digest("hex");
}

async function archiveHashes(artifacts, version) {
  const out = new Map();
  for (const { key } of platforms) {
    const archive = join(artifacts, `hashjunkie-cli-${version}-${key}.tar.xz`);
    out.set(key, await sha256(archive));
  }
  return out;
}

function replacePlatformSha(formula, platform, sha) {
  const url = `hashjunkie-cli-#{release_version}-${platform}.tar.xz`;
  const start = formula.indexOf(url);
  if (start === -1) {
    throw new Error(`Formula is missing URL for ${platform}`);
  }
  const shaStart = formula.indexOf('sha256 "', start);
  if (shaStart === -1) {
    throw new Error(`Formula is missing sha256 for ${platform}`);
  }
  const shaEnd = formula.indexOf('"', shaStart + 'sha256 "'.length);
  return `${formula.slice(0, shaStart)}sha256 "${sha}"${formula.slice(shaEnd + 1)}`;
}

export async function updateFormula({ formula, artifacts, version }) {
  const hashes = await archiveHashes(artifacts, version);
  let contents = await readFile(formula, "utf8");
  contents = contents.replace(
    /^  release_version = "[^"]+"$/m,
    `  release_version = "${version}"\n  version release_version`,
  );
  contents = contents.replace(
    /^  release_version = "([^"]+)"\n  version release_version\n  version release_version$/m,
    `  release_version = "$1"\n  version release_version`,
  );

  for (const [platform, hash] of hashes) {
    contents = replacePlatformSha(contents, platform, hash);
  }

  await mkdir(dirname(formula), { recursive: true });
  await writeFile(formula, contents);
}

function parseArgs(argv) {
  const args = new Map();
  for (let i = 0; i < argv.length; i += 2) {
    args.set(argv[i], argv[i + 1]);
  }
  return {
    formula: args.get("--formula"),
    artifacts: args.get("--artifacts-dir"),
    version: args.get("--version"),
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.formula || !args.artifacts || !args.version) {
    console.error(
      "Usage: node scripts/update-homebrew-formula.mjs --formula PATH --artifacts-dir DIR --version VERSION",
    );
    process.exitCode = 2;
    return;
  }
  await updateFormula(args);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
