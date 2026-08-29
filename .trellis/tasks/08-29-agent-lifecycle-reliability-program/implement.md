# Implement — Program Orchestration

## Purpose

This parent task coordinates the five child tasks. It is not the direct coding target.

## Ordered Checklist

### 1. Baseline and issue alignment

- [ ] Rebase/merge each child implementation branch from the then-current `main` before work starts.
- [ ] Recheck #31, #47, #101 and #141 for requirement changes; keep #68 and #71 as external dependencies, not duplicated scope.
- [ ] Record the exact main SHA and shipped-platform evidence used by each child.

### 2. Stage 1 gate

- [ ] Complete `08-29-agent-install-target-authority` first.
- [ ] Review candidate identity, revision, scope, owner, ambiguity, privacy and renderer request closure.
- [ ] Confirm Stage 2/3 cannot select or update an installation without the Stage 1 contract.

### 3. Platform implementation gates

- [ ] Complete macOS and Windows children independently.
- [ ] Require platform-neutral contract tests plus native HIL evidence; one platform cannot substitute for the other.
- [ ] Verify install failure preserves the previously usable candidate and leaves no undeclared duplicate.

### 4. Auth gate

- [ ] Complete the Auth session child with separate stage semantics.
- [ ] Verify every adapter's success authority and unsupported/unknown behavior.
- [ ] Confirm no vendor credential file, token, URL or device code enters DTOs, logs, DOM or Trellis evidence.

### 5. Frontend gate

- [ ] Complete CSS-first selection and Tabs semantics before removing the old keep-alive behavior.
- [ ] Add route-level lazy loading, explicit draft ownership and hidden-query controls.
- [ ] Revalidate applicable #141 UX findings at supported viewports and input methods.

### 6. Integration review

- [ ] Run task validation for all six task directories.
- [ ] Run full frontend/backend checks and architecture/security tests.
- [ ] Run installed-app macOS and Windows UAT on fresh candidates.
- [ ] Produce a matrix mapping every cross-child acceptance criterion to test/HIL evidence.
- [ ] Update #141 findings as `fixed`, `still applies` or `obsolete` with exact SHA/evidence.

## Validation Commands

Use the repository-owned tasks rather than ad hoc toolchains:

```bash
mise run check
mise run test:v2:browser
mise run rust:test
```

Children may add focused commands from their own `implement.md`. Native HIL is recorded separately and must not be replaced by portable test success.

## Stop / Rollback Conditions

- A child introduces renderer-controlled paths, URLs, commands or installer arguments.
- A multi-install state regresses to first-match selection.
- An update can silently cross user/system/custom scope.
- Auth reports success before authoritative verification.
- A frontend refactor removes visible state when animation or observers are unavailable.
- A shared abstraction creates two policy owners or weakens an existing security/rollback sequence.

On any condition above, return the child to planning, revise its artifacts, and do not start the next dependent stage.
