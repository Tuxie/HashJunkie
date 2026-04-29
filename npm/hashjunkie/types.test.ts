import { expect, test } from "bun:test";
import type { Algorithm } from "./types";
import { ALGORITHMS, parseAlgorithms } from "./types";

test("ALGORITHMS contains exactly 15 algorithms", () => {
  expect(ALGORITHMS).toHaveLength(15);
});

test("ALGORITHMS includes all required algorithm names", () => {
  const required: Algorithm[] = [
    "blake3",
    "cidv0",
    "cidv1",
    "crc32",
    "dropbox",
    "hidrive",
    "mailru",
    "md5",
    "quickxor",
    "sha1",
    "sha256",
    "sha512",
    "whirlpool",
    "xxh128",
    "xxh3",
  ];
  for (const name of required) {
    expect(ALGORITHMS).toContain(name);
  }
});

test("parseAlgorithms() with no argument returns all 15 algorithms", () => {
  const result = parseAlgorithms();
  expect(result).toHaveLength(15);
  expect(result).toEqual([...ALGORITHMS]);
});

test("parseAlgorithms() returns a mutable copy (not the const array)", () => {
  const result = parseAlgorithms();
  result.push("sha256" as never); // should not throw
  expect(ALGORITHMS).toHaveLength(15); // original unchanged
});

test("parseAlgorithms() with a valid subset returns that subset", () => {
  expect(parseAlgorithms(["sha256", "blake3"])).toEqual(["sha256", "blake3"]);
});

test("parseAlgorithms() with empty array throws TypeError", () => {
  expect(() => parseAlgorithms([])).toThrow(TypeError);
  expect(() => parseAlgorithms([])).toThrow("must not be empty");
});

test("parseAlgorithms() with unknown algorithm name throws TypeError", () => {
  expect(() => parseAlgorithms(["sha256", "md99"])).toThrow(TypeError);
  expect(() => parseAlgorithms(["sha256", "md99"])).toThrow('"md99"');
});

test("parseAlgorithms() with single unknown algorithm throws TypeError", () => {
  expect(() => parseAlgorithms(["bogus"])).toThrow(TypeError);
});
