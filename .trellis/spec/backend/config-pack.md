# Portable Configuration Pack

## 1. Scope / Trigger

Read before changing selected connection import/export, native preview authority,
portable fields, picker file access, or the atomic inactive-draft writer.
Owners: services/config_pack, database/dao/config_pack, commands/config_pack.
The Models entry currently offers saved Claude Code and Codex connections.
This version is an explicit portable format, not an application backup or a
permanent limit on future supported configuration types.

## 2. Signatures

Seven registered commands share allow-config-pack:
- list_config_pack_candidates() -> { entries: [{ selectionId, provider }], excluded }
- preview_config_pack_export(selection: string[]) -> { exportId, digest, text }
- pick_config_pack_file() -> string | null
- preview_config_pack_import(request: { text, choices }) -> { previewId, digest, entries }
- apply_config_pack_import(request: { previewId, digest }) -> { providers, skipped }
- cancel_config_pack_preview(previewId) -> null
- save_config_pack_export(request: { previewId, digest }) -> boolean

The database method insert_portable_provider_drafts accepts only typed portable
writes plus the expected inventory revision and native local-current selections.
It never accepts a raw Provider, metadata, credentials or caller SQL.

## 3. Contracts

The JSON format is fyagent-config-pack/v1. Top-level fields are format and
providers. Each provider contains app (claude/codex), name, endpoint, model and
wireApi (null for Claude, responses/chat for Codex). Unknown fields are rejected.
Bounds: 64 KiB UTF-8, 1–32 entries, 160-byte names, 128-byte model IDs,
2048-byte endpoint. Names are unique within an app.
The local source inventory is bounded to 256 rows, and a batch cannot exceed
that total; limits fail explicitly rather than returning a partial inventory.

Native projection allowlists these fields and excludes all authentication,
SecretRef/credentialRef, env values other than Claude base URL/model, metadata,
notes, headers, account references, commands, activation and device paths.
Unsafe portable values and values containing a known source credential are
not export candidates. No network request, execution, activation, file install
or Agent config write occurs in this feature.

Import choices are add, skip, rename or overwrite plus nullable name; rename
requires a valid unique final name. Conflicts default to skip. Only a
config-pack-draft row with exact generated credentialless settings, empty meta,
no endpoints, no current/failover selection and no pending/ready/revoked
provider_credentials row can be overwritten. Other records permit skip/rename.
No credentials are preserved through overwrite because credential-bearing rows
are ineligible.

A native preview stores final fields, final decisions, generated IDs, full
Provider-row inventory revision and native current selection. It expires after
10 minutes; at most 16 import/export previews exist. The digest binds final
entries and baseline to a UUID. Apply accepts only that ID/digest, checks local
current selection and inventory again under both Provider switch guards, and
consumes the capability before mutation. Editing text/choices requires a new
preview. Raw rows and their revision stay native.

One SQLite transaction performs all draft writes and reads the stored fields
back before commit. Any write/readback failure rolls back the batch. SQLite
also owns crash rollback; no compensating file writer or schema migration is
introduced. A lost response is reconciled by reopening the candidate list,
never by replaying a consumed token. Live configuration changes and restoration
remain owned by Change Plan and config/recovery.

Only native pickers choose files; renderer IPC carries no path. Input must be
bounded regular JSON; symlink/reparse paths and traversal are rejected.
Export requires *.fyagent-config.json and atomic create-only publication, then
readback. Existing files are never replaced. Cancelled picker returns null/false.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Unsupported version or field, invalid app/wire API | unsupported_version / invalid_pack, no write |
| File/text exceeds bounds | too_large |
| Secret, local/path-bearing or credential-bearing URL | unsafe_content |
| Duplicate name, protected overwrite or rename collision | conflict |
| Changed local rows/current selection or expired token | stale_preview, no write |
| Altered confirmation digest | invalid_preview, no write |
| Later row or stored-field readback fails | write_failed / readback_failed, whole transaction rolls back |
| Picker cancelled | null / false; no success message |
| Existing export file | conflict; original bytes preserved |
| Unknown raw host/parser error | closed safe error text; never display payload |

## 5. Good / Base / Bad Cases

Good: export two selected settings, preview one rename and one skip, save one
credentialless draft and show its stored fields. Base: an unsupported entry is
excluded with a count. Bad: serialize settingsConfig/meta wholesale, accept
caller paths, activate an import, or write a batch as independent Provider calls.

## 6. Tests Required

Run mise run test:unit configPack ConfigPackDialog tauriAclContract
rustModuleBoundaries verify-route-chunks, mise run typecheck, mise run lint,
mise run rust:test config_pack, mise run rust:fmt:check and Rust Clippy.
Build/production renderer and applicable native picker checks remain integration
gates. Cross-device/real-account/native-Windows behavior is not proven by fixtures.

Native tests cover strict fields/version/count/UTF-8 size, secret/path filtering,
selection, actual database readback, current-selection preservation,
skip/rename/overwrite, #35 credential table protection, stale preview/digest,
late-row failure and readback-trigger rollback, expiry/cancel, symlink/oversize
input, create-only export and exact file readback. The shared JSON fixture is
tests/fixtures/configPackDtoContract.v1.json.

## 7. Wrong vs Correct

Wrong: apply({ text, targetPath, activate: true }) or loop over add_draft writes.
Correct: strict preview -> immutable native final entries -> confirm ID/digest
-> compare current state -> one SQLite transaction -> read stored fields -> commit.
