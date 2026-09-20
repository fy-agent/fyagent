# Delivery evidence — #73

Worktree: `~/.codex/worktrees/fyagent-next-config-pack/fyagent`
Branch: `codex/next-config-pack-73`
Base: `2c09c4be2c5f50b4060fd6f7a13a7be31c364c54`
Parent: `09-20-next-iteration-engineering` in the root integration worktree.

## Implemented behavior

Models header → “导入 / 导出连接” opens the lazy shared dialog. The production
Models page currently routes Claude Code and Codex through ProviderPanel; this
format transfers selected saved connections for those two supported targets.
WorkBuddy, account bindings, prompts, memory, executable MCP/Skill configuration
and share links are outside this format version. The UI identifies this scope.

Native JSON v1 is a closed projection of app/name/endpoint/model/wire API, with
64 KiB UTF-8 and 32-entry limits. It omits credentials, credential references,
metadata, local paths and activation. Imported files must come from the native
picker and be bounded regular JSON. Export is previewed text and create-only
publication to a new `*.fyagent-config.json` file, followed by actual readback.

Import previews show final fields, skip/rename/eligible-draft overwrite and
pending credentials. Only native preview ID/digest is confirmed. Text/choice
changes revoke the preview; native inventory/current-selection drift fails
before writes. A single SQLite transaction writes inactive credentialless
drafts and reads them back before commit; a later-row or readback failure rolls
back all rows. No live Agent config or secret storage is written.

The dialog exposes “填入模型配置表单” for actual imported readback and for stored
candidates on reopening. Its typed callback preserves app/name/endpoint/model/
wireApi; it only fills a local form and closes the dialog. It neither provides
credentials nor dispatches native save/activation. The Models host must wire
the callback (root-owned integration described below).

## Checks and evidence boundary

All commands use `rtk proxy` and the canonical `mise` tasks.

- `mise run bootstrap`: passed with frozen dependencies. The optional macOS to
  Windows MSVC toolchain diagnostic was unavailable; no cross-device claim.
- `mise run typecheck`: passed after the UI/port/test implementation.
- `mise run lint`: passed after the UI/port/test implementation.
- `mise run test:unit configPack ConfigPackDialog tauriAclContract rustModuleBoundaries verify-route-chunks`:
  passed, 6 files / 39 tests. Covers strict portable DTOs, exact confirmation
  payload/readback, masking unknown host errors, native-only browser behavior,
  selected export, picker cancellation, actual preview fields, protected
  overwrite, choice/text invalidation, duplicate dispatch, late response and
  exact model-form fill fields including Chat without native writes.
- `mise run rust:fmt:check`: passed after final native edits.
- `git diff --check` and Trellis task context validation: passed.
- Earlier `mise run rust:test config_pack`: passed, 9 tests. These tests use
  actual in-memory SQLite transactions and actual temporary filesystem files;
  fault triggers prove whole-batch rollback and readback failure. They also
  cover secret/path/schema/size rejection, selective export, nonactivation,
  conflicts, stale preview/current selection, expiry/cancel, credential-table
  overwrite protection, symlink rejection and create-only file export.

The last Rust pass preceded small final edits requiring a rerun: explicit
nullable `wireApi`, source Authorization filtering, pre-open regular-file
validation, relative model-path rejection, duplicate/batch/inventory bounds,
and test SQL seeding compatible
with #35's credential-aware save facade. The matching new assertions have
been added and formatted; they are not represented by the earlier 9/9 result.

The parent requested no further heavy work during release acceptance and
explicitly owns final `rust:test config_pack`, Clippy, production renderer build
and route budgets after integration. Native dialog/picker interaction, Windows
runtime and a real-account migration have not been exercised. Unit/renderer
fixtures are not native UI, release or production evidence.

## Integration notes

Root explicitly retains Page.tsx write ownership for #56 save-handler changes.
This package therefore leaves its existing `<ConfigPackButton />` call for root
to connect using `onFillModelForm`. Import `ConfigPackFormFill` from
`src/shared/features/config-pack.ts` (or PortableProvider from the domain).
The callback must select provider.app and fill name/endpoint/model/wireApi,
with credentials still empty and a separate explicit apply preview required.
The current fixed quick-setup form cannot select UUID drafts itself; readback
and re-export alone are not complete migration acceptance. Root will connect
the callback and #40 will add the form/native-request protocol field so Chat
is preserved. This dependency is pending, not claimed complete here.

No schema migration. A portable-only atomic DAO was agreed with #35; it rejects
overwriting credential-bound providers when `provider_credentials` has pending,
ready or revoked records, and exact settings equality rejects credentialRef or
other extra settings. The table guard also permits the pre-#35 baseline.
Malicious legacy credentials in tests are synthetic SQL fixtures, never real
credentials admitted through the production facade.

Seven new commands require narrow merges in commands/mod.rs, services/mod.rs,
database/dao/mod.rs, lib.rs, `config-pack.toml` and capabilities/default.json.
FeaturePorts, browser/native factories, query key and Models header wiring are
small additions. The ACL command-count assertion rises 150 → 157 on this base;
root must combine counts with other new commands. `verify-route-chunks.mjs`
adds the lazy configPack port; its tests now derive counts from entry arrays
so root's separate About lazy-port addition can coexist. No byte budget changes.

Original dirty checkout, versions, release files, navigation definitions,
credential owners, AgentDirectory and shared Change Plan UI were untouched.
This package is implemented; integration acceptance and issue closure remain
with root.
