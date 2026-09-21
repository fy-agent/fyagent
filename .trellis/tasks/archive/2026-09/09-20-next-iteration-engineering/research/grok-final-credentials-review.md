Execution: terminal Grok CLI 1.0.34, grok-4.6, existing login. Session 01a0c11b-3792-73b3-ae02-da2baae8c870. Initial bounded review stopped with max turns reached (exit 1); resumed for a report only, no more tool calls, exit 0. Initial reviewed diff preserved under artifacts/grok-credentials-reviewed-delta.patch. This reviews the earlier delta, not the active replacement.

# Grok review: inspected dirty credential-gate delta

Scope: uncommitted production edits in `src-tauri/src/services/provider/live.rs`, `src-tauri/src/services/change_plan/adapter.rs`, `src-tauri/src/services/change_plan/service.rs` at HEAD `a0a170b2` plus the Cursor gate-repair dirty tree. This is a report of **that inspected delta**, not of any later Cursor replacement of the post-snapshot common-config copy.

Existing contract (source-proven, not inferred from the new helper):

- Codex live writes are **one read, in-memory patch, one atomic write**.
- `write_codex_live_for_provider` (`codex_config.rs:695-709`) reads with `read_and_validate_codex_config_text` (I/O error is an error; missing file is empty), `patch_source` copies only source-owned routing/model keys, then `write_codex_live_config_atomic` once.
- `source_switch.rs:1-3,72-156`: provider owns the routing table; MCP, profiles, features, credential-store, and other live tables stay with the live document. Absence of a desired key is not permission to reset live.
- `read_codex_config_text` / Quick Setup live read: `path.exists()` then `map_err(AppError::io)`, else empty. They never treat a failed read of an existing file as `""`.

---

## Q1 — `persist_unowned_codex_common_tables`

**No.** The helper retains `[tui]` for the focused happy path, but it is a second committed write that violates the existing transaction/ownership contract.

### Confirmed defects

**1. High — I/O failure treated as empty file, can wipe the just-written snapshot**

- File: `src-tauri/src/services/provider/live.rs:796-799`
- Trigger: `write_live_snapshot` succeeds; the follow-up `std::fs::read_to_string(&path).unwrap_or_default()` fails (permissions, sharing, transient I/O) or the file is unreadable.
- Source-proven result: `current` becomes `""`, `live` is a new empty `DocumentMut`, then only non-`SOURCE_OWNED` keys from desired are written. Owned routing (`model_provider`, `model`, `model_providers`, bearer) from the first write is replaced.
- Narrow repair: delete the post-snapshot re-read. Merge unowned snippet keys into the in-memory document inside `write_codex_live_for_provider` / `patch_source` **before** the existing single `write_codex_live_config_atomic`. Read via `read_codex_config_text` / `read_and_validate_codex_config_text`.
- Test: after a successful owned snapshot, inject a read error (or chmod unreadable) on `config.toml`; assert owned routing bytes are unchanged and the call returns I/O/`provider_codex_config_invalid`, not a truncated unowned-only file.

**2. High — partial commit / two-write race**

- File: `live.rs:777-778` then `829`
- Trigger: `write_live_snapshot` commits; `persist_unowned_*` then fails parse/write, or another writer mutates `config.toml` between the two atomics.
- Source-proven result: caller sees `Err` after live is already source-switched without common tables; or the second write last-writer-wins over a document that is not the snapshot just produced. This is not the existing one-write contract.
- Narrow repair: same as (1) — one in-memory merge, one atomic write. Do not re-read after commit.
- Test: fail the unowned merge/write path; assert `config.toml` is byte-identical to pre-call (all-or-nothing). Concurrent writer between the two current writes is the race; after a single-write repair it goes away.

**3. Medium — copies all desired unowned tables onto live, replacing whole tables**

- File: `live.rs:823-828` (`live[key] = item.clone()`)
- Trigger: effective desired config (resolve = stored provider config + common snippet) contains any non-`SOURCE_OWNED` key, e.g. snippet `[tui] notifications = false` while live `[tui]` also has other keys; or stored provider `config` carries `[mcp_servers]` / `[features]` / `[profiles]`.
- Source-proven result: inverse of `patch_source`. Live unowned tables that appear in desired are replaced, not merged. Extra live keys in the same table are dropped. `apply_common_config_to_settings` uses `merge_toml_table_like`; this helper does not. Focused test `common_config_keeps_legal_settings_*` only asserts `notifications = false` is present on a routing-only fixture, so it does not catch this.
- Narrow repair: merge **common-snippet** unowned keys with `merge_toml_table_like` onto the already-patched live document. Do not overlay arbitrary unowned keys from the provider document.
- Test: live `[tui] notifications=true` plus an extra tui key, plus live `[mcp_servers]`; snippet only `[tui] notifications=false`; after write, extra tui key and mcp survive; notifications becomes false. Second case: provider stored config contains `[mcp_servers]`; live mcp must remain the live document’s.

**Smallest correct placement (for this inspected version):** fold snippet-unowned merge into `write_codex_live_for_provider` after `patch_source`, before the existing atomic write. Remove `persist_unowned_codex_common_tables` as a post-snapshot writer. Quick Setup already returns before this helper (`live.rs:773-775`); that path is unchanged here.

This review does **not** cover any later replacement of this helper.

---

## Q2 — persisted-row upsert verify + first-SecretRef digest compare

