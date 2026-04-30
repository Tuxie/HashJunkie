# CLI Release Assets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the existing GitHub Actions release workflow so it builds, packages, and publishes per-platform `hashjunkie` CLI archives to GitHub Releases using the same version as the npm packages published in that run.

**Architecture:** Keep a single `ci.yml` workflow. Reuse the existing release gate and version computation, expand the native build matrix to also build and package `hashjunkie-cli`, then have the `publish` job download those archives and attach them to the `v{version}` GitHub Release only after npm publication succeeds.

**Tech Stack:** GitHub Actions, Bash, PowerShell, Rust cargo build, `gh` CLI, `tar`, `zip`, `act`

---

## File Structure

- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`

### Task 1: Expand release gating and matrix metadata for CLI assets

**Files:**
- Modify: `.github/workflows/ci.yml:1-220`
- Test: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the failing workflow assertions by editing comments and matrix expectations**

```yaml
  should-release:
    steps:
      - name: Detect release-relevant changes
        run: |
          # Expected release-trigger coverage now includes:
          #   crates/hashjunkie-cli/Cargo.toml
          #   crates/hashjunkie-cli/src/
          #   .github/workflows/ci.yml
          release_files=$(echo "$changed" \
            | grep -E '^(crates/hashjunkie-cli/Cargo\.toml|crates/[^/]+/src/|crates/hashjunkie-napi/build\.rs|Cargo\.(toml|lock)|npm/hashjunkie/[^/]+\.(ts|js)|npm/[^/]+/package\.json|\.github/workflows/ci\.yml)' \
            | grep -vE '\.test\.ts$')

  build-native:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            runner: ubuntu-latest
            node-file: hashjunkie.linux-x64-gnu.node
            cli-platform: linux-x64-gnu
            cli-archive-ext: tar.xz
```

- [ ] **Step 2: Run workflow validation to verify it fails or is incomplete before the implementation is finished**

Run: `python3 - <<'PY'\nfrom pathlib import Path\ntext = Path('.github/workflows/ci.yml').read_text()\nassert 'crates/hashjunkie-cli/Cargo.toml' in text\nassert '.github/workflows/ci.yml' in text\nassert 'cli-platform:' in text\nassert 'cli-archive-ext:' in text\nPY`
Expected: FAIL before the matrix and release gate are fully updated.

- [ ] **Step 3: Write the minimal implementation**

```yaml
  should-release:
    steps:
      - name: Detect release-relevant changes
        id: check
        run: |
          changed=$(git diff --name-only HEAD~1 HEAD)
          release_files=$(echo "$changed" \
            | grep -E '^(crates/hashjunkie-cli/Cargo\.toml|crates/[^/]+/src/|crates/hashjunkie-napi/build\.rs|Cargo\.(toml|lock)|npm/hashjunkie/[^/]+\.(ts|js)|npm/[^/]+/package\.json|\.github/workflows/ci\.yml)' \
            | grep -vE '\.test\.ts$')

  build-native:
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            runner: ubuntu-latest
            node-file: hashjunkie.linux-x64-gnu.node
            cli-platform: linux-x64-gnu
            cli-archive-ext: tar.xz
          - target: aarch64-unknown-linux-gnu
            runner: ubuntu-24.04-arm
            node-file: hashjunkie.linux-arm64-gnu.node
            cli-platform: linux-arm64-gnu
            cli-archive-ext: tar.xz
          - target: x86_64-apple-darwin
            runner: macos-latest
            node-file: hashjunkie.darwin-x64.node
            cli-platform: darwin-x64
            cli-archive-ext: tar.xz
          - target: aarch64-apple-darwin
            runner: macos-latest
            node-file: hashjunkie.darwin-arm64.node
            cli-platform: darwin-arm64
            cli-archive-ext: tar.xz
          - target: x86_64-pc-windows-msvc
            runner: windows-latest
            node-file: hashjunkie.win32-x64-msvc.node
            cli-platform: win32-x64-msvc
            cli-archive-ext: zip
```

- [ ] **Step 4: Run validation to verify it passes**

Run: `python3 - <<'PY'\nfrom pathlib import Path\ntext = Path('.github/workflows/ci.yml').read_text()\nassert 'crates/hashjunkie-cli/Cargo.toml' in text\nassert '.github/workflows/ci.yml' in text\nassert text.count('cli-platform:') == 5\nassert text.count('cli-archive-ext:') == 5\nPY`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: expand release gate for cli assets"
```

### Task 2: Build and upload per-platform CLI release archives in the matrix job

**Files:**
- Modify: `.github/workflows/ci.yml:180-260`
- Test: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the failing assertions for CLI packaging**

