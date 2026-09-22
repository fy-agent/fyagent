> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# OpenCode create-only rework

Status: implementation and bounded validation complete; ready for root integration. No commit made. OpenCode production file released after report handoff.

## Scope and mechanism

Only `native/opencode.rs` changes. Strategy `opencode.import-staged.create-only-v3`, exact supported CLI `1.18.30` remains. No dependency, schema, global framework, or persistent sidecar is added.

1. Validate the existing target database against the four exact native `project/session/message/part` column signatures, rejecting unknown/generated columns and triggers.
2. Use a private `TempDir` for both `debug config --pure` and `import --pure`, with the selected target working directory and local configuration. The official CLI resolves the target model and encodes its project binding. Its upserting importer never receives the target DB path.
3. Record the preallocated/reused session ID in the receipt before import. Official private export must exactly match the package roles/order/body digest before publication.
4. Open the existing target DB without CREATE, enable foreign keys, acquire `BEGIN IMMEDIATE`, and revalidate its schema under the write lock.
5. Insert a missing native project without modifying any existing project metadata; insert session/message/part with explicit allowlisted columns and ordinary INSERT only. Check exact row counts. No CLI call runs under the target lock.
6. Any row collision or copy failure rolls back the entire transaction, including a just-created project. Commit success is the sole Written path. Temporary DB, WAL/SHM, and input are removed by TempDir RAII.

The initial session-ID lookup remains an early diagnostic only. Atomic no-overwrite comes from SQLite primary keys and the transaction, even if another process writes between preflight and publication.

## Failure classification

- Version/store/schema/receipt/private import/private export failure: ProvenNoSideEffect for target session content.
- Target transaction cannot start: ProvenNoSideEffect.
- Transaction insert/schema/count failure with confirmed rollback: ProvenNoSideEffect; original target rows remain intact.
- Rollback or commit result cannot be confirmed: Unresolved; reconcile before retrying.

These classifications concern this restoration attempt's target content. The external CLI may update its own temporary staging files, and a concurrent writer can independently publish the colliding existing session.

## Final validation results

- **Actual Rust module tests:** `rtk proxy mise run rust:test -- session_manager::migrate::native::opencode::tests` → exit 0, **8 passed / 0 failed / 1 explicitly ignored**. Covers missing target project, preserved existing project metadata, session/message/part collision rollback, missing source project, copy-count mismatch, schema drift, target trigger rejection, version/store/receipt/reused-ID checks, and strict final-only re-extraction.
- **Actual Rust restore entry point:** `isolated_actual_writer_create_only_probe` → **1 passed**, explicitly invoked by `work/native-writer-staging/verify_create_only.py` against the exact test binary built by the canonical task. This executes product `OpenCodeWriter.restore`, including official private import/export and atomic target publication. It does not substitute handwritten native JSON for the writer.
- **Git project binding:** the target native DB initially contained no Git project. Restore created project `67ef81bcf651055bc4f66e322d571073d849b3ea`; session directory and project worktree exactly matched the fresh synthetic Git workspace. The product uses the official staged project ID, not the `global` JSON placeholder.
- **Receipt-time race:** after preflight found no session, a context callback inserted the same ID and independent existing rows. Product restore returned ProvenNoSideEffect after transaction rollback; snapshots of all project/session/message/part values were identical to the concurrent writer's state. Separate module tests additionally prove rollback when a missing project was inserted before a session/message/part collision.
- **Fresh native process readback:** official `export --pure` returned exact user/assistant order and all text, including leading/trailing spaces, CRLF, Chinese, Windows-style path text, two consecutive users, two consecutive final answers, and a final unanswered user. Deliberately decreasing source timestamps did not reorder the native messages.
- **Next-turn mock:** a fresh official `run --pure --format json --session <actual Rust-created ID>` without `--model` selected target `probe/local`. The loopback endpoint received the five historical messages verbatim, in order, followed by the synthetic new user prompt. It returned HTTP 400 intentionally. **No generated model answer, paid inference, or real-user UAT was performed.**
- **Cleanup:** after success and collision failure, no `fyagent-opencode-*` directories remained under the isolated TMPDIR. Target DB stayed available to a fresh official process. Source/configuration data stayed inside the synthetic HOME/XDG and test workspace.

## Reproduction and evidence

Selected artifacts: `evidence/native-continuation/create-only/` contains native test output, actual writer result, official export, projected captured request, intentional HTTP 400 continuation output, source hash/commands, and the reusable harness. Full isolated runtime is at `work/native-writer-staging/create-only-720880cca1`.

Build first with the canonical module command above; then run the scratch harness. It invokes only the explicitly ignored native test and binds every application home/data/config/temp path to a newly created directory. It links the already-installed OpenCode 1.18.30 binary, disables network-dependent model/update fetches, removes inherited secrets, points the only enabled provider at loopback, and blocks outbound proxy routes. No dependency installation or CLI upgrade occurs.

The initial E0433 capability test compile failure is retained in `first-module-test-blocked.txt` and **superseded** by the successful canonical run after the owning worker fixed its missing import.

## Boundaries and integration note

This proves the actual provider writer, its controlled receipt callback, official native visibility, and the next-request payload. It does not replace root's full migration/receipt database integration tests, a real model response, Windows UAT, or total suite/Clippy checks. The user waived unavailable Windows UAT.

After the native proof, both private staged export and final verify_readback export were connected to root’s `run_with_output_limit(..., MAX_NATIVE_READBACK_OUTPUT)` (256 MiB). Version/config/import output retains the small bound. The runner’s new OutputIncomplete result follows the existing failure/blocked branches; it cannot become successful readback. This last boundary-only integration was formatted but, as coordinated with root, the module compile and common runner tests are included in root’s final regression rather than repeating native inference-free probes. Evidence hashes distinguish the proven writer revision from this final interface-only revision. No shared files were changed by this package.
