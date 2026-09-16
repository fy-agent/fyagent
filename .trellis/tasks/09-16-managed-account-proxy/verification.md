# Verification

Candidate reviewed on 2026-09-16 in `<workspace-root>`, branch
`dev/laiyongjie`, on the current `darwin-arm64` host. This is a local
implementation task; remote push, PR merge and release publication are not
part of this acceptance.

## Reference and scope verification

The supplied `cherry-studio-2.0.14.zip` was reopened during final review and its
SHA-256 independently matched
`b69bc513d97c4fced35a05c4ac5dbca299cc56be8d13636c061d967453c14fbe`.
OAuth runtime, token-store, Codex/Grok request adapters and gateway admission
were compared with the actual snapshot, not a changing upstream branch.
Primary vendor references and the reuse/licensing decision remain in
`research/cherry-comparison.md`.

The resulting implementation reuses FyAgent's existing credential/vault,
Provider transaction, listener, protocol conversion and configuration owners.
Final `git diff --exit-code` inspection confirmed no changes in native consumer
projection, native login-session implementation, SecretRef backend or dependency
manifests/lockfiles. Shared proxy-facing services changed only at the documented
binding, refresh, forwarding and local observation boundaries.

## Accepted current-host checks

| Check | Observed result |
| --- | --- |
| `TRELLIS_CONTEXT_ID=cherry-proxy-20260916 mise run check:prearchive --exclude-active-task .trellis/tasks/09-16-managed-account-proxy` | Passed, exit 0. Includes environment, type/lint/format, unit, Rust, platform and release-contract gates. |
| Unit suite within the complete gate | 189 files passed; 1711 tests passed and one existing test skipped. |
| Rust formatting, compilation, Clippy and tests within the complete gate | All completed successfully. The library runner discovered 3267 tests; the complete gate also ran integration targets. No exact combined count is inferred from truncated console output. |
| Release contract suite within the complete gate | 34 files passed; 619 tests passed and one existing test skipped. This overlaps the unit suite and is not an additional unique-test total. |
| Desktop mock and native-fetch contract checks | Seven desktop mock tests and four native-fetch tests passed. They are not real desktop acceptance. |
| `task.py validate .trellis/tasks/09-16-managed-account-proxy` | Both five-entry context manifests passed, without required-SPEC truncation. |
| `git diff --check` | Passed. |

## Browser validation

`mise run test:browser` completed with exit 0: the production renderer build
and route-chunk verification passed, all three production-boot checks passed,
and all 631 configured browser cases passed across the Chromium viewport and
WebKit projects. There were no failed or retried cases in the final result.

The subscription cases exercise xAI selection and Claude application, Codex
saved-source preview/application, closed account-recovery failures and OpenAI
manual model binding to Grok Build. This validates the real renderer with
synthetic IPC, not real native writes or provider authorization. The full suite
also retains the surrounding account, API-key, MCP and prompt interactions.

## Acceptance traceability

| Requirement | Executable evidence and assertions |
| --- | --- |
| AC1: native/direct path preserved | `subscription_tests.rs` rejects wrong-purpose credentials; both provider roundtrips preserve native Codex/Grok auth bytes, unrelated settings and MCP. A native Codex login rotated during proxy use survives proxy restoration. |
| AC2: account to target binding | `subscription_tests.rs` exercises OpenAI/xAI to Claude Code, Codex and Grok Build. Codex binding does not change the active config before the existing Change Plan is applied. Renderer and Port tests cover exact payloads, ACL registration, target identity, authority rereads and old-command compatibility. |
| AC3: actual local transport | Real loopback listeners exercise Messages and Responses, JSON and SSE, selected-account Bearer/routing headers, upstream model mapping, function arguments, empty tool-result output and terminal events. Test-only I/O substitution first asserts the fixed official origin/path. |
| AC4: refresh and isolation | `proxy_refresh_tests.rs` covers concurrent expiry/401 renewal, terminal versus transient failures, deletion, relogin, revocation, generation drift and native ownership changes. `subscription_transport_tests.rs` proves one replay, terminal second 401 and no default-account or balance failover. Provider refresh tests distinguish exact invalid-grant errors from malformed, throttled and server responses. |
| AC5: runtime truth and compensation | Listener observation distinguishes saved, stopped, adopted, contended, malformed and unadopted states; deliberately disagreeing database/local Provider selections must follow the forwarding authority. Port-conflict/concurrent-target tests preserve committed listeners; incomplete rollback reports unknown. |
| AC6: SPEC and lifecycle | Reviewed executable contracts in backend managed-auth/proxy-runtime/local-proxy-pipeline and frontend models/subscriptions. Context manifests pass validation. Work, archive and journal commits provide the final lifecycle record. |

## Bounded gate repairs

Two pre-existing bookkeeping defects were corrected, as detailed in `review.md`:
the already-changed helper-priority test needed its exact source seal refreshed,
and the prior FDE task's archived verification header needed its workstation
home path replaced with `<workspace-root>`. Neither repair changes helper, FDE,
platform scanner or exception policy behavior.

## Evidence limits

All grants and upstream inference responses in these tests are synthetic.
Real loopback I/O proves the local transport and configuration integration, not
subscription entitlement, quota, every model's availability or successful
live authorization. No real subscription was used for unattended paid inference.
Signed desktop execution, external CLI runtime acceptance and Windows-native
behavior were not exercised. The existing trusted loopback boundary remains;
this task does not add a public gateway or per-client key-management product.

The post-archive `mise run check:contracts` result is recorded in the closure
journal, after the task has moved out of the active-task tree.
