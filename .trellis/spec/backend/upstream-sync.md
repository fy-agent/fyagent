# CC Switch Upstream Synchronization Contract

## 1. Scope / Trigger

Read this contract before fetching, auditing, merging, or documenting a CC
Switch upstream release. It owns remote roles, immutable source verification,
ancestry-preserving integration, semantic conflict precedence, FyAgent
identity/licensing/data invariants, and the boundary between an upstream merge
and later downstream modernization.

FyAgent versions are independent from upstream versions. The application
version comes only from
[Application Version and Installer Assets](./fyagent-version-contract.md).

## 2. Authorities and signatures

Remote roles are fixed:

```text
origin fetch/push = https://github.com/fy-agent/fyagent.git
upstream fetch    = https://github.com/farion1231/cc-switch.git
upstream push     = DISABLED
```

The exact canonical URLs are repository/governance inputs and must match the
current GitHub identity used by CI/Release. A redirect from a former owner is
historical continuity, not current authority.

Each approved integration has a provenance ledger under `docs/upstream/**`
that records at least:

```text
upstream repository URL
annotated tag name
full tag-object SHA
full peeled commit SHA
FyAgent baseline and merge base
resulting two-parent merge commit and parent order
```

The existing CC Switch import ledger is
[`docs/upstream/cc-switch-v3.19.2.md`](../../../docs/upstream/cc-switch-v3.19.2.md).
That ledger, CHANGELOG, and Git history own the one-time values; this contract
defines how every integration is verified.

Canonical preparation is parameterized by the approved tag:

```bash
git remote get-url origin
git remote get-url upstream
git remote get-url --push upstream
git ls-remote --tags upstream "refs/tags/$TAG" "refs/tags/$TAG^{}"
git fetch --no-tags upstream "refs/tags/$TAG:refs/tags/$TAG"
git merge-base "$FYAGENT_HEAD" "refs/tags/$TAG"
git merge --no-ff --no-commit "refs/tags/$TAG"
```

Repository `upstream:*` tasks may validate/fetch an approved tag and prepare an
uncommitted merge. They never choose conflict resolutions, commit, tag, push,
or change remotes.

## 3. Contracts

### Remote and immutable-source boundary

- `origin` is the only normal push target. `upstream` must have no usable push
  URL; automation must not silently repair or broaden it.
- Verify an annotated tag by full tag-object and peeled-commit identity against
  the approved provenance/task decision. A name, short SHA, release page, or
  mutable branch is insufficient.
- Fetch only the approved ref. Emergency commit picks require a separate
  recorded decision and are not treated as a tagged release integration.
- Require a clean/non-overlapping worktree and create a read-only recovery ref
  before mutation. Never reset, stash, or overwrite unrelated work implicitly.

### Merge and conflict boundary

- The integration commit is an explicit two-parent merge. Do not squash,
  rebase, replay, or reconstruct upstream history.
- That merge contains upstream content plus only semantic conflict resolutions
  needed to make the upstream state a valid FyAgent state. Tooling, CI/Release,
  dependency cleanup, product-version changes, and broad modernization belong
  in later, independently reviewable commits.
- Resolve each conflict in this order:
  1. preserve FyAgent identity, user data, licensing, and security boundaries;
  2. retain shared upstream correctness/security/compatibility/performance fixes;
  3. keep FyAgent-only behavior unless an explicit product decision removes it;
  4. leave unrelated engineering modernization to its owning task.
- Repository-wide `ours`/`theirs` is forbidden. Record meaningful conflict
  decisions and test the combined behavior.

### FyAgent invariants

- Preserve the product/bundle/protocol/data/environment identities owned by
  [Application Identity](./application-identity.md) and
  [Application Version and Installer Assets](./fyagent-version-contract.md).
- Preserve the current database authority (`SCHEMA_VERSION` and migrations) as
  implemented in the database owner. An upstream merge must not silently
  revert, skip, or bump it; a schema change requires a separate migration
  decision and tests.
- Preserve the repository's mixed licensing/provenance model. Upstream-derived
  code and attribution retain their upstream license; FyAgent-owned changes
  retain the repository's stated license boundary.
- Do not import sponsorship, partner, affiliate/tracking, or upstream
  distribution-channel product claims. Required factual attribution and
  historical source links are preserved.
- External tool compatibility may remain optional, but FyAgent installation,
  startup, and core runtime must not depend on development tooling such as mise.
