# Current macOS gap and reuse review

## Required full-contract reads

Before implementation, read the complete `.trellis/spec/backend/external-agent-p0.md` and `.trellis/spec/frontend/v2-agent-models.md`. They are intentionally omitted from automatic JSONL injection because each exceeds the configured context-file size limit.

## Confirmed current behavior

`src-tauri/src/agent_install/desktop.rs` mounts a downloaded DMG, discovers one `.app`, validates the product bundle ID, then computes the destination through `user_applications_dir()`. That function always returns `get_home_dir()/Applications`.

Observation scans both `~/Applications` and `/Applications`, but deployment does not retain the observed candidate path. This asymmetry explains the reported behavior: a system installation can be observed, then update writes a second user-scope copy.

The generic transaction currently uses direct `ditto` copy and validates the resulting bundle ID. It does not preserve an old target through backup/rollback and only the TRAE job path performs a fresh installed observation before success.

## Existing Codex Desktop implementation

`src-tauri/src/codex_desktop/platform/macos/dmg.rs` is explicitly a same-volume replacement transaction. It includes:

- generated staging and backup paths;
- target-parent confinement;
- staged bundle and installed replacement verification;
- backup restoration;
- refusal to delete a replacement whose identity changed;
- fresh system permission fallback tests;
- cleanup tests.

This is the primary reuse owner. Generic Agent update must preserve its own selected target and therefore cannot reuse the Codex fresh-install permission fallback unchanged.

## Architecture conclusion

- Consolidate DMG deployment under one private transaction owner.
- Keep product identity/version in small policy adapters.
- Pass a Stage 1 target capability, not a directory.
- Treat update and fresh install as distinct intents.
- Require inventory readback for every product.
