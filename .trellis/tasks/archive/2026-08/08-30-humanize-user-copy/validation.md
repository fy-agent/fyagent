# Validation evidence

## Scope review

- Audited production V2 user-visible strings, leftover renderer locales, root
  README files, and maintained public documentation.
- Preserved backend DTOs, write/readback behavior, rollback behavior, security
  boundaries, legal text, release facts, and historical records.
- Added `.trellis/spec/frontend/user-facing-copy.md` and linked it from the
  frontend pre-development checklist and guideline index.
- Added `tests/v2/app/userFacingCopy.test.ts` to reject the reviewed internal
  implementation phrases in production V2 presentation text.

## Successful checks

- `mise run test:i18n` — 3 tests passed.
- `mise run lint:v2` — passed.
- `mise run typecheck:v2` — passed.
- `mise run test:v2` — 59 files, 418 tests passed.
- `mise run test:v2:browser` — 140 tests passed across the configured four
  viewport projects.
- `mise run build:renderer` — passed; six route chunks and standalone preview
  verified.
- `mise run format:check` — passed.
- `git diff --check` — passed.
- `mise run check` — passed, including 1,555 frontend/shared tests, locale
  parity, desktop mock and visual preflight, Rust formatting/check/Clippy,
  2,945 Rust unit tests plus integration suites, task/docs contracts,
  supported-platform checks, and release checks.

## Explicit limits

- The desktop acceptance command is mock-only; no real Windows installer,
  UAC, signing, notarization, or release-candidate HIL was exercised by this
  copy-only task.
- Public documentation that was already technically precise, historical,
  legal, or unrelated to the reviewed copy problems was intentionally left
  unchanged.
