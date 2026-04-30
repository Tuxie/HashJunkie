#!/usr/bin/env node
import { readFile } from "node:fs/promises";

export function extractReleaseNotes(changelog, version) {
  const escaped = version.replaceAll(".", "\\.");
  const header = new RegExp(`^## ${escaped}(?:\\s+-\\s+.*)?$`, "m");
  const match = changelog.match(header);
  if (!match || match.index === undefined) {
    throw new Error(`CHANGELOG.md has no section for ${version}`);
  }

  const start = match.index + match[0].length;
  const rest = changelog.slice(start);
  const next = rest.search(/^## /m);
  const body = (next === -1 ? rest : rest.slice(0, next)).trim();
  if (body.length === 0) {
    throw new Error(`CHANGELOG.md section for ${version} is empty`);
  }
  return body;
}

async function main() {
  const version = process.argv[2];
  if (!version) {
    console.error("Usage: node scripts/release-notes.mjs VERSION");
    process.exitCode = 2;
    return;
  }
  const changelog = await readFile("CHANGELOG.md", "utf8");
  console.log(extractReleaseNotes(changelog, version));
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
