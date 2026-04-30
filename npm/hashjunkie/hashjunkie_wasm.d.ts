/* tslint:disable */
/* eslint-disable */

/**
 * A streaming multi-algorithm hasher exposed to JavaScript via WebAssembly.
 *
 * ```js
 * const h = new WasmHasher(['sha256', 'blake3']);
 * h.update(new Uint8Array([104, 101, 108, 108, 111]));
 * const digests = h.finalize(); // { sha256: '...', blake3: '...' }
 * ```
 */
export class WasmHasher {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Finalize all hashers and return a plain JS object mapping algorithm
     * name to digest string. Throws if called again after the
     * first `finalize()`.
     */
    finalize(): any;
    /**
     * Create a new hasher. Pass an array of algorithm name strings (e.g.
     * `['sha256', 'blake3']`) or omit / pass `null` / `undefined` to hash
     * with the default algorithms. Whirlpool is supported but opt-in because
     * it is much slower than the other hashes. Throws if any name is unrecognised or if an
     * empty array is passed.
     */
    constructor(algorithms: any);
    /**
     * Feed a chunk of data into all active hashers. Throws if called after `finalize()`.
     */
    update(data: Uint8Array): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmhasher_free: (a: number, b: number) => void;
    readonly wasmhasher_finalize: (a: number, b: number) => void;
    readonly wasmhasher_new: (a: number, b: number) => void;
    readonly wasmhasher_update: (a: number, b: number, c: number, d: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
