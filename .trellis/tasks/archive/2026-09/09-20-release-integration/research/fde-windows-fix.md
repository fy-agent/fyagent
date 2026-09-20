# PR #193 Windows backend CI 修复

日期：2026-09-20。执行树为 `fyagent-fde-control/fyagent`，起始 clean HEAD 为 `6f38c213`。本报告记录针对 [失败 job 106032279256](https://github.com/fy-agent/fyagent/actions/runs/35493454323/job/106032279256) 的限定修复；没有改组合树产品代码，没有 stage、commit 或 push。

## 已完成的修改

- `src-tauri/src/agent_install/mod.rs`：仅在 `#[cfg(test)]` 模块的 `test_app_state()` 中、Windows 分支最早调用 `initialize_windows_user_context().expect(...)`。Rust 测试不经过生产 `main`，而新增 FDE composition 在 AppState 构造中需要用户路径。5 个原失败的 Agent 测试继续执行原有业务断言。临时 Codex 日志根仍保留。
- `src-tauri/src/services/managed_auth/subscription_transport_tests.rs`：仅在实际 health inventory 测试开始处进行同样的 Windows 初始化，然后继续临时 HOME、内存 DB/vault 和原来的三次采集及不变性断言。修正了 inventory 全部局限于临时 home 的不准确注释；平台安装观察可能只读真实安装证据，但不运行 Agent 或 shell。
- 两处初始化均调用现有生产 owner；其 `OnceLock` 负责同进程重复/并发调用。没有改 `USER_CONTEXT`、native resolver、`require_interactive_user_context()`、`AppState::new()` 或路径选择。缺少真实 Explorer/Shell authority 时仍必须失败，不能用合成身份、进程环境变量回退、忽略错误或提前成功返回来使 CI 通过。
- `src-tauri/src/services/fde_workspace/tests.rs`：仅把依赖 macOS project-files owner 的 `fde_workspace_sample_binding_persistence_export_and_revision_invalidation` 及专属 imports 限定为 macOS。原跨平台 `fde_workspace_unbound_package_cannot_create_a_pass` 继续运行。新增 Windows 专属 `fde_workspace_windows_context_write_reports_unavailable_without_publication`，精确断言 `projects_platform_unavailable`、整条 project 记录不变、context 仍 `NotCreated`、revision 不变、无内容/目录/Codex 指令且没有创建 projects 目录。没有把 Windows 文件实现改为假成功，也没有跳过整个模块。
- `.trellis/spec/backend/windows-runtime-security.md`：补充测试 fixture 必须遵循生产身份初始化、临时文件与身份权限分别处理的契约。
- `.trellis/spec/backend/project-verification.md`：明确 macOS 文件工作流与 Windows unavailable/no-publication 测试责任。
- `scripts/tasks/supported-platform-structure-assets.json`：仅刷新审阅过的 `agent_install/mod.rs` identity，并加入新带平台 guard 的 FDE/health 两个测试文件；保持规范排序，不改扫描器或排除规则。

源代码在格式化后冻结。根控在同树同步的 `FirstUseGuide.css`、`first-use-guide.spec.ts` 和 frontend First Use Guide 规范均未由本执行者修改。

## 验证

使用 `rtk` 前缀与仓库 canonical mise 任务，仅在本机 macOS 执行；日志保存在 FDE 树 `.trellis/.runtime/verification/`。

| 检查 | 结果 | 证据 |
| --- | --- | --- |
| `mise run rust:fmt:check` | PASS，exit 0 | `fde-windows-fmt-final.log` |
| `mise run supported-platform:check` | PASS，2778 current files，exit 0 | `fde-windows-platform-check.log` |
| `git diff --check` | PASS | 工具回执 |
| `mise run rust:test -- agent_install::tests` | PASS，22/22，exit 0 | `fde-windows-agent-tests.log` |
| `mise run rust:test -- fde_workspace::tests` | PASS，macOS 2/2，exit 0 | `fde-windows-workspace-tests.log` |
| `mise run rust:test -- health_admission_collect_preserves_seeded_database_native_files_and_vault` | PASS，1/1，exit 0 | `fde-windows-health-tests.log` |
| `mise run test:unit -- tests/codexWindowsUserScopeContract.test.ts` | PASS，14/14，exit 0 | `fde-windows-user-scope-contract.log` |
| 根控暂存后重跑 Windows user scope / platform surface / workstation path 三组静态合约 | PASS，40/40；已读回日志 | `fde-ci-regression-contracts.log` |
| `mise run rust:clippy` | PASS，macOS all-targets，exit 0，无 warning | `fde-windows-clippy.log` |

首次 fmt check 仅发现新增测试的两处换行，已用 `mise run rust:fmt` 修复；没有修改其他 Rust 文件。所有 source identities 在该格式化之后更新。

额外静态组合检查首次出现 2 个 5 秒 watchdog 超时，以及根控当时新增但尚未暂存的归档 `ci-followup-20260920.md` 缺 Git index mode。未放宽 watchdog、扫描器或排除规则。根控暂存后独立重跑三组 40 个检查全过；首次失败日志 `fde-windows-static-contracts.log` 保留，不能作为最终失败或静默删除的证据。这里的暂存动作由根控执行，不是本执行者越权 stage。

## 验收边界

以上源码修复与 macOS/静态检查不证明 Windows native 测试已通过。必须由根控推送新 head 后重新读取 Windows backend job 及同一 head 的 required checks；旧 run 的 `3344 passed / 7 failed / 5 ignored` 不能重标成功。新 Windows guard 的执行、真实 Shell 初始化是否满足 hosted runner 前置，以及新测试的 Windows type/lint 检查都以该新 run 为准。真实账号、客户验收、Windows 实机 UAT 和发布仍不在此报告的完成声明内。
