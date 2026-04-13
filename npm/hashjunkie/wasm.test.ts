import { expect, test } from "bun:test";
import { makeWasmBackend } from "./wasm";

// SHA-256("abc") — verified via sha256sum and Python hashlib.
const ABC_SHA256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
// MD5("") — verified via md5sum /dev/null / RFC 1321.
const EMPTY_MD5 = "d41d8cd98f00b204e9800998ecf8427e";

test("makeWasmBackend: sha256 of 'abc' matches known value", () => {
  const backend = makeWasmBackend(["sha256"]);
  backend.update(new TextEncoder().encode("abc"));
  const digests = backend.finalize();
  expect(digests.sha256).toBe(ABC_SHA256);
});

test("makeWasmBackend: md5 of empty input matches known value", () => {
  const backend = makeWasmBackend(["md5"]);
  backend.update(new Uint8Array(0));
  const digests = backend.finalize();
  expect(digests.md5).toBe(EMPTY_MD5);
});

test("makeWasmBackend: multi-chunk update matches single-chunk", () => {
  const single = makeWasmBackend(["sha256"]);
  single.update(new TextEncoder().encode("hello world"));
  const singleDigests = single.finalize();

  const multi = makeWasmBackend(["sha256"]);
  multi.update(new TextEncoder().encode("hello"));
  multi.update(new TextEncoder().encode(" world"));
  const multiDigests = multi.finalize();

  expect(multiDigests.sha256).toBe(singleDigests.sha256);
});

test("makeWasmBackend: WASM init is idempotent (calling twice is safe)", () => {
  // Exercises the `if (initialized) return;` early-return branch in ensureInit().
  makeWasmBackend(["md5"]);
  const backend = makeWasmBackend(["sha256"]);
  backend.update(new Uint8Array([0x61])); // 'a'
  // SHA-256("a") — verified via sha256sum.
  expect(backend.finalize().sha256).toBe(
    "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb",
  );
});
