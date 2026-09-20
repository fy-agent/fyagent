# Validation and handoff evidence

## Status

Implementation ready for parent integration review. Final native validation is
**pending**, by the parent's explicit instruction to avoid duplicate Rust builds
and preserve the 0.4.6 release acceptance quiet window. This is not acceptance,
release, native OS-keyring HIL, Windows UAT, or issue closure.

## Executed checks

| Command | Observed result | Scope |
| --- | --- | --- |
| `mise run env:check -- --json` before bootstrap | Failed: missing `smol-toml` | Fresh worktree dependencies |
| `mise run bootstrap` | Passed | Frozen dependency setup, environment/task validation |
| First `mise run rust:check` | Failed: three managed_proxy readback calls lacked new DB argument | Fixed in this package |
| Second `mise run rust:check` | Passed | Earlier native source snapshot, before later export/plan/edge refinements |
| `mise run rust:test services::provider::credentials` | **8 passed, 1 failed** | Earlier 9-case snapshot; failure preserved below |
| `mise run typecheck` | Passed | Renderer implementation |
| `mise run test:unit tests/renderer/pages/models/quickSetup.test.ts tests/renderer/pages/models/Page.test.tsx` | **58 passed in 2 files** | Includes blank saved-key edit and plaintext file disclosure |
| `mise run lint` | Passed | Production renderer and scoped frontend tests |
| `mise run rust:fmt` | Passed after final Rust edits | Syntax/format only, not type checking |
| `mise run format:files ...` | Passed | Reviewed renderer/tests/spec/task files |
| `git diff --check` | Passed | Final diff whitespace |

## Preserved first focused failure

`locked_missing_and_foreign_refs_never_fall_back_to_inline_material` failed at:

```text
assertion failed: db.save_provider("codex", &stored).is_err()
```

Root cause: the save branch with freshly supplied inline input did not reject a
foreign credentialRef before allocating a new native item. The final code checks
requested binding ownership before all save branches. The original assertion is
retained. **This fix has not been rerun.** Separate tests now cover explicit new
input repairing a missing native item; that is distinct from foreign authority.

The first run's passing cases included DB failure compensation, locked migration
preservation/retry, blank/masked retention, rotation rollback then cleanup,
delete retry, ordinary/sync export canaries and real temporary-file Codex
create/edit/switch/delete. Later source changes mean those earlier passes do not
prove the final commit.

## Pending integration checks

Parent will run final native checks on the integrated tree. Suggested focused
regressions before/full aggregate: `mise run rust:test provider`,
`mise run rust:test change_plan`, `mise run rust:test database`, and full required
`rust:check`/`rust:clippy`/architecture checks. There are now 19 credential-owner
cases plus v24 schema and opaque-binding/Change Plan tests. All later native
cases are unexecuted, including cross-app export whitelist, local import route
preservation, inactive draft import, pending-create resume, explicit missing-key
repair, missing-target preflight, official-label bypass and proxy materialization.

Do not replace this pending evidence with a claim based on the earlier compile.
No real credential store or production configuration was read/tested. Native
fixtures use the explicit MemorySecretBackend and isolated test HOME directories.

## Integration notes

- See `export-import-contract.md` for the exact allow/omit/preserve behavior.
- See `ownership-audit.md` for save-path ownership and actual executor evidence.
- `service.rs` modifications are Codex-only; the parent's WorkBuddy recovery fix
  is independent and must be preserved during integration.
- No push, merge, release, or Issue closure performed by this package.
