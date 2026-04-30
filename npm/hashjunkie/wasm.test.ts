import { expect, test } from "bun:test";
import { makeWasmBackend } from "./wasm";

// SHA-256("abc") — verified via sha256sum and Python hashlib.
const ABC_SHA256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const ABC_CID = "bafkreif2pall7dybz7vecqka3zo24irdwabwdi4wc55jznaq75q7eaavvu";
const ZEROES_MULTI_CIDV0 = "Qmc2SWxBGrBtWKZxuyg8999QuzXsPR47zsWiM7Yq9YFUXT";
const ZEROES_MULTI_CIDV1 = "bafybeigllfqgfpqydppr6cmv56g7ax4wyhruzswvcefv6j5kj77nzttfki";
const ABC_ED2K = "a448017aaf21d8525fc10ae87aa6729d";
const EMPTY_TIGER = "LWPNACQDBZRYXW3VHJVCJ64QBZNGHOHHHZWCLNQ";
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

test("makeWasmBackend: ed2k of 'abc' matches known MD4-compatible value", () => {
  const backend = makeWasmBackend(["ed2k"]);
  backend.update(new TextEncoder().encode("abc"));
  const digests = backend.finalize();
  expect(digests.ed2k).toBe(ABC_ED2K);
});

test("makeWasmBackend: tiger of empty input matches known Gnutella Tiger", () => {
  const backend = makeWasmBackend(["tiger"]);
  backend.update(new Uint8Array(0));
  const digests = backend.finalize();
  expect(digests.tiger).toBe(EMPTY_TIGER);
});

test("makeWasmBackend: cidv1 of 'abc' matches raw-leaf IPFS CID", () => {
  const backend = makeWasmBackend(["cidv1"]);
  backend.update(new TextEncoder().encode("abc"));
  const digests = backend.finalize();
  expect(digests.cidv1).toBe(ABC_CID);
});

test("makeWasmBackend: cidv0 and cidv1 match Kubo for multi-chunk input", () => {
  const backend = makeWasmBackend(["cidv0", "cidv1"]);
  backend.update(new Uint8Array(262_145));
  const digests = backend.finalize();
  expect(digests.cidv0).toBe(ZEROES_MULTI_CIDV0);
  expect(digests.cidv1).toBe(ZEROES_MULTI_CIDV1);
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
