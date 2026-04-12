import type { Algorithm, Backend, Digests } from "./types";

/** Shape of the NativeHasher class exported by @hashjunkie/* platform packages. */
type NativeHasherInstance = {
  update(data: Buffer): void;
  finalize(): Record<string, string>;
};

type NativeAddon = {
  NativeHasher: new (algorithms: string[]) => NativeHasherInstance;
};

/**
 * Maps (platform, arch) to the .node filename.
 * Exported for unit testing all platform branches without modifying process globals.
 */
export function _getPlatformPackage(platform: string, arch: string): string | null {
  if (platform === "linux" && arch === "x64") return "hashjunkie.linux-x64-gnu.node";
  if (platform === "linux" && arch === "arm64") return "hashjunkie.linux-arm64-gnu.node";
  if (platform === "darwin" && arch === "x64") return "hashjunkie.darwin-x64.node";
  if (platform === "darwin" && arch === "arm64") return "hashjunkie.darwin-arm64.node";
  if (platform === "win32" && arch === "x64") return "hashjunkie.win32-x64-msvc.node";
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
 * Static require() literals are required so bun build --compile can embed .node files.
 * Non-current-platform branches are annotated c8 ignore — each CI runner covers its own.
 */
export function _defaultLoadNative(): NativeAddon | null {
  /* c8 ignore start */
  if (process.platform === "linux" && process.arch === "x64")
    return _tryRequire("./hashjunkie.linux-x64-gnu.node") as NativeAddon | null;
  if (process.platform === "linux" && process.arch === "arm64")
    return _tryRequire("./hashjunkie.linux-arm64-gnu.node") as NativeAddon | null;
  if (process.platform === "darwin" && process.arch === "x64")
    return _tryRequire("./hashjunkie.darwin-x64.node") as NativeAddon | null;
  if (process.platform === "darwin" && process.arch === "arm64")
    return _tryRequire("./hashjunkie.darwin-arm64.node") as NativeAddon | null;
  if (process.platform === "win32" && process.arch === "x64")
    return _tryRequire("./hashjunkie.win32-x64-msvc.node") as NativeAddon | null;
  return null;
  /* c8 ignore stop */
}

/** Returns null until WASM embedding is wired up in Plan 5. */
export function _defaultLoadWasm(): Backend | null {
  return null;
}

type Loaders = {
  loadNative: () => NativeAddon | null;
  loadWasm: () => Backend | null;
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
        return inst.finalize() as Digests;
      },
    };
  }

  const wasm = _loaders.loadWasm();
  if (wasm !== null) return wasm;

  throw new Error(
    "hashjunkie: no backend available — native addon failed to load and WASM is not embedded",
  );
}
