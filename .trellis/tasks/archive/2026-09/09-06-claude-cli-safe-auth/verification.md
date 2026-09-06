# Verification and delivery boundary

## Isolation

All implementation is on `feat/claude-cli-safe-auth` in the dedicated worktree,
based on `590e845e`. Verification did not change the main checkout, merge,
push, release, log into a real account or rewrite an existing user config.

## Completed validation

| Check                                                                                              | Result and evidence scope                                                                                                                                                                    |
| -------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mise run check`                                                                                   | Passed: main type/format checks, unit/integration/contract suites, current-host Cargo check/Clippy/tests, platform/source inventory, tool/lock/release contracts and desktop mock/preflight. |
| Main Vitest unit suite                                                                             | 184 files; 1,621 passed and one existing skipped case. Later contract runs repeat subsets, not additional unique tests.                                                                      |
| Current-host Cargo tests                                                                           | 3,495 passed across application/helper/integration targets; six existing ignored tests. macOS arm64 evidence only.                                                                           |
| `mise run typecheck:v2` and `mise run lint:v2`                                                     | Passed with strict file-impact/recovery parsers and synchronized ports.                                                                                                                      |
| `mise run test:v2`                                                                                 | 85 files, 561 passed. A memory draft-copy test initially failed under full load; its focused 18 cases and the next complete suite passed. No memory code/test was changed.                   |
| `mise run test:v2:browser -- tests/v2-browser/auth.spec.ts tests/v2-browser/agents-models.spec.ts` | 80 browser cases passed across the four configured viewport sizes. Mock/native-boundary interaction evidence only.                                                                           |
| Production bootstrap                                                                               | Seven lazy pages booted without initialization errors; existing route-graph/startup-size budgets passed with 635,257 initial JavaScript bytes. No budget increase.                           |
| Trellis context                                                                                    | Parent and both children validate with focused spec/research manifests; no truncated oversized owner.                                                                                        |

The first classic-schema build exceeded the startup budget. The final schemas
reuse the locked `zod/mini` API, not a new dependency, extra bootstrap import or
relaxed budget. Platform structure digests were refreshed after branch review;
removed Unix chmod allowances correspond only to production code moved into
the common private writer. The test assertions remain covered by their own
allowances and negative fixtures.

## Isolated official-package install

The macOS arm64 smoke verified Tencent mirror metadata for root and native
optional packages against the compiled manifest, installed into a temporary
HOME/global prefix/cache, and executed only `claude --version`. The executable
returned `2.1.261 (Claude Code)` with exit 0. Root and resolved optional-package
versions matched. Temporary home, prefix and cache were removed. No login,
inference, global npmrc or existing Claude installation was touched.

The smoke reproduced npm 12's blocked postinstall behavior without the narrow
script allowance. The final smoke uses the product's conditional official-
package allowance and checks the real executable, not merely npm's exit code.

## Safety review

- Auth preview/cancel performs no vendor write. Confirmation binds request,
  account, revision and native paths, expires and consumes once. OAuth
  completion stores the credential only; connection requires confirmation.
- Codex account changes write auth only and preserve config bytes. Explicit
  request-source selection preserves auth and unowned TOML/comments; missing
  third-party setup fails before config/catalog mutation.
- The common file owner writes a private exact preimage before replacement,
  retains one backup across scoped internal writes and restores only with
  matching path/receipt/pre/postimage evidence. Regression cases cover backup
  failure, first creation/deletion, repeated writes, corrupt backups, symlink
  leaves, stale receipts and later external changes.
- Disclosure/recovery controls expose only native-owned display paths, not
  credentials or digests. First-creation undo explicitly confirms deletion;
  historical backups without receipts remain manual recovery only.
- CLI policy, helper actions, package/registry/architecture, owner/prefix,
  scoped registry overrides, ACL closure and executable readback are tested.
  Existing install owners are not silently converted. A default-writer and
  bypass-owner architecture gate protects future modifications.

## Root cause and prevention review

The configuration loss was a cross-layer contract/implicit-assumption defect:
Auth projection called the Provider snapshot writer, conflating account
identity with request routing. Protecting only Quick Setup could not protect
that separate path. The fix removes the coupling and tests exact config-byte
preservation for account projection and targeted patching for source changes.

The broader recovery gap was a missing common safety contract: atomic rename
prevented partial files but did not make a logically wrong write reversible.
Per-feature backup calls also missed direct auth writers and could rotate to
an internal intermediate state. The shared default writer, scoped first
preimage, private backup/receipt and guarded restore close those paths; the
architecture gate constrains bypass owners. Codex and OpenCode consumer tests,
UI cancel/confirm tests and the focused code specs preserve these lessons.

## Native/runtime evidence limits

Windows helper install/update code and its closed protocol are included, but
native Windows execution, UAC/helper/signer interaction and a release candidate
were not run here. Elevated-parent refusal remains; there is no elevated npm
or arbitrary command fallback. Windows CLI authentication still requires the
separately reviewed ordinary-user auth boundary and is not claimed verified.

Real vendor OAuth, renewal, inference and external-process hot reload were not
tested with user credentials. File undo does not revoke server authorization,
delete saved Provider/account records or prove software pickup. The mechanism
guarantees per-file atomic replacement plus guarded backup recovery, not
filesystem-wide ACID or universal synchronization with external editors.
Existing protected WorkBuddy/Qoder transactions retain their own authority.

## Prearchive and commits

All three full prearchive gates passed with their exact active-task path:
`09-06-reversible-config-auth`, `09-06-claude-code-cli` and
`09-06-claude-cli-safe-auth`. Each ran `mise run check:prearchive
--exclude-active-task .trellis/tasks/<task>` under the matching session context
and exited 0. Work commits precede archive and journal bookkeeping. No remote
push or main-checkout integration is part of this delivery.
