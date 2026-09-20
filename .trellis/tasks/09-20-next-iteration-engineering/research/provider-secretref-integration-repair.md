# Provider SecretRef integration repair

Status: implementation and scoped formatting complete; writer stopped for root serial Rust validation. No commit was created in the shared integration tree. Earlier integration failures remain valid evidence until the new checks pass.

## Ownership and changed paths

This repair preserves root's existing image-extension exact-marker exception and positive/negative test, plus the explicit Memory backend test helpers. The repair itself changes:

- `src-tauri/src/services/provider/credentials.rs`
- `src-tauri/src/services/provider/credentials/material.rs` (new private owner module)
- `src-tauri/src/services/provider/credentials/tests.rs`
- `src-tauri/src/services/provider/mod.rs`
- `src-tauri/src/services/provider/live.rs`
- `src-tauri/src/provider.rs` (opaque serde-skipped native draft context)
- `src-tauri/src/database/backup.rs` (tests only)
- `src-tauri/tests/provider_commands.rs`
- `src-tauri/tests/provider_service.rs`
- `src-tauri/tests/deeplink_import.rs`

Root's earlier `src-tauri/tests/support.rs` changes are preserved without additional edits. No schema, proxy recovery, installation, frontend save handler, or version changes belong to this repair.

## Native material contract

- New native material uses `fyagent-provider-credential:1` plus strict typed JSON. The bundle contains the inference key and independent usage API key, access token, AccessKey ID and SecretAccessKey. It is private to the native owner and uses the existing SecretService, purpose, size cap and pending/ready/revoked/deleted lifecycle.
- Historical single-key records remain readable. Unknown family versions, malformed structured bundles, unknown fields, missing target metadata, masks-as-material and control characters fail closed; they are not used as API keys.
- One reference/version covers all material. Auxiliary rotation changes the reference. Constant-time native readback compares the complete canonical material. Old versions remain available for enclosing rollback and are deleted only during settle. Persistence now verifies auxiliary metadata on readback and verifies restoration after compensation.
- Persisted auxiliary fields contain only `[REDACTED]` presence markers. Native resolve restores actual values for existing generic usage and Coding Plan consumers. Ordinary SQL/sync export still drops usage scripts and references entirely. The Codex renderer projection retains the non-secret script configuration plus markers so editing no longer silently loses the usage configuration.
- Auxiliary fields: missing or empty means clear; a mask means retain only for this Provider and the same usage target. A fresh value replaces. Missing the whole usageScript removes it. Copying markers to another Provider or changing the usage endpoint, script, language, template/vendor or account identifiers rejects rather than transferring old material.
- The usage target stores a digest of script code, not the script body, to bound payload size. Inference retention compares both configured and effective endpoint, model-provider identity, protocol, requires_openai_auth and category. Blank/masked inference edits cannot change those fields; a fresh key can. Bundles also bind these target fields on native resolution.
- Common settings use a pure projection helper with one snippet snapshot. The Codex writer consumes the same owner-validated effective settings; it does not apply a second common overlay after key resolution. Legal common UI/features settings remain available. Common endpoint/protocol/auth-role changes reject retained credentials and require fresh material; implicit usage destinations share that boundary.
- A native materialized edit carries an opaque NativeCredentialDraft in a serde-skipped ProviderMeta field. Only the owner constructs it. It retains the source Provider/reference and a fingerprint of non-secret effective route/usage target data, never a credential-derived hash. Repeated merge, save and final projection revalidate this context, so changing the common snippet after merge cannot turn an inherited key into a fresh authorization. Rotating the source reference also invalidates the draft. Serde cannot import/export the context, and renderer projection removes it. The Change Plan draft map keeps typed Provider clones, preserving this native context.
- Retain also validates the current persisted row against the native bundle target. A matching request and externally tampered current row cannot jointly reauthorize the original material for a different target.
- Main and usage credentials can deliberately have equal values. Token Plan normalization remains independent and does not remove a same-valued explicit usage key.
- Auxiliary-only drafts can hold a native bundle without an inference key. Clearing their last auxiliary field removes their reference; existing activation validation still rejects a missing inference credential.

## Fixture corrections

- Old inline-key expectations now assert reference-only storage and actual native materialization or temporary Codex file projection.
- Codex switchback fixtures now have complete endpoint/model-provider/protocol routes; production guards remain unchanged.
- The Claude disk persistence case explicitly uses the disk-backed test helper, while Codex credential tests retain the explicit Memory backend helper.
- Generic SQLite BLOB round-trip coverage moved to a fixture scalar table; Provider settings remain valid JSON. A new negative test proves malformed binary Provider configuration rejects both ordinary and sync export without modifying its original bytes. No export guard was loosened.
- Quick Setup rollback assertions compare the persisted preimage, and bounded error-code assertions replace obsolete source-bearing error messages.

## Added checks (Rust execution pending)

Twelve new credential cases:

1. `auxiliary_credentials_share_native_lifecycle_and_never_enter_dto_or_export`
2. `auxiliary_masks_retain_then_explicit_empty_fields_and_script_removal_revoke`
3. `auxiliary_masks_cannot_cross_owners_targets_or_scripts`
4. `blank_inference_edits_cannot_redirect_credentials_but_fresh_capture_can`
5. `unknown_or_corrupt_native_bundle_never_becomes_a_key_or_plaintext_fallback`
6. `auxiliary_migration_failure_and_db_readback_failure_preserve_complete_previous_material`
7. `actual_usage_query_materializes_auxiliary_fields_before_script_validation`
8. `current_row_target_tampering_cannot_reauthorize_retained_material`
9. `common_config_keeps_legal_settings_but_redirect_requires_fresh_authorization`
10. `enabling_common_redirect_cannot_retain_inference_or_implicit_usage_keys`
11. `native_draft_cannot_redirect_after_merge_or_outlive_its_source_reference`
12. `serialized_provider_cannot_forge_or_export_native_draft_authority`

The actual usage query test executes the existing native query/JS validation path using isolated fixture state. Correct substitutions reach deterministic HTTP-scheme rejection; wrong or masked values fail script evaluation first. Both outcomes stop before network I/O. It is not a vendor-service acceptance test.

Backup adds `provider_export_rejects_non_json_storage_without_mutating_source`.

Completed after these edits:

- Scoped `mise exec -- rustfmt --edition 2021 --config skip_children=true` for all owned changed Rust files: passed.
- `git diff --check`: passed for the shared tree at this checkpoint.
- Read-only audit confirms `commands/provider.rs::query_provider_usage_inner` and `services/provider/usage.rs` already materialize through the owner; no additional consumer rewrite was needed.

## Remaining validation and limits

- Root must run the serial Rust suite after the other writer reaches a stable boundary. No new Rust compilation or test pass is claimed by this repair.
- Suggested focused diagnosis if the integrated run fails: provider credentials; provider unit tests; provider commands/service/deeplink integration modules; the two backup cases.
- The previously identified common-overlay and merge-to-writer native-draft gaps are now addressed in code and deterministic temporary-file tests; integrated Rust execution is still pending.
- The existing native material size limit is unchanged (2560 bytes); oversized multi-secret payloads reject before Provider cutover and retain the previous configuration. No unbounded vault or plaintext fallback was added.
- Compatibility behavior for former single-key native records has fixture coverage authored (execution pending); OS-keychain access, Windows native-store UAT, real subscriptions and vendor quota calls were not exercised.
- Runtime role observed: native trellis-implement subagent. Exact model and reasoning effort are not independently observable in this execution context.