```yaml
      - name: Build CLI
        run: >
          cargo build
          -p hashjunkie-cli
          --release
          --target ${{ matrix.target }}

      - name: Package CLI archive (Linux/macOS)
        if: runner.os != 'Windows'
        run: |
          V="0.0.0-test"
          OUT="hashjunkie-cli-${V}-${{ matrix.cli-platform }}.tar.xz"
          mkdir -p /tmp/hashjunkie-cli
          cp target/${{ matrix.target }}/release/hashjunkie /tmp/hashjunkie-cli/hashjunkie
          tar -C /tmp/hashjunkie-cli -cJf "$OUT" hashjunkie
```

- [ ] **Step 2: Run workflow validation to verify the archive steps are not present yet**

Run: `python3 - <<'PY'\nfrom pathlib import Path\ntext = Path('.github/workflows/ci.yml').read_text()\nassert 'Build CLI' in text\nassert 'Package CLI archive (Linux/macOS)' in text\nassert 'Package CLI archive (Windows)' in text\nassert 'Upload CLI archive artifact' in text\nPY`
Expected: FAIL before the archive steps are added.

- [ ] **Step 3: Write the minimal implementation**

```yaml
      - name: Build
        run: >
          cargo build
          -p hashjunkie-napi
          -p hashjunkie-cli
          --release
          --target ${{ matrix.target }}

      - name: Package CLI archive (Linux/macOS)
        if: runner.os != 'Windows'
        run: |
          V="${GITHUB_REF_NAME#v}"
          [ -n "$V" ] || V="0.0.0-dev"
          OUT="hashjunkie-cli-${V}-${{ matrix.cli-platform }}.tar.xz"
          mkdir -p /tmp/hashjunkie-cli
          cp target/${{ matrix.target }}/release/hashjunkie /tmp/hashjunkie-cli/hashjunkie
          tar -C /tmp/hashjunkie-cli -cJf "$OUT" hashjunkie
          echo "CLI_ARCHIVE=$OUT" >> "$GITHUB_ENV"

      - name: Package CLI archive (Windows)
        if: runner.os == 'Windows'
        shell: pwsh
        run: |
          $version = if ($env:GITHUB_REF_NAME) { $env:GITHUB_REF_NAME -replace '^v','' } else { '0.0.0-dev' }
          $out = "hashjunkie-cli-$version-${{ matrix.cli-platform }}.zip"
          New-Item -ItemType Directory -Force -Path dist-cli | Out-Null
          Copy-Item "target\${{ matrix.target }}\release\hashjunkie.exe" "dist-cli\hashjunkie.exe"
          Compress-Archive -Path "dist-cli\hashjunkie.exe" -DestinationPath $out -Force
          "CLI_ARCHIVE=$out" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append

      - name: Upload CLI archive artifact
        uses: actions/upload-artifact@v7
        with:
          name: cli-${{ matrix.cli-platform }}
          path: ${{ env.CLI_ARCHIVE }}
          retention-days: 1
```

- [ ] **Step 4: Run validation to verify it passes**

