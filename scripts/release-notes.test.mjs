import assert from "node:assert/strict";
import test from "node:test";
import { extractReleaseNotes } from "./release-notes.mjs";

test("extractReleaseNotes returns the requested version section", () => {
  const notes = extractReleaseNotes(
    `# Changelog

## 1.2.3 - 2026-04-30

Important notes.

## 1.2.2 - 2026-04-29

Older notes.
`,
    "1.2.3",
  );

  assert.equal(notes, "Important notes.");
});

test("extractReleaseNotes rejects missing versions", () => {
  assert.throws(
    () => extractReleaseNotes("# Changelog\n", "1.2.3"),
    /no section for 1\.2\.3/,
  );
});
