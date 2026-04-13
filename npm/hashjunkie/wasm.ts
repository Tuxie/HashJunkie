import { WasmHasher, initSync } from "./hashjunkie_wasm.js";
import type { Algorithm, Backend, Digests } from "./types";
import { WASM_BLOB } from "./wasm_blob";

let initialized = false;

function ensureInit(): void {
  if (initialized) return;
  const bytes = Uint8Array.from(atob(WASM_BLOB), (c) => c.charCodeAt(0));
  // initSync with { module: bytes } wraps bytes in WebAssembly.Module synchronously.
  // No filesystem access — bytes come from the inline base64 blob.
  initSync({ module: bytes });
  initialized = true;
}

export function makeWasmBackend(algorithms: Algorithm[]): Backend {
  ensureInit();
  const hasher = new WasmHasher(algorithms);
  return {
    update(data: Uint8Array): void {
      hasher.update(data);
    },
    finalize(): Digests {
      // Trust assertion: WasmHasher.finalize() returns exactly the requested
      // algorithm keys mapping to lowercase hex strings — same guarantee as
      // the Rust MultiHasher it wraps.
      return hasher.finalize() as Digests;
    },
  };
}
