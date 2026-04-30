import { loadBackend } from "./loader";
import type { Algorithm, Digests } from "./types";
import { parseAlgorithms } from "./types";

export { ALGORITHMS } from "./types";
export type { Algorithm, Digests };

/**
 * TransformStream that computes hashes on every byte that flows through it.
 *
 * Plug into a Web Streams pipeline with `pipeThrough`, or — if you only care
 * about the digests and not the pass-through bytes — reach for `hashBuffer` /
 * `hashStream` below, which handle the stream plumbing for you.
 */
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

    // Load before super() so backend is captured in the transformer callbacks' closure.
    const backend = loadBackend(algs);

    super(
      {
        transform(
          chunk: Uint8Array,
          controller: TransformStreamDefaultController<Uint8Array>,
        ): void {
          backend.update(chunk);
          controller.enqueue(chunk);
        },
        flush(): void {
          // Called only on clean close — resolve with final digests.
          // Note: if finalize() threw, TransformStream would error the readable side,
          // but digests would hang. finalize() is infallible (pure state computation),
          // so this is an accepted known limitation.
          resolveDigests(backend.finalize());
        },
      },
      // writableStrategy: default is fine — callers pace their own writes.
      undefined,
      // readableStrategy: effectively-unbounded HWM so `controller.enqueue` never
      // back-pressures. Without this, the WHATWG default readable HWM of 0 stalls
      // `writer.write()` forever whenever no one drains the readable side — which
      // is exactly the "compute digests, discard bytes" pattern this class is
      // designed for (see hashBuffer / hashStream).
      { highWaterMark: Number.POSITIVE_INFINITY },
    );

    this.digests = digests;

    // Intercept writable.abort() to reject the digests promise when the stream is aborted.
    // writable.closed is not implemented in Bun, so we patch the abort method instead.
    // flush() is not called on abort, so this is the only hook available.
    const origAbort = this.writable.abort.bind(this.writable);
    let digestsRejected = false;
    this.writable.abort = async (reason?: unknown): Promise<void> => {
      if (!digestsRejected) {
        digestsRejected = true;
        rejectDigests(reason);
      }
      return origAbort(reason);
    };
  }
}

/**
 * Hash a single in-memory buffer. The bytes go in, digests come out — no
 * streams, no writers, no pipes.
 *
 * @param data - input bytes to hash
 * @param algorithms - subset of algorithms to compute; omit for all 15
 */
export async function hashBuffer(data: Uint8Array, algorithms?: Algorithm[]): Promise<Digests> {
  const hj = new HashJunkie(algorithms);
  const writer = hj.writable.getWriter();
  await writer.write(data);
  await writer.close();
  return hj.digests;
}

/**
 * Hash everything emitted by a `ReadableStream<Uint8Array>`. The stream is
 * fully drained; the pass-through bytes are discarded.
 *
 * @param stream - source stream to consume
 * @param algorithms - subset of algorithms to compute; omit for all 15
 */
export async function hashStream(
  stream: ReadableStream<Uint8Array>,
  algorithms?: Algorithm[],
): Promise<Digests> {
  const hj = new HashJunkie(algorithms);
  await stream.pipeThrough(hj).pipeTo(
    new WritableStream({
      write(): void {},
    }),
  );
  return hj.digests;
}
