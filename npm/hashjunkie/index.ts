import { loadBackend } from "./loader";
import { parseAlgorithms } from "./types";
import type { Algorithm, Digests } from "./types";

export { ALGORITHMS } from "./types";
export type { Algorithm, Digests };

export class HashJunkie extends TransformStream<Uint8Array, Uint8Array> {
  /** Resolves with all requested digests when the stream closes cleanly. Rejects on error. */
  readonly digests: Promise<Digests>;

  constructor(algorithms?: Algorithm[]) {
    // Validate algorithm list synchronously before any IO — fast fail with a clear TypeError.
    const algs = parseAlgorithms(algorithms);

    let resolveDigests!: (d: Digests) => void;
    let rejectDigests!: (e: unknown) => void;
    const digests = new Promise<Digests>((resolve, reject) => {
      resolveDigests = resolve;
      rejectDigests = reject;
    });

    const backend = loadBackend(algs);

    super({
      transform(chunk: Uint8Array, controller: TransformStreamDefaultController<Uint8Array>): void {
        backend.update(chunk);
        controller.enqueue(chunk);
      },
      flush(): void {
        // Called only on clean close — resolve with final digests.
        resolveDigests(backend.finalize());
      },
    });

    this.digests = digests;

    // Intercept writable.abort() to reject the digests promise when the stream is aborted.
    // writable.closed is not implemented in Bun, so we patch the abort method instead.
    // flush() is not called on abort, so this is the only hook available.
    const origAbort = this.writable.abort.bind(this.writable);
    this.writable.abort = (reason?: unknown): Promise<void> => {
      rejectDigests(reason);
      return origAbort(reason);
    };
  }
}
