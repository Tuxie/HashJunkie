export const ALGORITHMS = [
  "blake3",
  "cidv0",
  "cidv1",
  "crc32",
  "dropbox",
  "ed2k",
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
] as const;

export type Algorithm = (typeof ALGORITHMS)[number];

export const DEFAULT_ALGORITHMS = ALGORITHMS.filter((algorithm) => algorithm !== "whirlpool");

export type Digests = Record<Algorithm, string>;

const ALGORITHM_SET = new Set<string>(ALGORITHMS);

/** Backend interface implemented by both the native addon wrapper and the WASM wrapper. */
export type Backend = {
  update(data: Uint8Array): void;
  finalize(): Digests;
};

/** Backend interface for native file hashing. */
export type FileBackend = {
  hashFile(path: string, algorithms: Algorithm[]): Promise<Digests>;
};

/**
 * Validates and returns the algorithm list.
 * Returns the default algorithms when called with no argument. Whirlpool is
 * supported but opt-in because it is much slower than the other hashes.
 * Throws TypeError for unknown algorithm names or an empty array.
 */
export function parseAlgorithms(algorithms?: readonly string[]): Algorithm[] {
  if (algorithms === undefined) return [...DEFAULT_ALGORITHMS];
  if (algorithms.length === 0) {
    throw new TypeError(
      "algorithms must not be empty; omit the argument to use default algorithms",
    );
  }
  for (const alg of algorithms) {
    if (!ALGORITHM_SET.has(alg)) {
      throw new TypeError(`unknown algorithm: "${alg}"`);
    }
  }
  return algorithms as Algorithm[];
}
