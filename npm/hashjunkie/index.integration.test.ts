import { expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { HashJunkie } from "./index";

// Skip when the native addon is not present (CI without pre-built artifact).
const NODE_FILE = join(import.meta.dir, "hashjunkie.linux-x64-gnu.node");
const hasAddon = existsSync(NODE_FILE);

async function hashWith(hj: HashJunkie, data: Uint8Array): Promise<Record<string, string>> {
  const writer = hj.writable.getWriter();
  const reader = hj.readable.getReader();
  await Promise.all([
    (async () => {
      await writer.write(data);
      await writer.close();
    })(),
    (async () => {
      for (;;) {
        const { done } = await reader.read();
        if (done) break;
      }
    })(),
  ]);
  return hj.digests;
}

// Known SHA-256 and MD5 digests — verified via coreutils sha256sum / md5sum.
const EMPTY_SHA256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const EMPTY_MD5 = "d41d8cd98f00b204e9800998ecf8427e";
// SHA-256("abc") — verified via sha256sum and Python hashlib.
const ABC_SHA256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

test.if(hasAddon)(
  "HashJunkie with real native backend: sha256 of empty input matches known value",
  async () => {
    const hj = new HashJunkie(["sha256", "md5"]);
    const digests = await hashWith(hj, new Uint8Array(0));
    expect(digests.sha256).toBe(EMPTY_SHA256);
    expect(digests.md5).toBe(EMPTY_MD5);
  },
);

test.if(hasAddon)(
  "HashJunkie with real native backend: sha256 of known bytes matches expected",
  async () => {
    const hj = new HashJunkie(["sha256"]);
    const data = new TextEncoder().encode("abc");
    const digests = await hashWith(hj, data);
    expect(digests.sha256).toBe(ABC_SHA256);
  },
);

test.if(hasAddon)(
  "HashJunkie with real native backend: output bytes match input bytes",
  async () => {
    const hj = new HashJunkie(["sha256"]);
    const input = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
    const out: Uint8Array[] = [];
    const reader = hj.readable.getReader();
    const writer = hj.writable.getWriter();
    await Promise.all([
      (async () => {
        await writer.write(input);
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
    const combined = new Uint8Array(out.flatMap((c) => [...c]));
    expect(combined).toEqual(input);
  },
);
