# JS/TS Package Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `npm/hashjunkie` — the JS/TS package that exposes `HashJunkie extends TransformStream` with a `digests: Promise<Digests>` property, backed by the native `.node` addon with WASM fallback.

**Architecture:** A CommonJS TypeScript package using Bun's native test runner. Three source modules: `types.ts` (Algorithm union + runtime validation), `loader.ts` (platform addon dispatch + backend adapter + `_setLoaders` test hook), `index.ts` (TransformStream subclass). Bun runs TS directly — no build step needed for dev/test.

**Tech Stack:** TypeScript, Bun 1.3+, Biome (format+lint), `bun test --coverage`, napi-rs native addon (pre-built `.node` file)

---

## File Structure

| File | Role |
|---|---|
| `npm/package.json` | Workspace root — links all `npm/*` packages |
| `npm/hashjunkie/package.json` | Package manifest: name, scripts, optionalDependencies |
| `npm/hashjunkie/tsconfig.json` | `strict: true`, `moduleResolution: "bundler"`, `noEmit: true` |
| `npm/hashjunkie/biome.json` | Biome format + lint config |
| `npm/hashjunkie/types.ts` | `Algorithm`, `ALGORITHMS`, `Digests`, `Backend`, `parseAlgorithms()` |
| `npm/hashjunkie/types.test.ts` | Unit tests for all type-level logic |
| `npm/hashjunkie/loader.ts` | `_getPlatformPackage()`, `_tryRequire()`, `_defaultLoadNative/Wasm()`, `_setLoaders()`, `loadBackend()` |
| `npm/hashjunkie/loader.test.ts` | Unit tests for loader with mock injection + platform mapping |
| `npm/hashjunkie/index.ts` | `HashJunkie extends TransformStream`, re-exports |
| `npm/hashjunkie/index.test.ts` | Unit tests for HashJunkie: passthrough, digests, rejection, constructor validation |

**Not in this plan:** WASM embedding (Plan 5 / CI), esbuild/SEA packaging, npm publish config.

---

### Task 1: Workspace & Project Scaffolding

**Files:**
- Create: `npm/package.json`
- Create: `npm/hashjunkie/package.json`
- Create: `npm/hashjunkie/tsconfig.json`
- Create: `npm/hashjunkie/biome.json`
- Modify: `.gitignore` (verify `*.node` files are excluded)

- [ ] **Step 1: Create the workspace manifest**

`npm/package.json`:
```json
{
  "name": "hashjunkie-workspace",
  "private": true,
  "workspaces": [
    "hashjunkie",
    "hashjunkie-linux-x64-gnu",
    "hashjunkie-linux-arm64-gnu",
    "hashjunkie-darwin-x64",
    "hashjunkie-darwin-arm64",
    "hashjunkie-win32-x64-msvc"
  ]
}
```

- [ ] **Step 2: Create the package manifest**

`npm/hashjunkie/package.json`:
```json
{
  "name": "hashjunkie",
  "version": "0.1.0",
  "description": "Multi-hash streaming library for Node.js and Bun — all hashes in one pass",
  "main": "index.ts",
  "scripts": {
    "lint": "biome check .",
    "lint:fix": "biome check --write .",
    "test": "bun test",
    "test:coverage": "bun test --coverage"
  },
  "devDependencies": {
    "@biomejs/biome": "^1.9.0"
  },
  "optionalDependencies": {
    "@hashjunkie/linux-x64-gnu": "workspace:*",
    "@hashjunkie/linux-arm64-gnu": "workspace:*",
    "@hashjunkie/darwin-x64": "workspace:*",
    "@hashjunkie/darwin-arm64": "workspace:*",
    "@hashjunkie/win32-x64-msvc": "workspace:*"
  },
  "license": "MIT"
}
```

- [ ] **Step 3: Create tsconfig.json**

`npm/hashjunkie/tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "Preserve",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM"],
    "strict": true,
    "noEmit": true,
    "skipLibCheck": true
  },
  "include": ["*.ts"]
}
```

