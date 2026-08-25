# Design — Refine CI change classification and title policy

## 1. Design principles

1. Preserve the current trusted topology: one Required CI workflow, one
   repository-owned classifier, job-level conditional execution, and one stable
   `CI / Required` aggregate.
2. Treat control-plane ownership as an impact relationship, not a blanket risk
   level.
3. Keep the safety bias fail-closed: known narrow authorities may be narrowed;
   unknown paths remain classification failures; CI/toolchain authorities stay
   Full CI where they can invalidate the checking mechanism itself.
4. Avoid a new CI framework or domain unless the current domain model cannot
   express the desired evidence.

## 2. Classification model

The existing six public domains remain unchanged:

```text
contracts
frontend
desktop
backend
windowsNative
docsSpec
```

The implementation should replace the broad generic control-plane bucket with
responsibility-oriented rules inside `scripts/ci/classify-changes.mjs`.

### 2.1 CI authority → Full CI

Paths that define CI scheduling, classification, or Required aggregation remain
`forceFull=true`, including the owning workflow/script/test contracts. This is
the trust root: if these files are wrong, affected-domain selection itself may
be wrong.

### 2.2 Release authority → contracts/release evidence

Release workflow/scripts and release-specific contract tests should select the
contracts owner without automatically selecting product domains. Product-domain
changes in the same diff are added through ordinary union rules.

The first implementation should reuse the existing `contracts` job rather than
adding a seventh public domain. Existing release-check/task contracts already
provide the intended release validation surface.

### 2.3 Repository task/config authority

Remove blanket `scripts/tasks/** → Full`. Classify known task families by their
actual owner. Where a task truly controls global build/tooling behavior, retain
Full CI. Do not guess a narrow owner for ambiguous global task/config files.

### 2.4 GitHub metadata/workflows

Remove blanket `.github/** → Full` and classify:

- Required CI authority as Full;
- Release workflow as contracts/release evidence;
- repository metadata/templates/labeler as lightweight contracts/docs as
  applicable;
- any other workflow according to the subsystem it controls, conservatively.

### 2.5 Toolchain and unknown paths

Global Node/Python/Rust/mise toolchain roots remain Full CI in this iteration.
Unknown paths remain `unknownPaths` and cause the CLI to fail. They are not
silently promoted to Full CI because a silent promotion would hide missing
ownership instead of forcing the ownership table to be maintained.

## 3. Classification observability

Keep the stable classifier JSON plan exactly at `domains`, `unknownPaths`, and
`forceFull`, because Required CI validates that schema with exact keys. Add an
optional classifier-owned Markdown summary output derived from the same
path-ownership function so per-path matches/reasons and path-derived Full CI
reasons are observable without widening the machine contract.

The `changes` workflow step should ask the classifier to append that diagnostic
summary to `$GITHUB_STEP_SUMMARY`, then append the event-level final plan.
Workflow YAML may format already-produced booleans/event policy, but must not
duplicate path globs or ownership rules.

Manual `workflow_dispatch` forcing should appear as an event-level Full CI
reason in the summary even though path-derived `forceFull` remains classifier
owned.

## 4. Commit and PR title validation

`verify-commit-messages.mjs` continues to own Conventional Commit validation.
Remove `SUBJECT_MAX_LENGTH` and all maximum-length branches from
`isConventionalCommitSubject()` and `validateCommitSubject()`.

The validator continues to enforce:

- non-empty subject/title;
- Conventional Commit type and optional scope syntax;
- merge/revert exceptions;
- GitHub squash PR suffix normalization.

Because PR titles pass through the same validator, removing the cap there
removes both repository-defined limits consistently without creating separate
PR-title policy.

## 5. Label ownership

`.github/labeler.yml` should stop treating every `tests/**` path as frontend.
Use concrete frontend test directories consistent with classifier ownership
(`tests/components/**`, `tests/config/**`, `tests/hooks/**`,
`tests/integration/**`, `tests/lib/**`, `tests/msw/**`, `tests/utils/**`, plus
any current frontend-specific roots proven by repository contents). Do not try
to make labeler a second exact classifier; the goal is to remove the known
misleading blanket mapping.

## 6. Compatibility

- No public application behavior changes.
- No new dependency or runner requirement.
- Existing Required CI check name remains stable.
- Existing six-domain JSON shape remains usable by current workflow/gate logic.
- Branch-push commit convention continues to validate structure but no longer
  rejects valid long subjects.

## 7. Rollback

The change is repository-policy-only. If validation exposes unsafe narrowing,
rollback is to restore the prior ownership rule for the affected path family,
not to replace the whole classifier. The Required CI topology and aggregate
remain untouched, which keeps rollback local.
