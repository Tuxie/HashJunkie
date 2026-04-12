import { afterEach, beforeEach, expect, test } from "bun:test";
import { ALGORITHMS, HashJunkie } from "./index";
import { _setLoaders } from "./loader";
import type { Digests } from "./types";

const MOCK_DIGESTS: Digests = {
  blake3: "aa",
  crc32: "bb",
  dropbox: "cc",
  hidrive: "dd",
  mailru: "ee",
  md5: "ff",
  quickxor: "00",
  sha1: "11",
  sha256: "22",
  sha512: "33",
  whirlpool: "44",
  xxh128: "55",
  xxh3: "66",
};

// Install a working mock backend before every test so constructing HashJunkie succeeds.
beforeEach(() => {
  _setLoaders({
    loadNative: () => ({
      NativeHasher: class {
        update(_data: Buffer): void {}
        finalize(): Record<string, string> {
          return MOCK_DIGESTS;
        }
      },
    }),
    loadWasm: () => null,
  });
});

afterEach(() => {
  _setLoaders({ loadNative: () => null, loadWasm: () => null });
});

/** Writes chunks through hj, collects all output chunks, closes the stream. */
async function pipe(hj: HashJunkie, inputs: Uint8Array[]): Promise<Uint8Array[]> {
  const out: Uint8Array[] = [];
  const reader = hj.readable.getReader();
  const writer = hj.writable.getWriter();
  await Promise.all([
    (async () => {
      for (const chunk of inputs) await writer.write(chunk);
      await writer.close();
    })(),
    (async () => {
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        out.push(value);
      }
    })(),
  ]);
  return out;
}

// --- re-exports ---

test("ALGORITHMS is re-exported from index", () => {
  expect(ALGORITHMS).toHaveLength(13);
  expect(ALGORITHMS).toContain("sha256");
});

// --- passthrough ---

test("HashJunkie passes all chunks through unchanged", async () => {
  const hj = new HashJunkie(["sha256"]);
  const chunks = [new Uint8Array([1, 2, 3]), new Uint8Array([4, 5])];
  const result = await pipe(hj, chunks);
  expect(result).toHaveLength(2);
  expect(result[0]).toEqual(new Uint8Array([1, 2, 3]));
  expect(result[1]).toEqual(new Uint8Array([4, 5]));
});

test("HashJunkie works with zero chunks (empty stream)", async () => {
  const hj = new HashJunkie(["sha256"]);
  const result = await pipe(hj, []);
  expect(result).toHaveLength(0);
});

// --- digests ---

test("HashJunkie.digests resolves with backend digests after stream close", async () => {
  const hj = new HashJunkie(["sha256", "blake3"]);
  await pipe(hj, [new Uint8Array([0xca, 0xfe])]);
  expect(await hj.digests).toEqual(MOCK_DIGESTS);
});

test("HashJunkie with no constructor arg uses all algorithms", async () => {
  const hj = new HashJunkie();
  await pipe(hj, []);
  const digests = await hj.digests;
  expect(Object.keys(digests).sort()).toEqual([...ALGORITHMS].sort());
});

// --- constructor validation ---

test("HashJunkie constructor throws TypeError synchronously for unknown algorithm", () => {
  // `as never` bypasses the type system to pass invalid runtime values without triggering noExplicitAny
  expect(() => new HashJunkie(["sha256", "not-real"] as never)).toThrow(TypeError);
  expect(() => new HashJunkie(["sha256", "not-real"] as never)).toThrow('"not-real"');
});

test("HashJunkie constructor throws TypeError synchronously for empty array", () => {
  expect(() => new HashJunkie([])).toThrow(TypeError);
  expect(() => new HashJunkie([])).toThrow("must not be empty");
});

test("HashJunkie constructor throws Error when no backend is available", () => {
  _setLoaders({ loadNative: () => null, loadWasm: () => null });
  expect(() => new HashJunkie(["sha256"])).toThrow("no backend available");
});

// --- digests rejection ---

test("HashJunkie.digests rejects when the writable stream is aborted", async () => {
  const hj = new HashJunkie(["sha256"]);
  const abortError = new Error("upstream abort");

  // abort() closes the writable side with an error; this.writable.closed rejects → rejectDigests fires
  await hj.writable.abort(abortError);

  let caught: unknown = undefined;
  try {
    await hj.digests;
  } catch (e) {
    caught = e;
  }
  expect(caught).toBe(abortError);
});

test("HashJunkie.digests rejects with undefined when writable is aborted with no reason", async () => {
  const hj = new HashJunkie(["sha256"]);
  await hj.writable.abort(); // no reason argument

  let caught: unknown = "NOT_SET";
  try {
    await hj.digests;
  } catch (e) {
    caught = e;
  }
  expect(caught).toBeUndefined();
});