Run: `python3 - <<'PY'\nfrom pathlib import Path\ntext = Path('.github/workflows/ci.yml').read_text()\nfor needle in ['-p hashjunkie-cli', 'Package CLI archive (Linux/macOS)', 'Package CLI archive (Windows)', 'Upload CLI archive artifact']:\n    assert needle in text, needle\nPY`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: package cli archives in native build matrix"
```

### Task 3: Release the CLI archives with the same computed publish version

**Files:**
- Modify: `.github/workflows/ci.yml:260-420`
- Test: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the failing assertions for publish-job asset handling**

```yaml
      - name: Download all CLI archive artifacts
        uses: actions/download-artifact@v8
        with:
          pattern: "cli-*"
          merge-multiple: true
          path: /tmp/cli-archives

      - name: Rename CLI archives to publish version
        if: steps.check.outputs.skip != 'true'
        run: |
          V="${{ steps.ver.outputs.new }}"
          for f in /tmp/cli-archives/*; do
            platform=$(basename "$f" | sed -E 's/^hashjunkie-cli-(0\.0\.0-dev|0\.0\.0-test)-//')
            mv "$f" "/tmp/cli-archives/hashjunkie-cli-${V}-${platform}"
          done
```

- [ ] **Step 2: Run workflow validation to verify it fails before publish logic is added**

Run: `python3 - <<'PY'\nfrom pathlib import Path\ntext = Path('.github/workflows/ci.yml').read_text()\nfor needle in ['Download all CLI archive artifacts', 'Rename CLI archives to publish version', 'Create GitHub Release', 'Upload CLI archives to release']:\n    assert needle in text, needle\nPY`
Expected: FAIL before publish-job CLI release steps exist.

- [ ] **Step 3: Write the minimal implementation**

```yaml
      - name: Download all CLI archive artifacts
        if: steps.check.outputs.skip != 'true'
        uses: actions/download-artifact@v8
        with:
          pattern: "cli-*"
          merge-multiple: true
          path: /tmp/cli-archives

      - name: Rename CLI archives to publish version
        if: steps.check.outputs.skip != 'true'
        run: |
          set -e
          V="${{ steps.ver.outputs.new }}"
          shopt -s nullglob
          for f in /tmp/cli-archives/*; do
            base=$(basename "$f")
            renamed=$(printf '%s\n' "$base" | sed -E "s/^hashjunkie-cli-(0\.0\.0-dev|0\.0\.0-test)-/hashjunkie-cli-${V}-/")
            mv "$f" "/tmp/cli-archives/$renamed"
          done

      - name: Tag the release
        if: steps.check.outputs.skip != 'true'
        run: |
          git config user.name  "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git tag "v${{ steps.ver.outputs.new }}"
          git push origin "v${{ steps.ver.outputs.new }}"

      - name: Create GitHub Release
        if: steps.check.outputs.skip != 'true'
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          gh release create "v${{ steps.ver.outputs.new }}" \
            --repo "${{ github.repository }}" \
            --title "v${{ steps.ver.outputs.new }}" \
            --notes "CLI binaries and npm packages for v${{ steps.ver.outputs.new }}" \
            || gh release edit "v${{ steps.ver.outputs.new }}" \
              --repo "${{ github.repository }}" \
              --title "v${{ steps.ver.outputs.new }}"

      - name: Upload CLI archives to release
        if: steps.check.outputs.skip != 'true'
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          gh release upload "v${{ steps.ver.outputs.new }}" /tmp/cli-archives/* \
            --repo "${{ github.repository }}" \
            --clobber
```

- [ ] **Step 4: Run validation to verify it passes**

Run: `python3 - <<'PY'\nfrom pathlib import Path\ntext = Path('.github/workflows/ci.yml').read_text()\nfor needle in ['Download all CLI archive artifacts', 'Rename CLI archives to publish version', 'gh release create', 'gh release upload']:\n    assert needle in text, needle\nPY`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: publish cli archives to github releases"
```

### Task 4: Document the release assets and verify with local tooling plus `act`

**Files:**
- Modify: `README.md`
- Modify: `.github/workflows/ci.yml`
- Test: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the failing documentation and verification expectations**

```md
Download the latest binary archive from Releases:

- `hashjunkie-cli-{version}-linux-x64-gnu.tar.xz`
- `hashjunkie-cli-{version}-linux-arm64-gnu.tar.xz`
- `hashjunkie-cli-{version}-darwin-x64.tar.xz`
- `hashjunkie-cli-{version}-darwin-arm64.tar.xz`
- `hashjunkie-cli-{version}-win32-x64-msvc.zip`
```

- [ ] **Step 2: Run validation to verify the README is not updated yet**

Run: `python3 - <<'PY'\nfrom pathlib import Path\ntext = Path('README.md').read_text()\nassert 'hashjunkie-cli-{version}-linux-x64-gnu.tar.xz' in text\nassert 'hashjunkie-cli-{version}-win32-x64-msvc.zip' in text\nPY`
Expected: FAIL before the README is updated.

- [ ] **Step 3: Write minimal implementation**

```md
Download the latest binary archive from [Releases](https://github.com/Tuxie/HashJunkie/releases):

- Linux x64: `hashjunkie-cli-{version}-linux-x64-gnu.tar.xz`
- Linux arm64: `hashjunkie-cli-{version}-linux-arm64-gnu.tar.xz`
- macOS x64: `hashjunkie-cli-{version}-darwin-x64.tar.xz`
- macOS arm64: `hashjunkie-cli-{version}-darwin-arm64.tar.xz`
- Windows x64: `hashjunkie-cli-{version}-win32-x64-msvc.zip`
```

- [ ] **Step 4: Run verification commands**

Run: `python3 - <<'PY'\nfrom pathlib import Path\ntext = Path('.github/workflows/ci.yml').read_text()\nassert 'gh release upload' in text\nassert 'hashjunkie-cli-' in text\nPY`
Expected: PASS

Run: `python3 - <<'PY'\nfrom pathlib import Path\ntext = Path('README.md').read_text()\nassert 'hashjunkie-cli-{version}-linux-x64-gnu.tar.xz' in text\nassert 'hashjunkie-cli-{version}-win32-x64-msvc.zip' in text\nPY`
Expected: PASS

Run: `act -n push -W .github/workflows/ci.yml`
Expected: best-effort dry-run output showing the workflow parses. If `act` is unavailable or unsupported for parts of the matrix, capture that limitation explicitly and proceed with static validation plus any Linux-compatible `act` coverage available.

- [ ] **Step 5: Commit**

```bash
git add README.md .github/workflows/ci.yml
git commit -m "docs: document cli release archives"
```

## Self-Review

- Spec coverage: release gate expansion, matrix builds, CLI packaging, version coupling, GitHub Release upload, and `act` verification limits are all covered by Tasks 1 through 4.
- Placeholder scan: no `TODO`, `TBD`, or undefined workflow names remain.
- Type consistency: the plan consistently uses `cli-platform`, `cli-archive-ext`, `/tmp/cli-archives`, and `steps.ver.outputs.new` as the version source.
