import type { Algorithm, Backend, Digests } from "./types";
import { makeWasmBackend } from "./wasm";

/** Shape of the NativeHasher class exported by @perw/hashjunkie-* platform packages. */
type NativeHasherInstance = {
  update(data: Buffer): void;
  finalize(): Record<string, string>;
};

type NativeAddon = {
  NativeHasher: new (algorithms: string[]) => NativeHasherInstance;
};

/**
 * Maps (platform, arch) to the npm platform package name.
 * Exported for unit testing all platform branches without modifying process globals.
 */
export function _getPlatformPackage(platform: string, arch: string): string | null {
  if (platform === "linux" && arch === "x64") return "@perw/hashjunkie-linux-x64-gnu";
  if (platform === "linux" && arch === "arm64") return "@perw/hashjunkie-linux-arm64-gnu";
  if (platform === "darwin" && arch === "x64") return "@perw/hashjunkie-darwin-x64";
  if (platform === "darwin" && arch === "arm64") return "@perw/hashjunkie-darwin-arm64";
  if (platform === "win32" && arch === "x64") return "@perw/hashjunkie-win32-x64-msvc";
  return null;
}

/**
 * Attempts to require() a module path. Returns null if the module is not found or
 * cannot be loaded. Exported so both success and failure branches are unit-testable.
 */
// biome-ignore lint/suspicious/noExplicitAny: returns unknown module shape
export function _tryRequire(path: string): any {
  try {
    return require(path);
  } catch {
    return null;
  }
}

/**
 * Loads the native addon for the current platform.
 * Uses static npm package name literals so bun build --compile can embed the .node file.
 * Non-current-platform branches are not coverable per runner — each CI runner covers its own.
 */
export function _defaultLoadNative(): NativeAddon | null {
  if (process.platform === "linux" && process.arch === "x64")
    // Trust assertion: if the package loads, napi-rs guarantees this shape
    return _tryRequire("@perw/hashjunkie-linux-x64-gnu") as NativeAddon | null;
  if (process.platform === "linux" && process.arch === "arm64")
    return _tryRequire("@perw/hashjunkie-linux-arm64-gnu") as NativeAddon | null;
  if (process.platform === "darwin" && process.arch === "x64")
    return _tryRequire("@perw/hashjunkie-darwin-x64") as NativeAddon | null;
  if (process.platform === "darwin" && process.arch === "arm64")
    return _tryRequire("@perw/hashjunkie-darwin-arm64") as NativeAddon | null;
  if (process.platform === "win32" && process.arch === "x64")
    return _tryRequire("@perw/hashjunkie-win32-x64-msvc") as NativeAddon | null;
  return null;
}

/** Returns a WASM backend for the given algorithms. Throws if WASM initialisation fails. */
export function _defaultLoadWasm(algorithms: Algorithm[]): Backend | null {
  return makeWasmBackend(algorithms);
}

type Loaders = {
  loadNative: () => NativeAddon | null;
  loadWasm: (algorithms: Algorithm[]) => Backend | null;
};

let _loaders: Loaders = {
  loadNative: _defaultLoadNative,
  loadWasm: _defaultLoadWasm,
};

/** Override loaders in tests. Always restore via afterEach. */
export function _setLoaders(l: Loaders): void {
  _loaders = l;
}

/**
 * Returns an active Backend for the given algorithm set.
 * Tries the native addon first; falls back to WASM; throws if neither is available.
 */
export function loadBackend(algorithms: Algorithm[]): Backend {
  const addon = _loaders.loadNative();
  if (addon !== null) {
    const inst = new addon.NativeHasher(algorithms);
    return {
      update(data: Uint8Array): void {
        inst.update(Buffer.from(data));
      },
      finalize(): Digests {
        // Trust assertion: the Rust layer always returns exactly the 13 Algorithm keys
        return inst.finalize() as Digests;
      },
    };
  }

  const wasm = _loaders.loadWasm(algorithms);
  if (wasm !== null) return wasm;

  throw new Error(
    "hashjunkie: no backend available — native addon failed to load and WASM initialisation failed",
  );
}