- Upstream release-note files may enter the ancestry merge. Product-facing
  release notes are cleaned in a later documentation commit and must not be
  rebranded as FyAgent releases.

### Maintained Native Fetch delta

FyAgent intentionally removes the upstream test-only `cross-fetch` layer and
uses the Node runtime selected by `.node-version` for Fetch globals. This is a
downstream modernization commit, not a conflict resolution inside the
two-parent merge.

The delta remains valid only while:

- the native Fetch -> MSW -> Tauri mock suite covers JSON success, non-2xx text
  errors, empty responses, and cross-realm Headers behavior;
- `cross-fetch`, its obsolete `node-fetch`/URL chain, warning suppression, and
  unknown DEP0040 reverse paths are absent;
- `scripts/tasks/dep0040-check.mjs`, its focused tests, `pnpm-lock.yaml`, and
  `pnpm why --json` agree on the reviewed remaining dependency origins.

The executable checker/tests own exact package versions and ancestor suffixes;
the prose contract does not duplicate those volatile values. A dependency
update that changes the watched graph requires a new reverse-path review. A
pending-deprecation probe supplements but never replaces lock/graph analysis.
Prefer removing this downstream delta when a reviewed future upstream no longer
needs the compatibility dependency.

### Provenance handoff

After a successful integration, update the per-release ledger with the exact
source identities, ancestry, conflict summary, validation, and merge commit.
The spec may link that ledger but must not copy its one-time SHAs into the
long-term contract.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| `upstream` has a usable push URL | Stop before fetch/merge and restore the reviewed read-only role explicitly. |
| `origin` is a former-owner redirect instead of canonical authority | Correct it before integration; redirect continuity is not current authority. |
| Tag object or peeled commit differs from the approved full identity | Stop; do not merge a similarly named tag. |
| Worktree changes overlap the integration | Stop and preserve them; no implicit stash/reset. |
| Merge changes FyAgent product/version/data/license identity | Reject the resolution. |
| Upstream security/correctness fix conflicts with naming | Port the behavior under FyAgent identity and retain regression coverage. |
| Conflict markers or unmerged index entries remain | Do not commit. |
| Result is not a two-parent merge or approved source is not its ancestor | Reject the integration and do not modernize on top. |
| Database authority drifts without an approved migration | NO-GO; separate migration/design required. |
| Partner/sponsorship/tracking metadata enters active product surfaces | Remove it and rerun identity/promotion tests. |
| Only local/static tests exist for native artifacts | Report native evidence as pending. |

## 5. Good / Base / Bad Cases

- **Good:** full immutable identities match the ledger; one two-parent merge
  preserves upstream ancestry and FyAgent invariants; later modernization is
  separate and reviewable.
- **Base:** historical upstream names, issue links, copyrights, and license
  notices remain factual source/provenance evidence while current product
  surfaces use FyAgent identity.
- **Bad:** merge a branch, accept a short SHA, enable upstream push, choose one
  conflict side globally, copy upstream version/partner copy, or squash the
  ancestry into a feature commit.

## 6. Tests Required

- Verify remote fetch/push roles, full tag/peeled identities from the approved
  ledger, merge base, parent count/order, and ancestor relation.
- Scan the index/worktree for unresolved conflicts before commit.
- Run application identity, licensing/promotion, version, database/migration,
  renderer, Rust, and security tests affected by conflict resolutions.
- Run the DEP0040 checker plus focused native Fetch/MSW/Tauri behavior suite
  while the maintained downstream delta exists.
- Record local portable validation separately from matching native platform,
  architecture, installer, and Release evidence.
- Validate that the provenance ledger matches the fetched immutable tag
  identity and resulting Git graph after the merge, and is the only long-term
  document containing that integration's exact SHAs.

## 7. Wrong vs Correct

Wrong:

```bash
git merge upstream/main
git checkout --theirs .
git commit
git rebase -i main
```

Correct:

```bash
git ls-remote --tags upstream "refs/tags/$TAG" "refs/tags/$TAG^{}"
git fetch --no-tags upstream "refs/tags/$TAG:refs/tags/$TAG"
git merge --no-ff --no-commit "refs/tags/$TAG"
# Resolve and test each semantic conflict.
git commit -m "merge(upstream): merge CC Switch $TAG"
git merge-base --is-ancestor "refs/tags/$TAG" HEAD
```