Success-path identity for first persist is source-consistent. Compensation classification for a never-written create is not.

### What holds (not defects)

- Preview/admission identity is unchanged: upsert `inspect`/`precheck` still uses `inspect_codex_intended_provider` on the in-memory draft (`adapter.rs:217-218,263-264`). Stale admission still compares that snapshot to the stored plan (`service.rs:737-744`).
- `merge_edit` always `remove`s `credentialRef` (`credentials.rs:621-624`), so the stored upsert digest is the unbound definition. After first persist, public row gains `pc_*`. `definition_matches_after_first_secretref` (`service.rs:1867-1883`) exact-matches or strips readback `credentialRef` and re-hashes. That is the allowed first-assign exception; it does not require identical opaque refs.
- `provider_definition_digest` hashes id/name/category/`credentialBinding` plus credential-neutral routing, not auth bytes. Stripping only `credentialRef` is the right hashed delta for first persist.
- Switch rotation before writer still fails closed: switch precheck inspects the DB row; `credential_rotation_after_plan_is_stale_before_writer` is unchanged. If stored digest **includes** a ref, stripping readback’s ref cannot equal stored (comment at `1876-1877` is true for **switch**).
- Verify is read-only. Success `upsert_plan_is_side_effect_free_and_apply_writes_once` still proves one writer call + idempotent replay. This delta does not add a second write on the success path.

### Confirmed defect

**4. High — upsert writer failure with no row classifies as readback unavailable, not compensated baseline**

- File: `adapter.rs:275-280` together with `service.rs:1893-1905` and `2012-2016`
- Trigger: `plan_codex_upsert` create; `managed_write` returns `Err` without inserting the reserved row (or activation rolls the row back). `verify()` calls `inspect_codex_switch(state, &self.provider.id)` → `TargetNotFound`.
- Source-proven result: `classify_job` treats any readback `Err` as `ReadbackUnavailable` + `RecoveryRequired`. It never reaches `WriterFailedBaselineRestored`, which requires a successful inspection plus `baseline_db && baseline_device && baseline_live && definition_target`.
- Prior verify used `self.inspect()` on the adapter’s draft clone (still present after `take_upsert_draft`). Definition matched the stored unbound digest; baseline current/live would classify compensated.
- Switch can use saved-row verify because the target row always exists. Upsert create does not.
- Narrow repair: `verify()` uses `inspect_codex_switch` when the row exists; on `TargetNotFound` fall back to `inspect_codex_intended_provider` of `self.provider` (writer-failed create), **or** map `TargetNotFound` + writer `Err` + baseline current/live to `WriterFailedBaselineRestored` without requiring definition readback of a missing row.
- Falsifiable test: upsert create, upsert writer `|_| Err(())`, no insert; assert `result_code == WriterFailedBaselineRestored`, `recovery_state == Succeeded`, writer calls `== 1`, live/current unchanged. Today this must fail as `ReadbackUnavailable`.

### SecretRef upsert/classification — questions (not confirmed defects)

These are untested or comment/assumption gaps. Evidence is insufficient to call them bugs.

1. **Upsert stored digest never includes `credentialRef`.** The helper comment (`service.rs:1876-1877`) says rotation still mismatches because stored includes the old ref. That is true for switch, **false for upsert**, because `merge_edit` strips the ref before plan inspect. Post-write classify will accept **any** first `pc_*` with the same unbound identity. Whether that is intended is the stated first-persist exception; it is not a proven extra hole if the writer is the planned draft under the mutation lock.

2. **Upsert precheck does not observe DB-row rotation.** It inspects the frozen draft, not `inspect_codex_switch`. Rotation of an existing reserved row after plan is rejected only if apply-time `merge_edit` / `validate_native_draft` fails the stale `source_ref`. No upsert-equivalent of `credential_rotation_after_plan_is_stale_before_writer` exists in this delta. Insufficient to claim a new classify bug; it is an untested gate.

3. **Upsert update that persists a new ref while the plan draft was unbound** should still match via strip (same as create). If a future change stopped stripping the ref at plan time, the helper would false-fail a planned rotation. Not present in the inspected `merge_edit`.

4. **Shared `classify_job`.** The strip helper runs for switch too. Safe while switch stored digests include `credentialBinding`. A switch plan against a legacy unbound row that later gained a ref would now succeed definition match. No such production path was shown; capability tests reject inactive/unmanaged material before classify.

5. **Reconcile** (`service.rs:1277-1278`) already used `inspect_codex_switch` for upsert. Immediate post-write classify now matches reconcile for missing-row create (both `ReadbackUnavailable`). Aligning them does not make the compensation contract correct.

6. **One-write on mixed failure** (save landed, live rolled back, leftover row): inspect-by-id + strip can mark `definition_target` true while current/live are baseline → compensated with a leftover row. That is the writer-owned rollback contract, not proven broken by this digest helper. No focused test.

---

## Out of scope / not re-audited

- Fixture-only edits (`save_provider_record` for inactive bearer, blank-legacy model-only edit, Tokio usage-query runtime) are test/admission layering, not the two asked production questions.
- Adjacent dirty files (`commands/provider.rs`, `universal.rs`, `provider_service.rs`, usage `UsageTestInput`) were not the review target.
- Passing 35 credential tests + six focused filters do not cover findings 1–4.
- No claim about any pending replacement of the post-snapshot common-config copy.
