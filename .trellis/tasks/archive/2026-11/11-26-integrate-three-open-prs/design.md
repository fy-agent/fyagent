# 本地集成设计

## 固定输入

仓库：[fy-agent/fyagent](https://github.com/fy-agent/fyagent)。最初预检只抓取对象到 `refs/trellis/integrate-three-open-prs/`。后续一度误启动，保留的本地分支和索引见 [暂停现场](paused-state.md)。执行只用固定 SHA，若 PR 更新则先审查新快照与差异，不静默换输入。

| 输入 | 固定 SHA | GitHub 状态 |
| --- | --- | --- |
| `main` | `82d6204908ea28d2dfb8a8af61cc7b67b65ea161` | 基线 |
| #443 | `65d3c40b576cb5a53166c37970ff566a021b191e` | CLEAN |
| #444 | `86e7ea784d809dc582c655803dc6e6165db12cdd` | BLOCKED；与 `main` 的本地两方试合并干净，不代表满足远端保护规则 |
| #198 | `367c62ec4d5a746aed4c739459fc3f38ecc80f5c` | DIRTY |

## 关键决策

1. 推荐 `main → merge #443 → merge #444 → 定向修复 #198 残差`。前两个在独立工作树用 merge commit 保留原 PR head；#198 不整支合或全量 cherry-pick，而按[剩余价值评审](pr198-review.md)选功能、保留作者来源并编写回归。不可用 `ours` 策略假造已完整采纳其历史。
2. #443/#444 的两方探针在 `scripts/tasks/supported-platform-structure-assets.json`、`src-tauri/permissions/legacy-application-commands.toml`、`src-tauri/src/commands/agent_catalog.rs` 和 `tests/renderer/platform/tauriAclContract.test.ts` 报冲突。以 #443 的 `workspace-transfer.toml` 和 `model-management-loop.toml` 拆分为基准，逐项复核 #444 新命令的注册、scope、ACL、typed ports 和浏览器 fallback；同一命令只归属一个 ACL，不恢复退役 Proxy IPC。平台清单保留双方仍有效的语义，复核现行扫描规则。
3. `main`/#198 的四处冲突留作历史证据；新版不启动这次 merge。实现只进入当前 Session 架构，保留已在主线的 `FailureFeedback`、独立 writer 门和 Claude npm 判定，不替换现有对话框或恢复协议。
4. 每个冲突以现行契约和行为回归裁决；逐项记录最终路径、保留的行为与测试。两方探针用于规划，不保证按顺序真正合并时冲突文件相同。
5. H1 启动和导航冒烟可在日常 Windows 普通账户执行，不要求 VM；worktree 不隔离用户配置，实际写入和外部服务仍遵守专项验收条件。

## 冲突逐文件裁决

不要对整文件批量选 `ours` 或 `theirs`。对未合并路径使用 `git ls-files -u -- <路径>`，按需用 `git show :1:<路径>`、`:2:`、`:3:` 查看基线、当前集成、传入 PR，再看 `git diff --cc -- <路径>`；若某个 stage 不存在，按新增/删除判断，不能假定三份都在。

| 冲突文件／相关文件 | 保留内容及检查 |
| --- | --- |
| `scripts/tasks/supported-platform-structure-assets.json` | 保留 #443 对退役路径和 IPC 的移除、#444 的扫描入口与有效新增文件；定向修复 #198 后重新核对受影响会话资产条目和最终源码，不能移植旧快照的结构摘要。 |
| `src-tauri/permissions/legacy-application-commands.toml`、`workspace-transfer.toml`、`capabilities/default.json`、`src-tauri/src/lib.rs` | 原工作区迁移命令保留在拆出的权限中；将 `list_ssh_push_profiles`、`save_ssh_push_profile`、`delete_ssh_push_profile`、`list_wsl_distros`、`preview_wsl_config_sync`、`apply_wsl_config_sync` 放在 `workspace-transfer.toml`，不得留在 legacy。仅保留当前本地主窗口能力范围；交叉核对 Tauri handler、TypeScript 调用和浏览器实现，不新增远程窗口授权。 |
| `agent_catalog.rs`、`tauriAclContract.test.ts` | 保留 #443 对 `get_global_proxy_config`、`update_global_proxy_config`、`get_proxy_access_key`、`regenerate_proxy_access_key` 的缺席断言，以及两套拆分 manifest 的唯一性检查；补 #444 六项命令的存在与权限归属。按各 PR 差量预测 `428 − 4 + 6 = 430`；以最终注册集、授权集及重复项校核为准，不只改计数，也不为通过测试削弱断言。 |
| `Page.tsx`、`ImportPackageDialog.tsx` | 若验证残差成立，让按钮和键盘的当前导入入口触发重新探测，处理过期状态；现有 `FailureFeedback` 不迁回旧视图，只回归中文提示、焦点/滚动和关闭行为。 |
| `session-migration.ts`、`import-dialog-behavior.test.tsx` | 保留已存在的 installed/writeSupported 门；按需补 `targetStoreUnidentified` 的首次启动指引和回归，不重复移植 Gemini 生产逻辑。 |
| `migrate/capability.rs`、`migrate/native/codex.rs`、`providers/codex.rs` | 拟移植只读预探测与结构化附件标题修复；原生执行的身份/并发/版本门不变，无权、无身份和不支持版本仍 fail closed。Claude 实现已在主线，不改 `user-helper/src/claude.rs`。 |

每项记录：输入 PR、冲突文件与最终文件、双方各保留/删除的语义、是否影响权限或落盘、对应验证项 ID。无法证明权限或写入门不扩大时停止该 merge，不临时扩大 ACL、跳过测试或把原生检查改为 UI 检查。

macOS 与 Windows 共享行为一起交付，不删除 macOS 路径；在本机只保留未测边界。由维护者在授权后用 [Issue 草稿](macos-issue-draft.md) 完成确切候选 SHA 的验收，再走仓库保护流程。

## 失败与回滚

检查或编译失败时保留独立 worktree 和日志，不把失败视作放行；遇到无把握的冲突保留现场并记录。若决定放弃正在进行的单次合并，仅在该 worktree 执行 `git merge --abort`。人工复核结束后，确认无未保存资料再移除临时 worktree；不对当前 checkout 做 reset、stash 或 clean。
