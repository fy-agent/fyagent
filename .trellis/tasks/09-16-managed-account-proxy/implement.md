# Implementation plan

1. [x] Inspect supplied source, existing owners and primary official configuration references.
2. [x] Record boundary, requirements and design before product edits.
3. [x] Implement provider-specific request normalization and Responses routing; add protocol tests.
4. [x] Generalize proxy account binding to OpenAI/xAI and Grok Build using existing transactions; preserve xAI command compatibility and Codex Change Plan.
5. [x] Harden proxy-only refresh and bounded 401 retry; add deterministic refresh/race/failure tests.
6. [x] Correct proxy connection/runtime evidence and minimal renderer/Port integration.
7. [x] Execute focused native + renderer + listener integration tests; review native-path preservation, secret-negative output, rollback and streaming tool semantics.
8. [x] Update smallest owning SPECs (managed-auth, proxy-runtime, local-proxy-pipeline, frontend subscription) before archive.
9. [x] Run `TRELLIS_CONTEXT_ID=cherry-proxy-20260916 mise run check:prearchive --exclude-active-task .trellis/tasks/09-16-managed-account-proxy`; fix task regressions without unrelated refactors.

## Closure protocol

Commit scoped code/SPEC/task artifacts, archive via the task script, then run
canonical post-archive contracts without an exclusion and record the journal.
The task's final `status` and Git history establish archival; the closure journal
records the work commit and post-archive check result. This post-implementation
bookkeeping is separate from the completed engineering checklist above.
Accepted checks and evidence limits are recorded in `verification.md`.

## Validation

- Native focused: `mise run rust:test <filter>` (one filter per invocation); existing managed_auth/subscription, provider, proxy/transform and native consumer tests.
- Frontend focused: canonical unit task filtered to affected Models, managed-auth and Port tests; `mise run typecheck` and `mise run lint`.
- Full current-host: `mise run check:prearchive` as above; after archive `mise run check:contracts` with no exclusion.
- Real network tests use only local ephemeral listeners and fake secrets; fixture verifies official origin/path before redirecting network I/O.
- Do not run unattended requests using the user's real subscription or claim native Windows/runtime acceptance from portable mocks.

## Review checklist

- No upstream token in DTO/provider/config/log/diff.
- No native credential/session transferred into proxy purposes.
- No fallback from failed subscription to another account/paid source.
- One listener owner; no unrelated target stopped during compensation.
- Full tool call + tool result + final stream events, not just HTTP 200.
- Saved draft, active route and live upstream availability reported separately.
