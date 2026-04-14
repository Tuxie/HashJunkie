# CLI Release Assets Design

## Goal

Extend the GitHub Actions release pipeline so `hashjunkie-cli` binaries are built for the same platform matrix as the native addon packages, packaged into release archives, and published on GitHub Releases using the exact same version as the npm packages published in that workflow run.

## Scope

This change applies to the existing `.github/workflows/ci.yml` workflow.

- Reuse the existing release decision gate and version computation.
- Build CLI binaries for:
  - Linux `x86_64`
  - Linux `aarch64`
  - macOS `x86_64`
  - macOS `aarch64`
  - Windows `x86_64`
- Package CLI binaries as:
  - Linux/macOS: `hashjunkie-cli-{version}-{platform}.tar.xz`
  - Windows: `hashjunkie-cli-{version}-{platform}.zip`
- Package contents must contain the executable named `hashjunkie` on Linux/macOS and `hashjunkie.exe` on Windows.
- Publish those archives to the GitHub Release tagged `v{version}` after npm publishing succeeds.

## Version Coupling

The workflow already computes the publish version in the `publish` job based on:

- latest npm version
- optional manual bump in `npm/hashjunkie/package.json`

That computed version remains the single source of truth.

- CLI archive names must use `steps.ver.outputs.new`.
- GitHub release tag remains `v{steps.ver.outputs.new}`.
- npm packages and CLI archives must never use independently computed versions.

## Architecture

Keep a single workflow instead of introducing a second release workflow.

### Build Stage

The existing `build-native` matrix job becomes responsible for two artifact families per target:

- native addon artifact (`hashjunkie.*.node`) as today
- CLI release archive for the same target

Each matrix entry continues to run on its native OS runner.

### Packaging Stage

Each runner packages its own CLI executable after `cargo build -p hashjunkie-cli --release --target ...`.

- Linux/macOS:
  - source executable: `target/{target}/release/hashjunkie`
  - archive format: `.tar.xz`
  - archive contains a top-level executable file named `hashjunkie`
- Windows:
  - source executable: `target\{target}\release\hashjunkie.exe`
  - archive format: `.zip`
  - archive contains a top-level executable file named `hashjunkie.exe`

The archive filename includes a normalized platform suffix, for example:

- `linux-x64-gnu`
- `linux-arm64-gnu`
- `darwin-x64`
- `darwin-arm64`
- `win32-x64-msvc`

Resulting filenames:

- `hashjunkie-cli-{version}-linux-x64-gnu.tar.xz`
- `hashjunkie-cli-{version}-linux-arm64-gnu.tar.xz`
- `hashjunkie-cli-{version}-darwin-x64.tar.xz`
- `hashjunkie-cli-{version}-darwin-arm64.tar.xz`
- `hashjunkie-cli-{version}-win32-x64-msvc.zip`

### Publish Stage

The `publish` job continues to:

- stamp package versions
- publish platform npm packages
- publish the main npm package
- tag the repo

After npm publishing succeeds, it also:

- downloads the CLI archive artifacts
- creates or updates the GitHub Release for tag `v{version}`
- uploads all CLI archives as release assets

Using a single publish job ensures GitHub Release publication only happens if npm publication succeeded.

## Workflow Data Flow

1. `should-release` decides whether release work should run.
2. Build matrix produces:
   - native addon artifacts
   - per-platform CLI archives
3. `publish` computes the version once.
4. `publish` publishes npm packages using that version.
5. `publish` tags `v{version}`.
6. `publish` creates or updates the corresponding GitHub Release and attaches the CLI archives named with the same version.

## Release Trigger Coverage

The release-relevant path filter must continue to catch artifact-producing changes and expand to include:

- `crates/hashjunkie-cli/src/`
- `crates/hashjunkie-cli/Cargo.toml`
- `.github/workflows/ci.yml`

This keeps CLI and release workflow changes from being skipped by the release gate.

## Verification Strategy

Verification must distinguish between what can be proven locally and what requires GitHub-hosted runners.

### Local Verification

- Validate workflow YAML after edits.
- Run targeted Rust tests if helper scripts or packaging logic are factored into repo code.
- Use `act` for Linux-compatible jobs or focused dry runs where possible.
- Verify archive naming and packaging commands on the local platform where feasible.

### GitHub-Only Verification Limits

`act` cannot fully emulate this workflow because:

- macOS runners are not available in `act`
- Windows runner behavior is not faithfully reproduced
- GitHub Release upload and npm publish steps depend on hosted credentials and release APIs

Therefore the local acceptance bar is:

- successful YAML validation
- successful `act` execution for the Linux-compatible subset, if available
- manual inspection of workflow logic for matrix packaging and publish ordering

The final rollout risk that remains is cross-OS archive creation and hosted release upload behavior.

## Error Handling

- If any platform build fails, release publication must not proceed.
- If npm publication is skipped because the version is already published, CLI release creation must also be skipped.
- If GitHub Release upload fails after npm publication, the workflow should fail clearly so the partial-release condition is visible and can be repaired manually.

## Non-Goals

- No standalone installer packages.
- No checksum/signature generation in this change.
- No separate workflow triggered by tags or releases.
- No expansion beyond the existing native target matrix.