- [ ] **Step 4: Create biome.json**

`npm/hashjunkie/biome.json`:
```json
{
  "$schema": "https://biomejs.dev/schemas/1.9.4/schema.json",
  "organizeImports": { "enabled": true },
  "linter": {
    "enabled": true,
    "rules": { "recommended": true }
  },
  "formatter": {
    "enabled": true,
    "indentStyle": "space",
    "indentWidth": 2,
    "lineWidth": 100
  }
}
```

- [ ] **Step 5: Verify .gitignore covers .node files**

Check that the repo root `.gitignore` ignores `*.node` files:

```bash
grep '\.node' /src/HashJunkie/.gitignore
```

Expected output: a line like `*.node`. If missing, add it:
```
*.node
```

- [ ] **Step 6: Copy built native addon for local dev**

Copy the release-built `.so` to the expected `.node` path so the loader and integration tests can find it:

```bash
cp /src/HashJunkie/target/release/libhashjunkie_napi.so \
   /src/HashJunkie/npm/hashjunkie/hashjunkie.linux-x64-gnu.node
```

This file is gitignored (`*.node`). CI will provide it via artifact download.

- [ ] **Step 7: Install workspace dependencies**

```bash
cd /src/HashJunkie/npm && /home/per/.bun/bin/bun install
```

Expected: Bun installs Biome devDependency and links workspace packages. No errors.

- [ ] **Step 8: Commit scaffolding**

```bash
cd /src/HashJunkie
git add npm/package.json npm/hashjunkie/package.json npm/hashjunkie/tsconfig.json npm/hashjunkie/biome.json
git commit -m "chore: scaffold npm/hashjunkie package with workspace and tooling"
```

---

### Task 2: Types Module

**Files:**
- Create: `npm/hashjunkie/types.ts`
- Create: `npm/hashjunkie/types.test.ts`

- [ ] **Step 1: Write the failing tests**

`npm/hashjunkie/types.test.ts`:
```ts
import { expect, test } from 'bun:test';
import { ALGORITHMS, parseAlgorithms } from './types';

test('ALGORITHMS contains exactly 13 algorithms', () => {
  expect(ALGORITHMS).toHaveLength(13);
});

test('ALGORITHMS includes all required algorithm names', () => {
  const required = [
    'blake3', 'crc32', 'dropbox', 'hidrive', 'mailru',
    'md5', 'quickxor', 'sha1', 'sha256', 'sha512',
    'whirlpool', 'xxh128', 'xxh3',
  ];
  for (const name of required) {
    expect(ALGORITHMS).toContain(name);
  }
});

test('parseAlgorithms() with no argument returns all 13 algorithms', () => {
  const result = parseAlgorithms();
  expect(result).toHaveLength(13);
  expect(result).toEqual([...ALGORITHMS]);
});

test('parseAlgorithms() returns a mutable copy (not the const array)', () => {
  const result = parseAlgorithms();
  result.push('sha256' as never); // should not throw
  expect(ALGORITHMS).toHaveLength(13); // original unchanged
});

test('parseAlgorithms() with a valid subset returns that subset', () => {
  expect(parseAlgorithms(['sha256', 'blake3'])).toEqual(['sha256', 'blake3']);
});

test('parseAlgorithms() with empty array throws TypeError', () => {
  expect(() => parseAlgorithms([])).toThrow(TypeError);
  expect(() => parseAlgorithms([])).toThrow('must not be empty');
});

test('parseAlgorithms() with unknown algorithm name throws TypeError', () => {
  expect(() => parseAlgorithms(['sha256', 'md99'])).toThrow(TypeError);
  expect(() => parseAlgorithms(['sha256', 'md99'])).toThrow('"md99"');
});

test('parseAlgorithms() with single unknown algorithm throws TypeError', () => {
  expect(() => parseAlgorithms(['bogus'])).toThrow(TypeError);
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun test types.test.ts
```

Expected: FAIL — `Cannot find module './types'`

- [ ] **Step 3: Write the implementation**

