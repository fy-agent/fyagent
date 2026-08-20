# Design

Change `scripts/release/dev-release-eligibility.mjs` formal path:

- `sourceSha` = tag target commit (annotated tag object target, or lightweight ref object SHA).
- Do not `expectEqual(remoteDev.headSha, candidate.sourceSha)`.
- Skip `selectLatestSuccessfulCi` as a hard gate. Frozen output may omit `ciRunId`/`ciRunAttempt` or set them null. Downstream jobs that currently consume those fields must tolerate absence (attest/provenance notes can record tag SHA only).
- `validateRemoteTag`: accept `refObject.type === "tag"` (annotated) or `type === "commit"` (lightweight; then SHA equals sourceSha and `tagObject` is null).

Remote collector `verify-dev-release-remote.mjs` must not fail-closed when CI list is empty.

Publish job still refuses existing draft/published Release for that tag.

Port 0.4.2 notarize-once from `04bf9939` onto this branch (divergent history; cherry-pick file-level, do not merge main).

Speed:

- Release `setup-node` `cache: pnpm`
- Optional `actions/cache` for `~/.cargo/registry` + `~/.cargo/git` keyed on `src-tauri/Cargo.lock`, restore on CI backend + Release native jobs. Never cache `target/`.
- Keep rust-toolchain `cache: false`.

Update specs `github-release-workflow.md` and `github-ci-workflow.md` (fix stale “dev push chain” sentence).
