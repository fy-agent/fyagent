# Implementation plan — Human-readable user copy

## 1. Inventory and review

- [x] Scan production V2 literals and JSX text for internal implementation
      language, generic filler, duplicated loading text, and unclear next steps.
- [x] Scan leftover locale values for the same reviewed patterns.
- [x] Scan root public Markdown and `docs/**`; classify findings as product
      overview, task guidance, technical contract, historical record, or legal
      text before editing.
- [x] Compare the inventory against external plain-language and UX-writing
      guidance and persist the synthesis in the task research file.

## 2. Rewrite the interface

- [x] Rewrite Agent assignment, prompt, install/action, authentication, and
      status copy.
- [x] Rewrite Models preview/apply copy and remove rendered diagnostic event
      sequence metadata.
- [x] Rewrite other clear V2 offenders found by the full scan, including mixed
      internal English/Chinese labels and errors without a next step.
- [x] Update clear leftover locale offenders in all four locales together.

## 3. Rewrite public docs

- [x] Restructure and rewrite `README.md`, `README_EN.md`, and `README_JA.md`
      from one shared content outline.
- [x] Correct stale navigation/setup wording using current production routes.
- [x] Rewrite only audited problem passages in other public Markdown; preserve
      technical, historical, security, legal, and attribution facts.
- [x] Run link/path and formatting checks after the rewrite.

## 4. Persist and test the contract

- [x] Add `.trellis/spec/frontend/user-facing-copy.md` and index it.
- [x] Add a focused production V2 copy regression test.
- [x] Update unit and browser tests coupled to changed visible copy.

## 5. Validation

- [x] Run focused tests while editing.
- [x] Run `mise run lint:v2`.
- [x] Run `mise run typecheck:v2`.
- [x] Run `mise run test:v2`.
- [x] Run `mise run test:v2:browser` when the environment supports it.
- [x] Run `mise run build:renderer`.
- [x] Run applicable documentation and formatting checks.
- [x] Run `mise run check` as the final local gate.
- [x] Review the final diff for behavior changes, unsupported claims, and
      accidental edits to archived/internal/legal content.

## 6. Delivery

- [x] Update task/spec evidence for archival.
- [x] Commit product, SPEC, documentation, and regression-test changes.
- [ ] Push `dev/laiyongjie` after archival, as requested.
- [ ] Open a PR to `main`, wait for required checks, merge it, and verify the
      merged `main` commit.
