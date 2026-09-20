# Research: FDE Windows CI failure

- Query: 定位 PR #193（head `6f38c213`）Windows backend job `106032279256` / run `35493454323` 的实际失败，判断 flaky 与代码/测试契约问题，并给出最小修正建议。
- Scope: internal / mixed
- Date: 2026-09-20

## Findings

### Failure receipt

- `gh api repos/fy-agent/fyagent/actions/jobs/106032279256/logs` 返回完整 job log。`Backend Checks (Windows)` 在 `cargo test --workspace --features fyagent/test-hooks --locked ... --no-fail-fast` 的 `rust-tests` 步骤失败；runner、Rust 版本、`FYAGENT_WINDOWS_MANIFEST=test` 和 `secretref-hil` 均正常，Clippy/check/SecretRef HIL 均成功。
- 主测试结果为 `3344 passed; 7 failed; 5 ignored`。失败集合是：
  - 5 个 `agent_install::tests::*` 测试，在 `src/windows_runtime/mod.rs:409` 的 `require_interactive_user_context()` panic，消息为 `WindowsInteractiveUserContext must be initialized before any Windows user-path access`；调用链经过 `config::get_app_config_dir` / `AppState::new` 或 Windows inventory。
  - `services::fde_workspace::tests::fde_workspace_sample_binding_persistence_export_and_revision_invalidation` 在 `src-tauri/src/services/fde_workspace/tests.rs:30` unwrap `InvalidInput("projects_platform_unavailable")`。
  - `services::managed_auth::subscription_tests::transport_tests::health_admission_collect_preserves_seeded_database_native_files_and_vault` 在 `src-tauri/src/services/managed_auth/subscription_transport_tests.rs:76` unwrap `health_read_failed`；其栈也回到 `windows_runtime::require_interactive_user_context()`，由 Windows inventory 在 health collection 中触发。
- 随后的独立 `secretref-hil` 命令实际成功：`native_os_backend_crud_readback ... ok`，`1 passed; 0 failed`。因此不是 Credential Manager HIL 或 Windows runner 本身失效；Required gate 失败的直接原因是 `rust-tests`。

### Root cause classification

- 这不是随机 flaky 的证据。失败是确定性的环境/测试契约不一致：生产入口 `src-tauri/src/main.rs:12` 调用公开的 `initialize_windows_user_context()`，但 Rust unit-test binaries 不经过该 production `main`；当前 Windows-only context 是 `OnceLock`，在未初始化时按设计 panic（`src-tauri/src/windows_runtime/mod.rs:388-411`）。测试并行启动后，多条路径稳定撞到同一未初始化边界。
- FDE workspace 失败是另一个明确的跨平台测试缺口：`src-tauri/src/services/projects/files.rs:187-205` 在 Windows 明确返回 `project_error("platform_unavailable")`，而 `fde_workspace_sample_binding_persistence_export_and_revision_invalidation` 没有 `#[cfg(target_os = "macos")]`，却无条件在 `tests.rs:30` 要求 `write_context(...).unwrap()`。相邻的 `services/projects/tests.rs` 已给依赖 macOS 文件实现的测试加 `#[cfg(target_os = "macos")]`（例如其 51、73、210、327、371 行），说明这里漏了同类平台门禁。
- 当前 FDE PR body 已把 Windows 实机/正式 UAT 列为未验收，但 CI 仍把一个明确声明 Windows `platform_unavailable` 的测试纳入 Windows 全量 Rust gate；这属于代码/测试契约问题，不能靠重跑标记为 flaky。

### Minimal fix recommendation

1. 先给 FDE workspace 的端到端文件工作流测试加 `#[cfg(target_os = "macos")]`，或把它拆成 Windows 可执行的临时目录实现并补齐 Windows `files` owner。按照当前产品边界和 PR body，前者是最小、诚实的修正；不要在 Windows 上把 `platform_unavailable` 改成假通过。
2. 为 Windows unit-test 进程提供一次且仅一次的真实 context 初始化，再运行依赖 `AppState`/health/inventory 的 tests。最小可审查实现是在 Windows test bootstrap/测试辅助入口中调用已公开的 `initialize_windows_user_context()`，并让 `agent_install` 的 `test_app_state()`、`subscription_transport_tests` 的 AppState fixture 和其他直接构造 AppState 的 Windows tests 共享该 bootstrap；不要把 production `require_interactive_user_context()` 放宽为环境变量或重新回退到 `dirs::home_dir()`。若项目不接受 test bootstrap，应至少在这些 fixture 的最早入口显式初始化并断言成功。
3. 修正后至少重跑 Windows `cargo test --workspace ... --no-fail-fast`，确认 `rust-tests` 全绿，再让同一次 CI run 的 `CI / Required` 重算；随后保留 `secretref-hil` 的独立 1/1 通过结果。前端、macOS 和其他检查不构成这次 Windows failure 的修复证据。

### Existing evidence and regression signal

- 当前 FDE 本机 macOS closure log 中 `fde_workspace_sample_binding...` 与 `health_admission_collect...` 均通过，说明失败并非这两个业务断言在 macOS 的回归；它也不能证明 Windows 路径正确，因为 Windows implementation 明确返回 unavailable。
- 旧的成功 Windows CI（例如 run `35084015781` 的 job `104754630031`）曾通过 `3279 passed; 0 failed; 5 ignored`，但该 run 发生在 FDE workspace/verification 测试加入前，不能作为本次新增测试的反证或修复证明。
- 本次 run 的 checkout 记录为 merge ref `refs/pull/193/merge`，checkout commit `37b77971`；因此日志不是 stale branch-only run。修复必须推送新 head 后重新读取新 run，不能复用 `35493454323` 的失败/部分成功结果。

## Caveats / Not Found

- 未编辑源代码、未运行会改变工作树的命令、未 stage 或 commit；本报告只使用 GitHub job log 和 FDE 只读树源文件核对。
- 没有证据表明 Windows native SecretRef 失败；该步骤已独立通过。也没有证据支持把 `rust-tests` 失败归因于 runner flake、缓存或超时。
- 如果主控选择实现真正的 Windows project-files owner，而不是 macOS-only 测试门禁，所需范围会超过本次最小修正，必须另行补 Windows 行为测试与 UAT 边界。
