import assert from "node:assert/strict";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { checkVersions, readVersion, syncVersions } from "./version-sync.mjs";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));

async function makeFixture() {
  const root = await mkdtemp(join(tmpdir(), "hashjunkie-version-sync-"));
  const paths = [
    "crates/hashjunkie-core/Cargo.toml",
    "crates/hashjunkie-cli/Cargo.toml",
    "crates/hashjunkie-napi/Cargo.toml",
    "crates/hashjunkie-wasm/Cargo.toml",
    "npm/hashjunkie/package.json",
    "npm/hashjunkie-linux-x64-gnu/package.json",
    "npm/hashjunkie-linux-arm64-gnu/package.json",
    "npm/hashjunkie-darwin-x64/package.json",
    "npm/hashjunkie-darwin-arm64/package.json",
    "npm/hashjunkie-win32-x64-msvc/package.json",
    "npm/bun.lock",
  ];

  for (const path of paths) {
    const source = join(repoRoot, path);
    const target = join(root, path);
    await mkdir(dirname(target), { recursive: true });
    await cp(source, target, { recursive: true });
  }
  await writeFile(join(root, "VERSION"), "1.2.3\n");

  return root;
}

test("check fails when any mirrored version differs from VERSION", async () => {
  const root = await makeFixture();
  try {
    await writeFile(join(root, "VERSION"), "9.8.7\n");

    const mismatches = await checkVersions(root);

    assert(mismatches.length > 0);
    assert(mismatches.some((line) => line.includes("npm/hashjunkie/package.json")));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("sync updates all mirrored version fields from VERSION", async () => {
  const root = await makeFixture();
  try {
    await writeFile(join(root, "VERSION"), "9.8.7\n");

    await syncVersions(root);

    assert.deepEqual(await checkVersions(root), []);

    const packageJson = JSON.parse(
      await readFile(join(root, "npm/hashjunkie/package.json"), "utf8"),
    );
    assert.equal(packageJson.version, "9.8.7");

    const cargoToml = await readFile(
      join(root, "crates/hashjunkie-cli/Cargo.toml"),
      "utf8",
    );
    assert.match(cargoToml, /^version = "9\.8\.7"$/m);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("readVersion returns only the VERSION value", async () => {
  const root = await makeFixture();
  try {
    await writeFile(join(root, "VERSION"), "9.8.7\n");

    assert.equal(await readVersion(root), "9.8.7");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
