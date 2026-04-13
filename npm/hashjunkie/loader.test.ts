import { afterEach, expect, test } from "bun:test";
import {
  _defaultLoadNative,
  _defaultLoadWasm,
  _getPlatformPackage,
  _setLoaders,
  _tryRequire,
  loadBackend,
} from "./loader";
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

afterEach(() => {
  // Reset loaders so test isolation is preserved
  _setLoaders({ loadNative: () => null, loadWasm: () => null });
});

// --- _getPlatformPackage ---

test("_getPlatformPackage maps all 5 supported platform/arch combos", () => {
  expect(_getPlatformPackage("linux", "x64")).toBe("hashjunkie.linux-x64-gnu.node");
  expect(_getPlatformPackage("linux", "arm64")).toBe("hashjunkie.linux-arm64-gnu.node");
  expect(_getPlatformPackage("darwin", "x64")).toBe("hashjunkie.darwin-x64.node");
  expect(_getPlatformPackage("darwin", "arm64")).toBe("hashjunkie.darwin-arm64.node");
  expect(_getPlatformPackage("win32", "x64")).toBe("hashjunkie.win32-x64-msvc.node");
});

test("_getPlatformPackage returns null for unsupported platform", () => {
  expect(_getPlatformPackage("freebsd", "x64")).toBeNull();
});

test("_getPlatformPackage returns null for unsupported arch", () => {
  expect(_getPlatformPackage("linux", "arm")).toBeNull();
});

// --- _tryRequire ---

test("_tryRequire returns null when module does not exist", () => {
  expect(_tryRequire("./definitely-does-not-exist-xyz.node")).toBeNull();
});

test("_tryRequire returns the module when it exists", () => {
  // 'path' is a Node/Bun built-in that always resolves
  const result = _tryRequire("path");
  expect(result).not.toBeNull();
});

// --- _defaultLoadNative platform branches ---
// Mocking process.platform/arch lets us exercise every branch on a single runner.
// _tryRequire returns null for missing .node files, so non-current-platform branches
// return null — the important thing is that each conditional executes.

type ProcessPlatArch = { platform: string; arch: string };

function withPlatform(platform: string, arch: string, fn: () => void): void {
  const proc = process as unknown as ProcessPlatArch;
  const orig = { platform: proc.platform, arch: proc.arch };
  proc.platform = platform;
  proc.arch = arch;
  try {
    fn();
  } finally {
    proc.platform = orig.platform;
    proc.arch = orig.arch;
  }
}

test("_defaultLoadNative linux/arm64: tries the arm64 addon path", () => {
  withPlatform("linux", "arm64", () => {
    // .node file does not exist on this runner → null, but the branch executes
    expect(_defaultLoadNative()).toBeNull();
  });
});

test("_defaultLoadNative darwin/x64: tries the darwin x64 addon path", () => {
  withPlatform("darwin", "x64", () => {
    expect(_defaultLoadNative()).toBeNull();
  });
});

test("_defaultLoadNative darwin/arm64: tries the darwin arm64 addon path", () => {
  withPlatform("darwin", "arm64", () => {
    expect(_defaultLoadNative()).toBeNull();
  });
});

test("_defaultLoadNative win32/x64: tries the win32 x64 addon path", () => {
  withPlatform("win32", "x64", () => {
    expect(_defaultLoadNative()).toBeNull();
  });
});

test("_defaultLoadNative unknown platform: falls through to null", () => {
  withPlatform("freebsd", "x64", () => {
    expect(_defaultLoadNative()).toBeNull();
  });
});

// --- _defaultLoadWasm ---

test("_defaultLoadWasm returns a working WASM backend", () => {
  const backend = _defaultLoadWasm(["sha256"]);
  expect(backend).not.toBeNull();
  if (backend === null) throw new Error("expected non-null backend");
  backend.update(new TextEncoder().encode("abc"));
  expect(backend.finalize().sha256).toBe(
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
  );
});

// --- loadBackend with native addon ---

test("loadBackend returns a backend that delegates update() and finalize() to the native instance", () => {
  const updateCalls: Buffer[] = [];
  _setLoaders({
    loadNative: () => ({
      NativeHasher: class {
        update(data: Buffer): void {
          updateCalls.push(data);
        }
        finalize(): Record<string, string> {
          return MOCK_DIGESTS;
        }
      },
    }),
    loadWasm: () => null,
  });

  const backend = loadBackend(["sha256", "blake3"]);
  const chunk = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
  backend.update(chunk);

  expect(updateCalls).toHaveLength(1);
  const firstCall = updateCalls[0];
  expect(firstCall).toBeInstanceOf(Buffer);
  expect(Array.from(firstCall ?? [])).toEqual([0xde, 0xad, 0xbe, 0xef]);
  expect(backend.finalize()).toEqual(MOCK_DIGESTS);
});

test("loadBackend converts Uint8Array to Buffer before passing to native update", () => {
  let received: Buffer | null = null;
  _setLoaders({
    loadNative: () => ({
      NativeHasher: class {
        update(data: Buffer): void {
          received = data;
        }
        finalize(): Record<string, string> {
          return MOCK_DIGESTS;
        }
      },
    }),
    loadWasm: () => null,
  });

  loadBackend(["sha256"]).update(new Uint8Array([0x01, 0x02]));
  expect(received).toBeInstanceOf(Buffer);
});

test("loadBackend forwards algorithm list to NativeHasher constructor", () => {
  let receivedAlgorithms: string[] | null = null;
  _setLoaders({
    loadNative: () => ({
      NativeHasher: class {
        constructor(algorithms: string[]) {
          receivedAlgorithms = algorithms;
        }
        update(_data: Buffer): void {}
        finalize(): Record<string, string> {
          return MOCK_DIGESTS;
        }
      },
    }),
    loadWasm: () => null,
  });

  loadBackend(["sha256", "blake3"]);
  expect(receivedAlgorithms).toEqual(["sha256", "blake3"]);
});

// --- loadBackend WASM fallback ---

test("loadBackend uses WASM backend when native returns null", () => {
  const mockWasm = {
    update(_data: Uint8Array): void {},
    finalize(): Digests {
      return MOCK_DIGESTS;
    },
  };
  _setLoaders({ loadNative: () => null, loadWasm: () => mockWasm });

  const backend = loadBackend(["sha256"]);
  expect(backend).toBe(mockWasm);
});

// --- loadBackend no backend ---

test("loadBackend throws Error when both loaders return null", () => {
  _setLoaders({ loadNative: () => null, loadWasm: () => null });
  expect(() => loadBackend(["sha256"])).toThrow("no backend available");
});
