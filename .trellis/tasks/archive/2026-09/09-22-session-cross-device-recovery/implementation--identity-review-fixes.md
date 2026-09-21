> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Grok identity review fixes

Scope: only `migrate/identity.rs`, `migrate/identity_platform.rs`, and this report. The two product files are released for integration after this report. DAO/receipt/export/provider files were not edited.

## P2 #3: installation rename

Accepted. Installation identity no longer includes the canonical directory pathname. It retains the persisted random v4 seed, OS machine/user fingerprint and directory filesystem instance identity. Therefore renaming the same directory preserves install identity, device binding and source namespaces, while copying/replacing the directory or moving it to another machine/user still invalidates them. `store_instance_id` still includes its canonical path digest, as requested, because native writers address stores by path.

Regression: `same_directory_rename_preserves_install_binding_and_source_namespace`. The existing copy, replacement, other-machine/user tests remain.

## P2 #4: directory replacement around lock acquisition / use

Accepted. The transaction now retains an actual `IdentityDirectory` handle, opens its lock through that owner, and validates both the directory instance and locked file identity after lock acquisition and before returning success.

- macOS opens/reads identity children with `openat(O_NOFOLLOW|O_CLOEXEC)` relative to the retained directory fd. It stages private metadata in that same directory, syncs the file, publishes with `renameat` using the same fd on both sides, and syncs the directory. If a rename/replacement slips between the final check and publication, publication remains in the held old directory; the replacement receives no write, and the transaction post-check rejects the result.
- The existing config atomic writer accepts paths only. A local probe confirmed macOS does not support `/dev/fd/<directory-fd>/child` as a suitable adapter. The small fd-relative writer is consequently private to these migration-control JSON files; no shared config API or general filesystem framework was added.
- Windows pins the canonical directory ancestor chain with handles that omit DELETE sharing and reject reparse points. It then retains the existing `config::atomic_write` owner. This path is source/API checked, not Windows runtime evidence.

Regressions: directory replaced before admission does not invoke the body or create state in the replacement; directory replaced inside a held transaction causes failure without touching the replacement; direct publication in the exact final-check -> rename window remains in the held directory.

## Platform and compatibility constraints

Per coordinator correction, concrete Linux implementation and broad `cfg(unix)`/`cfg(windows)` branches were removed. Product branches are explicitly macOS/Windows; other hosts fail closed. No dependency, lockfile, MSRV (1.85), public IPC or scanner change was made. The contract owner must freeze the new exact source signatures without relaxing platform policy.

## Actual checks and iteration history

- Only the two owned files were formatted with mise's locked rustfmt.
- Initial focused run was blocked by another writer's temporary `capability.rs` test calling the new `detect_version(&Path)` API with `&str`. That owner fixed it; no cross-package edit was made here.
- First executable run passed the four new regressions but caught one existing 12-thread test failure while opening the lock. The new raw `openat` wrapper was missing standard EINTR retry semantics; these were added. The failed run did not record the underlying errno, so the exact original errno is not claimed as proven.
- After the final production edits, `rtk proxy mise run rust:test session_manager::migrate::identity` completed with exit 0: **23 passed, 0 failed**. This includes the 12-thread namespace preservation and six actual child-process lock tests. No complete test suite was run by this package.
- The contract owner then identified an unreviewed runtime `cfg!` selector in a test fixture filename. It was removed by using the same `history-one.jsonl` filename on both supported hosts; product behavior was unchanged. The affected replacement/append regression was rerun through `mise run rust:test file_replacement_changes_store_and_unknown_origin_but_appends_do_not` after that test-only edit: exit 0, 2 matching tests passed (library and integration harness).
- Remaining compile warnings in that run were outside these files: `extract_session`, native `RunOutcome::NotStarted.reason`, and the integration harness's unused `config::get_app_config_dir` stub. They remain with their owners for the final Clippy gate.
- Windows runtime/UAT and unsupported hosts were not tested. No real sessions or user credentials were read.

## Final source hashes

- identity.rs: `1b20fab8d5c39b4ab185590df107e16bd1e992da5a912a0f9bc0e2490f0e6586`
- identity_platform.rs: `7762707fef517577a505a6fe023c35d71b0b50726acff988ffe3f2d2a87858b0`

Earlier staging/report statements about Linux support and canonical path in installation identity are superseded by this report and the current source. Store-path binding is unchanged.
