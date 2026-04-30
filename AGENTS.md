# Repository Instructions

## Release Notes

- Every release must have proper release notes before it is published.
- Update `CHANGELOG.md` in the same commit or release-prep series that bumps `VERSION`.
- The `CHANGELOG.md` section for `VERSION` is the source used by GitHub Releases. It must describe the user-facing changes, compatibility notes, performance work, packaging changes, and important fixes since the previous release.
- Do not publish a release whose notes are empty, generic, or just the latest commit subject.
- Before release, run `node scripts/release-notes.mjs "$(node scripts/version-sync.mjs print)"` and read the output. If it would not be useful to a user deciding whether to upgrade, rewrite it before publishing.

## Commit Messages

- Write commit messages so `git log --oneline <previous-tag>..HEAD` is a useful first draft for release notes.
- Use a specific conventional-commit prefix and a plain-English subject that names the user-visible change or maintenance outcome.
- Good examples:
  - `feat: add CIDv1 IPFS hash support`
  - `fix: match Kubo CIDv0 for large nocopy files`
  - `perf: parallelize Dropbox block hashing`
  - `docs: document hashFile best practices`
  - `ci: publish Homebrew formula from release assets`
- Avoid vague subjects like `update stuff`, `fix tests`, `release changes`, `misc cleanup`, or `ci fixes` unless the exact CI behavior being fixed is named.
- If a commit is release-relevant, its subject should make clear why a release-note writer would care.