`npm/hashjunkie/types.ts`:
```ts
export const ALGORITHMS = [
  'blake3', 'crc32', 'dropbox', 'hidrive', 'mailru',
  'md5', 'quickxor', 'sha1', 'sha256', 'sha512',
  'whirlpool', 'xxh128', 'xxh3',
] as const;

export type Algorithm = (typeof ALGORITHMS)[number];

export type Digests = Record<Algorithm, string>;

const ALGORITHM_SET = new Set<string>(ALGORITHMS);

/** Backend interface implemented by both the native addon wrapper and the WASM wrapper. */
export type Backend = {
  update(data: Uint8Array): void;
  finalize(): Digests;
};

/**
 * Validates and returns the algorithm list.
 * Returns all 13 algorithms when called with no argument.
 * Throws TypeError for unknown algorithm names or an empty array.
 */
export function parseAlgorithms(algorithms?: readonly string[]): Algorithm[] {
  if (algorithms === undefined) return [...ALGORITHMS];
  if (algorithms.length === 0) {
    throw new TypeError(
      'algorithms must not be empty; omit the argument to use all algorithms',
    );
  }
  for (const alg of algorithms) {
    if (!ALGORITHM_SET.has(alg)) {
      throw new TypeError(`unknown algorithm: "${alg}"`);
    }
  }
  return algorithms as Algorithm[];
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun test types.test.ts
```

Expected: 8 tests pass, 0 failures.

- [ ] **Step 5: Run Biome check**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun run lint
```

Expected: no errors or warnings. Fix any formatting issues with `bun run lint:fix`.

- [ ] **Step 6: Commit**

```bash
cd /src/HashJunkie
git add npm/hashjunkie/types.ts npm/hashjunkie/types.test.ts
git commit -m "feat: Algorithm types, ALGORITHMS constant, and parseAlgorithms validator"
```

---

### Task 3: Loader Module

**Files:**
- Create: `npm/hashjunkie/loader.ts`
- Create: `npm/hashjunkie/loader.test.ts`

- [ ] **Step 1: Write the failing tests**

`npm/hashjunkie/loader.test.ts`:
```ts
import { afterEach, expect, test } from 'bun:test';
import {
  _defaultLoadWasm,
  _getPlatformPackage,
  _setLoaders,
  _tryRequire,
  loadBackend,
} from './loader';
import type { Digests } from './types';

const MOCK_DIGESTS: Digests = {
  blake3: 'aa', crc32: 'bb', dropbox: 'cc', hidrive: 'dd', mailru: 'ee',
  md5: 'ff', quickxor: '00', sha1: '11', sha256: '22', sha512: '33',
  whirlpool: '44', xxh128: '55', xxh3: '66',
};

afterEach(() => {
  // Reset loaders so test isolation is preserved
  _setLoaders({ loadNative: () => null, loadWasm: () => null });
});

// --- _getPlatformPackage ---

test('_getPlatformPackage maps all 5 supported platform/arch combos', () => {
  expect(_getPlatformPackage('linux', 'x64')).toBe('hashjunkie.linux-x64-gnu.node');
  expect(_getPlatformPackage('linux', 'arm64')).toBe('hashjunkie.linux-arm64-gnu.node');
  expect(_getPlatformPackage('darwin', 'x64')).toBe('hashjunkie.darwin-x64.node');
  expect(_getPlatformPackage('darwin', 'arm64')).toBe('hashjunkie.darwin-arm64.node');
  expect(_getPlatformPackage('win32', 'x64')).toBe('hashjunkie.win32-x64-msvc.node');
});

test('_getPlatformPackage returns null for unsupported platform', () => {
  expect(_getPlatformPackage('freebsd', 'x64')).toBeNull();
});

test('_getPlatformPackage returns null for unsupported arch', () => {
  expect(_getPlatformPackage('linux', 'arm')).toBeNull();
});

// --- _tryRequire ---

test('_tryRequire returns null when module does not exist', () => {
  expect(_tryRequire('./definitely-does-not-exist-xyz.node')).toBeNull();
});

