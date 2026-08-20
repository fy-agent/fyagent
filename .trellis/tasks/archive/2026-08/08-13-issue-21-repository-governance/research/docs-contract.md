# Documentation contract research

## Scope and baseline

- Source requirement: GitHub Issue #21 plus the maintainer addendum dated
  2026-08-13.
- Verified repository baseline:
  `origin/main@9be29455a081d3ff0bc761465672727d09ffb3e6`.
- The root public README contract is deliberately three-way:
  `README.md`, `README_EN.md`, and `README_JA.md`.
- `tests/currentDocsContract.test.ts` is already part of the CI-safe contract
  suite through `scripts/tasks/release-check.mjs`; it is the correct existing
  regression owner for public documentation invariants.

## Workstation path findings

Six tracked Markdown lines contain real Windows user-profile paths:

1. `docs/fyagent/audits/vibekey-to-fyagent-capability-gap.md:37`:
   submission draft -> `<local-submission-draft>`.
2. The same file at line 38: project archive ->
   `<local-vibekey-project-archive>`.
3. The same file at line 39: driver checkout -> `<local-vibekey-driver>`.
4. `docs/fyagent/marketing/vibekey-reference-audit.md:27`: project archive ->
   `<local-vibekey-project-archive>`.
5. The same file at line 28: driver checkout -> `<local-vibekey-driver>`.
6. `.trellis/tasks/archive/2026-07/07-29-fyagent-v1-integration/implement.md:41`:
   source image -> `<local-fyagent-source-image>`.

The sixth line is archived process evidence, but it is still public Markdown
in the current Git tree and therefore falls under the explicit Issue #21
acceptance criterion. The permitted edit is a narrow privacy redaction only:
do not reconstruct the task, change its conclusions, or alter its hash and
image evidence.

Known generic examples that must remain valid include localized
`C:\Users\<...>\...` examples in the Codex history guide and FAQ manuals, and
the explicit `example/profile/...` privacy examples in the screenshot
card. System paths such as `C:\ProgramData` and `C:\Program Files`, test roots
such as `D:\FyAgent-Acceptance`, `%USERPROFILE%`, and `~` are not personal
user-profile disclosures.

## Existing README facts and gaps

- Goal/current state: already present and protected by
  `tests/currentDocsContract.test.ts`; retain the vision-versus-current feature
  boundary and release trust limitations.
- Architecture gap: current READMEs say only that data is local and use SQLite
  and atomic writes. Current code supports the concise path
  `React + Vite renderer -> Tauri IPC -> Rust commands/services -> SQLite,
  target-tool configuration, and local proxy`.
- First-checkout gap: READMEs omit `mise run system:check`. The maintained
  sequence in `CONTRIBUTING.md` and
  `docs/fyagent/development/tooling/mise.md` is `mise trust`,
  `mise run bootstrap`, `mise run system:check`, `mise run dev`, with
  `mise >= 2026.8.0`.
- Evidence gap: `mise run check` is the complete current-host gate;
  `CI / Required` is the stable remote merge result; a formal Release requires
  its separate exact-SHA/tag/workflow evidence. A local build is not a formal
  Release asset and none of these static statements prove installer HIL.
- Scope wording: WorkBuddy is an independent top-level configuration entry;
  it should be named without turning the provider-domain list into an
  inaccurate exhaustive list.

## Canonical repository and contribution text

- `.trellis/spec/backend/application-identity.md` fixes the canonical current
  repository at `fy-agent/fyagent`.
- `CONTRIBUTING.md` currently tells all contributors to “Fork and branch” but
  later uses `origin` for the writable FyAgent repository and `upstream` for
  the separate CC Switch fetch-only source. That is correct for a maintainer
  checkout but ambiguous for an external contributor fork.
- The maintained text should distinguish:
  - maintainer checkout: canonical `origin` may be writable;
  - external fork: personal fork is normally `origin`, while the canonical
    FyAgent repository is an additional fetch source;
  - CC Switch maintenance upstream: a separate repository-specific remote and
    task contract, not the same concept as the canonical FyAgent source.
- `.github/CODEOWNERS` has a valid owner mapping, but its comments incorrectly
  claim live Code Owner enforcement. Preserve the mapping and correct only the
  enforcement claim.

## Recommended executable coverage

Extend `tests/currentDocsContract.test.ts` rather than adding a parallel test
framework:

- enumerate every tracked `*.md` file through NUL-delimited `git ls-files`;
- reject a Windows `Users` segment followed by a non-placeholder profile name;
- explicitly prove the localized angle-bracket examples and demo examples are
  accepted;
- assert three-way README architecture, checkout, validation, current-state,
  and WorkBuddy semantics;
- assert canonical-source and branch/PR/CI/squash wording in both halves of
  `CONTRIBUTING.md`;
- assert CODEOWNERS says the mapping is advisory unless live protection is
  configured, without encoding a dynamic GitHub setting as an executable local
  fact.

Focused validation:

```text
mise run test:unit tests/currentDocsContract.test.ts
mise run release:check
mise run check
```
