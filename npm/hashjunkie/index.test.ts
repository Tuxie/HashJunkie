import { afterEach, beforeEach, expect, test } from "bun:test";
import {
  ALGORITHMS,
  DEFAULT_ALGORITHMS,
  HashJunkie,
  hashBuffer,
  hashFile,
  hashStream,
} from "./index";
import { _setLoaders } from "./loader";
import type { Digests } from "./types";

const MOCK_DIGESTS: Digests = {
  aich: "a9",
  blake3: "aa",
  btv2: "99",
  cidv0: "Qm...",
  cidv1: "bafkrei...",
  crc32: "bb",
  dropbox: "cc",
  ed2k: "dd",
  hidrive: "ee",
  mailru: "ff",
  md5: "00",
  quickxor: "11",
  sha1: "22",
  sha256: "33",
  sha512: "44",
  tiger: "55",
  whirlpool: "66",
  xxh128: "77",
  xxh3: "88",
};

function pickDigests(algorithms: readonly string[]): Digests {
  return Object.fromEntries(
    algorithms.map((algorithm) => [algorithm, MOCK_DIGESTS[algorithm as keyof Digests]]),
  ) as Digests;
}

// Install a working mock backend before every test so constructing HashJunkie succeeds.
beforeEach(() => {
  _setLoaders({
    loadNative: () => ({
      NativeHasher: class {
        readonly algorithms: string[];

        constructor(algorithms: string[]) {
          this.algorithms = algorithms;
        }

        update(_data: Buffer): void {}
        finalize(): Record<string, string> {
          return pickDigests(this.algorithms);
        }
      },
      hashFile: async (_path: string, algorithms: string[]) => pickDigests(algorithms),
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
  expect(ALGORITHMS).toHaveLength(19);
  expect(ALGORITHMS).toContain("aich");
  expect(ALGORITHMS).toContain("sha256");
  expect(ALGORITHMS).toContain("btv2");
  expect(ALGORITHMS).toContain("cidv0");
  expect(ALGORITHMS).toContain("cidv1");
  expect(ALGORITHMS).toContain("ed2k");
  expect(ALGORITHMS).toContain("tiger");
});

test("DEFAULT_ALGORITHMS is re-exported from index", () => {
  expect(DEFAULT_ALGORITHMS).toHaveLength(18);
  expect(DEFAULT_ALGORITHMS).toContain("aich");
  expect(DEFAULT_ALGORITHMS).toContain("btv2");
  expect(DEFAULT_ALGORITHMS).toContain("ed2k");
  expect(DEFAULT_ALGORITHMS).toContain("tiger");
  expect(DEFAULT_ALGORITHMS).not.toContain("whirlpool");
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
  const algorithms = ["sha256", "blake3"] as const;
  const hj = new HashJunkie([...algorithms]);
  await pipe(hj, [new Uint8Array([0xca, 0xfe])]);
  expect(await hj.digests).toEqual(pickDigests(algorithms));
});

test("HashJunkie with no constructor arg uses default algorithms", async () => {
  const hj = new HashJunkie();
  await pipe(hj, []);
  const digests = await hj.digests;
  expect(Object.keys(digests).sort()).toEqual([...DEFAULT_ALGORITHMS].sort());
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

  let caught: unknown;
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

// --- regression: README "write-without-piping" pattern must not hang ---

// The readable side is never drained by the caller; earlier versions configured
// the default readable HWM of 0, which back-pressured the transformer on the
// first enqueue and hung `writer.write()` forever. The fix is a high readable
// HWM so enqueues never block when nobody is reading.
test("HashJunkie supports write-close-digests without anyone reading readable", async () => {
  const hj = new HashJunkie(["sha256"]);
  const w = hj.writable.getWriter();
  await w.write(new TextEncoder().encode("hello"));
  await w.close();
  expect(await hj.digests).toEqual(pickDigests(["sha256"]));
});

test("HashJunkie handles many writes without a reader (no back-pressure deadlock)", async () => {
  const hj = new HashJunkie(["sha256"]);
  const w = hj.writable.getWriter();
  for (let i = 0; i < 100; i++) await w.write(new Uint8Array([i]));
  await w.close();
  expect(await hj.digests).toEqual(pickDigests(["sha256"]));
});

// --- hashBuffer helper ---

test("hashBuffer resolves digests for a Uint8Array without stream boilerplate", async () => {
  const digests = await hashBuffer(new TextEncoder().encode("hello"), ["sha256"]);
  expect(digests).toEqual(pickDigests(["sha256"]));
});

test("hashBuffer with no algorithm argument uses default algorithms", async () => {
  const digests = await hashBuffer(new Uint8Array([1, 2, 3]));
  expect(Object.keys(digests).sort()).toEqual([...DEFAULT_ALGORITHMS].sort());
});

test("hashBuffer handles an empty input", async () => {
  const digests = await hashBuffer(new Uint8Array(), ["sha256"]);
  expect(digests).toEqual(pickDigests(["sha256"]));
});

// --- hashStream helper ---

test("hashStream drains a ReadableStream and resolves digests", async () => {
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(new Uint8Array([1, 2]));
      controller.enqueue(new Uint8Array([3, 4]));
      controller.close();
    },
  });
  const digests = await hashStream(stream, ["sha256"]);
  expect(digests).toEqual(pickDigests(["sha256"]));
});

test("hashStream with no algorithm argument uses default algorithms", async () => {
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      controller.close();
    },
  });
  const digests = await hashStream(stream);
  expect(Object.keys(digests).sort()).toEqual([...DEFAULT_ALGORITHMS].sort());
});

// --- hashFile helper ---

test("hashFile resolves digests for a path without stream boilerplate", async () => {
  const digests = await hashFile("/tmp/example.raw", ["blake3"]);
  expect(digests).toEqual(pickDigests(["blake3"]));
});

test("hashFile with no algorithm argument uses default algorithms", async () => {
  const digests = await hashFile("/tmp/example.raw");
  expect(Object.keys(digests).sort()).toEqual([...DEFAULT_ALGORITHMS].sort());
});