test('_tryRequire returns the module when it exists', () => {
  // 'path' is a Node/Bun built-in that always resolves
  const result = _tryRequire('path');
  expect(result).not.toBeNull();
});

// --- _defaultLoadWasm ---

test('_defaultLoadWasm returns null (WASM not yet embedded — Plan 5)', () => {
  expect(_defaultLoadWasm()).toBeNull();
});

// --- loadBackend with native addon ---

test('loadBackend returns a backend that delegates update() and finalize() to the native instance', () => {
  const updateCalls: Buffer[] = [];
  _setLoaders({
    loadNative: () => ({
      NativeHasher: class {
        update(data: Buffer): void { updateCalls.push(data); }
        finalize(): Record<string, string> { return MOCK_DIGESTS; }
      },
    }),
    loadWasm: () => null,
  });

  const backend = loadBackend(['sha256', 'blake3']);
  const chunk = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
  backend.update(chunk);

  expect(updateCalls).toHaveLength(1);
  expect(updateCalls[0]).toBeInstanceOf(Buffer);
  expect(Array.from(updateCalls[0]!)).toEqual([0xde, 0xad, 0xbe, 0xef]);
  expect(backend.finalize()).toEqual(MOCK_DIGESTS);
});

test('loadBackend converts Uint8Array to Buffer before passing to native update', () => {
  let received: Buffer | null = null;
  _setLoaders({
    loadNative: () => ({
      NativeHasher: class {
        update(data: Buffer): void { received = data; }
        finalize(): Record<string, string> { return MOCK_DIGESTS; }
      },
    }),
    loadWasm: () => null,
  });

  loadBackend(['sha256']).update(new Uint8Array([0x01, 0x02]));
  expect(received).toBeInstanceOf(Buffer);
});

// --- loadBackend WASM fallback ---

test('loadBackend uses WASM backend when native returns null', () => {
  const mockWasm = {
    update(_data: Uint8Array): void {},
    finalize(): Digests { return MOCK_DIGESTS; },
  };
  _setLoaders({ loadNative: () => null, loadWasm: () => mockWasm });

  const backend = loadBackend(['sha256']);
  expect(backend).toBe(mockWasm);
});

// --- loadBackend no backend ---

