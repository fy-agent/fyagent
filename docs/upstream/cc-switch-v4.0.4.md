# CC Switch v4.0.4 integration ledger

## Status

Draft public integration, 2026-10-10. The migration and current FyAgent main have been reconciled; the final iteration changes are a separate child commit. Final runtime acceptance and public CI are still in progress. This ledger does not claim a merged PR or a released build.

## Immutable source identity

| Item | Identity |
| --- | --- |
| Upstream repository | `https://github.com/farion1231/cc-switch.git` |
| Annotated tag | `v4.0.4` |
| Full tag-object SHA | `7e3be0a6385873987a1cb77123d76ccaef298532` |
| Full peeled commit SHA | `a29a4f3868e9c6ffc2212957cfa66a718642e344` |
| Original FyAgent baseline | `7097d86a3a98d86cb79d4c0332c8fadb7aa4960d` |
| Original upstream merge base | `43e1d99084ed9b2f5dc252fd35c5adaf29d6876e` |
| Original upstream merge | `7586d0e1cc6c7d104592d5809ff12d493f570045` |
| Original merge parent order | FyAgent baseline first; upstream peeled commit second |
| Public migration head (#360) | `263c974e387760ae35a5f1ee0f02d7b9355cdc9c` |
| Current main baseline | `6757da454aa97a3f2f4c3f96aac6dbc72560da2e` |
| Main/migration merge base | `5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798` |
| Public semantic integration | `c065c030840584829fe3628295fb298447f02b31` |
| Public integration parent order | Current main baseline first; public migration head second |

The tag-object and peeled identities were read from the official upstream repository's tag refs on 2026-10-10. Both upstream merge ancestry and the peeled commit remain reachable through #360 and the public semantic integration. The public branch does not squash, replay or reconstruct upstream history. Repository roles remain governed by [Upstream Synchronization](../../.trellis/spec/backend/upstream-sync.md); this source check did not add a remote or broaden push access.

## Semantic integration

- Retain current Windows installation behavior, installation-location labels and user documentation from main.
- Keep the migrated Gemini field-level writer. Existing invalid, empty or whitespace-only settings are rejected before live-file writes; a missing settings file can be created. Preserve MCP and unrelated user fields. The owned settings writer retains the UTF-8/BOM parsing boundary.
- Align main's CLI install regression with the migrated preflight action label without changing the action behavior.
- Reconcile the platform inventory entry affected by the main installation change. Other iteration-specific source changes and their review seals belong to the following commit.

## Iteration scope and retained boundaries

The following child commit supplies the aggregation UI/runtime wiring, account-backed provider handling, account quota display/manual refresh, same-account reauthentication fix and concise account/connection summaries. These are subsequent FyAgent iteration changes, not reconstructed upstream commits.

Keep FyAgent product identities, schema v27, data isolation, account credential ownership and the repository's license/attribution boundaries. No upstream updater, partner or sponsorship surface is enabled by this integration. Pi/Mcode are not claimed as completed UI acceptance in this iteration. Existing legacy source or backend code alone is not a complete feature claim.

## Validation boundary

The semantic integration's parent order, upstream ancestry, conflict resolution and source inventory identities were checked statically. The final iteration source is carried without code/build/lock/test changes from the frozen implementation snapshot; only the documented public task/provenance/validation text differs.

Earlier isolated macOS VM candidates exercised the core aggregation request and recovery flow. Final Rust checks, Auth renderer regressions and the live quota-refresh acceptance are in progress. Native Windows runtime, installer/signing, public CI, PR merge and release are separate evidence scopes and are not established by these earlier VM results.

See [iteration scope and validation](../fyagent/planning/iteration-0411-public-validation-2026-10-10.md) for the current public status. The earlier October 8 worktree-only/Clippy-blocked notes are superseded by this integration status; they are not the final candidate's acceptance result.