test('loadBackend throws Error when both loaders return null', () => {
  _setLoaders({ loadNative: () => null, loadWasm: () => null });
  expect(() => loadBackend(['sha256'])).toThrow(Error);
  expect(() => loadBackend(['sha256'])).toThrow('no backend available');
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun test loader.test.ts
```

Expected: FAIL — `Cannot find module './loader'`

- [ ] **Step 3: Write the implementation**

`npm/hashjunkie/loader.ts`:
```ts
import type { Algorithm, Backend, Digests } from './types';

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
  if (platform === 'linux' && arch === 'x64') return 'hashjunkie.linux-x64-gnu.node';
  if (platform === 'linux' && arch === 'arm64') return 'hashjunkie.linux-arm64-gnu.node';
  if (platform === 'darwin' && arch === 'x64') return 'hashjunkie.darwin-x64.node';
  if (platform === 'darwin' && arch === 'arm64') return 'hashjunkie.darwin-arm64.node';
  if (platform === 'win32' && arch === 'x64') return 'hashjunkie.win32-x64-msvc.node';
  return null;
}

/**
 * Attempts to require() a module path. Returns null if the module is not found or
 * cannot be loaded. Exported so both success and failure branches are unit-testable.
 */
// biome-ignore lint/suspicious/noExplicitAny: returns unknown module shape
export function _tryRequire(path: string): any {
  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
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
  if (process.platform === 'linux' && process.arch === 'x64')
    return _tryRequire('./hashjunkie.linux-x64-gnu.node') as NativeAddon | null;
  if (process.platform === 'linux' && process.arch === 'arm64')
    return _tryRequire('./hashjunkie.linux-arm64-gnu.node') as NativeAddon | null;
  if (process.platform === 'darwin' && process.arch === 'x64')
    return _tryRequire('./hashjunkie.darwin-x64.node') as NativeAddon | null;
  if (process.platform === 'darwin' && process.arch === 'arm64')
    return _tryRequire('./hashjunkie.darwin-arm64.node') as NativeAddon | null;
  if (process.platform === 'win32' && process.arch === 'x64')
    return _tryRequire('./hashjunkie.win32-x64-msvc.node') as NativeAddon | null;
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
    'hashjunkie: no backend available — native addon failed to load and WASM is not embedded',
  );
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun test loader.test.ts
```

Expected: 11 tests pass, 0 failures.

- [ ] **Step 5: Run Biome check**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun run lint
```

Expected: no errors. Fix any issues with `bun run lint:fix`.

- [ ] **Step 6: Commit**

```bash
cd /src/HashJunkie
git add npm/hashjunkie/loader.ts npm/hashjunkie/loader.test.ts
git commit -m "feat: loader module with platform dispatch, _setLoaders test hook, and loadBackend"
```

---

### Task 4: HashJunkie Class

**Files:**
- Create: `npm/hashjunkie/index.ts`
- Create: `npm/hashjunkie/index.test.ts`

- [ ] **Step 1: Write the failing tests**

`npm/hashjunkie/index.test.ts`:
```ts
import { afterEach, beforeEach, expect, test } from 'bun:test';
import { _setLoaders } from './loader';
import { ALGORITHMS, HashJunkie } from './index';
import type { Digests } from './types';

const MOCK_DIGESTS: Digests = {
  blake3: 'aa', crc32: 'bb', dropbox: 'cc', hidrive: 'dd', mailru: 'ee',
  md5: 'ff', quickxor: '00', sha1: '11', sha256: '22', sha512: '33',
  whirlpool: '44', xxh128: '55', xxh3: '66',
};

// Install a working mock backend before every test so constructing HashJunkie succeeds.
beforeEach(() => {
  _setLoaders({
    loadNative: () => ({
      NativeHasher: class {
        update(_data: Buffer): void {}
        finalize(): Record<string, string> { return MOCK_DIGESTS; }
      },
    }),
    loadWasm: () => null,
  });
});

afterEach(() => {
  _setLoaders({ loadNative: () => null, loadWasm: () => null });
});

/** Writes chunks through hj, collects all output chunks, closes the stream. */
async function pipe(hj: HashJunkie, inputs: Uint8Array[]): Promise<Uint8Array[]> {
  const out: Uint8Array[] = [];
  const reader = hj.readable.getReader();
  const writer = hj.writable.getWriter();
  await Promise.all([
    (async () => {
      for (const chunk of inputs) await writer.write(chunk);
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
  return out;
}

// --- re-exports ---

test('ALGORITHMS is re-exported from index', () => {
  expect(ALGORITHMS).toHaveLength(13);
  expect(ALGORITHMS).toContain('sha256');
});

// --- passthrough ---

test('HashJunkie passes all chunks through unchanged', async () => {
  const hj = new HashJunkie(['sha256']);
  const chunks = [new Uint8Array([1, 2, 3]), new Uint8Array([4, 5])];
  const result = await pipe(hj, chunks);
  expect(result).toHaveLength(2);
  expect(result[0]).toEqual(new Uint8Array([1, 2, 3]));
  expect(result[1]).toEqual(new Uint8Array([4, 5]));
});

test('HashJunkie works with zero chunks (empty stream)', async () => {
  const hj = new HashJunkie(['sha256']);
  const result = await pipe(hj, []);
  expect(result).toHaveLength(0);
});

// --- digests ---

test('HashJunkie.digests resolves with backend digests after stream close', async () => {
  const hj = new HashJunkie(['sha256', 'blake3']);
  await pipe(hj, [new Uint8Array([0xca, 0xfe])]);
  expect(await hj.digests).toEqual(MOCK_DIGESTS);
});

test('HashJunkie with no constructor arg uses all algorithms', async () => {
  const hj = new HashJunkie();
  await pipe(hj, []);
  const digests = await hj.digests;
  expect(Object.keys(digests).sort()).toEqual([...ALGORITHMS].sort());
});

// --- constructor validation ---

test('HashJunkie constructor throws TypeError synchronously for unknown algorithm', () => {
  expect(() => new HashJunkie(['sha256', 'not-real'] as never)).toThrow(TypeError);
  expect(() => new HashJunkie(['sha256', 'not-real'] as never)).toThrow('"not-real"');
});

test('HashJunkie constructor throws TypeError synchronously for empty array', () => {
  expect(() => new HashJunkie([])).toThrow(TypeError);
  expect(() => new HashJunkie([])).toThrow('must not be empty');
});

test('HashJunkie constructor throws Error when no backend is available', () => {
  _setLoaders({ loadNative: () => null, loadWasm: () => null });
  expect(() => new HashJunkie(['sha256'])).toThrow('no backend available');
});

// --- digests rejection ---

test('HashJunkie.digests rejects when the writable stream is aborted', async () => {
  const hj = new HashJunkie(['sha256']);
  const abortError = new Error('upstream abort');

  // abort() closes the writable side with an error; this.writable.closed rejects → rejectDigests fires
  await hj.writable.abort(abortError);

  let caught: unknown = undefined;
  try {
    await hj.digests;
  } catch (e) {
    caught = e;
  }
  expect(caught).toBe(abortError);
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun test index.test.ts
```

Expected: FAIL — `Cannot find module './index'`

- [ ] **Step 3: Write the implementation**

`npm/hashjunkie/index.ts`:
```ts
import { loadBackend } from './loader';
import { parseAlgorithms } from './types';
import type { Algorithm, Digests } from './types';

export { ALGORITHMS } from './types';
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

    // Reject digests when the writable side errors (flush is not called on error).
    this.writable.closed.catch((e: unknown) => {
      rejectDigests(e);
    });

    this.digests = digests;
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun test index.test.ts
```

Expected: 10 tests pass, 0 failures.

- [ ] **Step 5: Run Biome check**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun run lint
```

Expected: no errors. Fix any issues with `bun run lint:fix`.

- [ ] **Step 6: Run the full test suite**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun test
```

Expected: 29 tests pass (8 types + 11 loader + 10 index), 0 failures.

- [ ] **Step 7: Commit**

```bash
cd /src/HashJunkie
git add npm/hashjunkie/index.ts npm/hashjunkie/index.test.ts
git commit -m "feat: HashJunkie TransformStream class with digests promise"
```

---

### Task 5: Integration Test + Coverage Gate

**Files:**
- Create: `npm/hashjunkie/index.integration.test.ts`

The integration test calls `loadBackend()` with the real loaders (no mocking) and verifies actual hash output against known values. It is skipped automatically if the `.node` file is not present (e.g., CI without the artifact).

- [ ] **Step 1: Verify the .node file is present from Task 1**

```bash
ls -lh /src/HashJunkie/npm/hashjunkie/hashjunkie.linux-x64-gnu.node
```

Expected: file exists (copied in Task 1 Step 6). If missing, re-run:
```bash
cp /src/HashJunkie/target/release/libhashjunkie_napi.so \
   /src/HashJunkie/npm/hashjunkie/hashjunkie.linux-x64-gnu.node
```

- [ ] **Step 2: Write the integration test**

`npm/hashjunkie/index.integration.test.ts`:
```ts
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { expect, test } from 'bun:test';
import { HashJunkie } from './index';

// Skip when the native addon is not present (CI without pre-built artifact).
const NODE_FILE = join(import.meta.dir, 'hashjunkie.linux-x64-gnu.node');
const hasAddon = existsSync(NODE_FILE);

async function hashWith(hj: HashJunkie, data: Uint8Array): Promise<Record<string, string>> {
  const writer = hj.writable.getWriter();
  const reader = hj.readable.getReader();
  await Promise.all([
    (async () => { await writer.write(data); await writer.close(); })(),
    (async () => { for (;;) { const { done } = await reader.read(); if (done) break; } })(),
  ]);
  return hj.digests;
}

// Known SHA-256 and MD5 digests for the empty input (NIST / RFC-verified values).
const EMPTY_SHA256 = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855';
const EMPTY_MD5 = 'd41d8cd98f00b204e9800998ecf8427e';

test.if(hasAddon)(
  'HashJunkie with real native backend: sha256 of empty input matches known value',
  async () => {
    const hj = new HashJunkie(['sha256', 'md5']);
    const digests = await hashWith(hj, new Uint8Array(0));
    expect(digests['sha256']).toBe(EMPTY_SHA256);
    expect(digests['md5']).toBe(EMPTY_MD5);
  },
);

test.if(hasAddon)(
  'HashJunkie with real native backend: sha256 of known bytes matches expected',
  async () => {
    // SHA-256("abc") = ba7816bf 8f01cfea 414140de 5dae2ec7 3b338c18 8c8c326c 1abb2163 c923be99
    const hj = new HashJunkie(['sha256']);
    const data = new TextEncoder().encode('abc');
    const digests = await hashWith(hj, data);
    expect(digests['sha256']).toBe(
      'ba7816bf8f01cfea414140de5dae2ec73b338c188c8c326c1abb2163c923be99',
    );
  },
);

test.if(hasAddon)(
  'HashJunkie with real native backend: output bytes match input bytes',
  async () => {
    const hj = new HashJunkie(['sha256']);
    const input = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
    const out: Uint8Array[] = [];
    const reader = hj.readable.getReader();
    const writer = hj.writable.getWriter();
    await Promise.all([
      (async () => { await writer.write(input); await writer.close(); })(),
      (async () => { for (;;) { const { done, value } = await reader.read(); if (done) break; out.push(value); } })(),
    ]);
    const combined = new Uint8Array(out.flatMap(c => [...c]));
    expect(combined).toEqual(input);
  },
);
```

- [ ] **Step 3: Run the integration tests**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun test index.integration.test.ts
```

Expected (with .node present): 3 tests pass. Expected (without .node): 3 tests skip.

- [ ] **Step 4: Run the full test suite with coverage**

```bash
cd /src/HashJunkie/npm/hashjunkie && /home/per/.bun/bin/bun test --coverage
```

Expected output: all tests pass. Coverage report shows `types.ts`, `loader.ts`, and `index.ts` at or near 100%, with only the `/* c8 ignore start/stop */` block in `_defaultLoadNative()` excluded.

If any file is below 100% (excluding c8 ignore regions), add the missing test before proceeding.

- [ ] **Step 5: Commit**

```bash
cd /src/HashJunkie
git add npm/hashjunkie/index.integration.test.ts
git commit -m "test: integration tests for HashJunkie with real native backend"
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Covered by |
|---|---|
| `HashJunkie extends TransformStream<Uint8Array, Uint8Array>` | Task 4 `index.ts` |
| `constructor(algorithms?: Algorithm[])` | Task 4, tested in Task 4 tests |
| `readonly digests: Promise<Digests>` | Task 4 `index.ts`, tested for resolve + reject |
| Invalid algorithm → synchronous TypeError | Task 2 `parseAlgorithms()`, tested |
| Native addon: static `require()` literals for bundler | Task 3 `_defaultLoadNative()` |
| WASM fallback path | Task 3 `loadBackend()` with mock, tested |
| Both fail → Error thrown | Task 3 loader test + Task 4 constructor test |
| Passthrough: output bytes === input bytes | Task 4 passthrough test + Task 5 integration |
| Digests reject on stream abort | Task 4 rejection test |
| `bun test --coverage` at 100% | Task 5 Step 4 |
| Biome format + lint | Every task Step 5 |

**Placeholder scan:** No TBD, TODO, or vague steps found.

**Type consistency:** `Backend`, `Algorithm`, `Digests` defined once in `types.ts`, imported everywhere. `NativeAddon`, `NativeHasherInstance` defined in `loader.ts` only. `HashJunkie` constructor calls `parseAlgorithms()` (defined in Task 2) and `loadBackend()` (defined in Task 3) — no naming drift.
