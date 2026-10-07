# Design

## (a) Helper gate 与未运行结果

- 位置：`codex_desktop/platform/windows/helper.rs`。
  - `HelperGateState<R = HelperLifetime>` 改为泛型只是为了在测试里用 `()` 代替真实 lifetime，生产类型不变。
  - `enter_helper_gate` / `leave_helper_gate` 是纯同步逻辑：`Mutex` + 静态 `Condvar`。Active 时
    `wait_timeout` 到截止时间；Quarantined 立即返回原来的 quarantine 错误；`retain_quarantined_lifetime`
    之后 `notify_all`，让等待者马上失败。
  - `helper_not_invoked_error(ProcessLaunchError)` 按原因加 platform code；`fail_before_admission` 与 gate 超时写一行 `log::warn!`。
- 位置：`platform/process_launch.rs` 新增 Windows 专属 `ProcessLaunchError::ShellDesktopUnavailable`，公共 code 与
  `InteractiveUserUnavailable` 相同，避免改 IPC 合同。`platform/windows/interactive_user.rs` 在 `FindWindowSW`
  失败时返回它，`retry_shell_desktop_lookup` 只对它做有界重试。
- 位置：`services/tooling.rs` 定义 `WINDOWS_HELPER_UNCONFIRMED_MESSAGE` 与 code 判定；`grok.rs`、`claude.rs`
  （新增 `ClaudeLifecycleError::HelperUnconfirmed`）映射；`agent_install/cli.rs` 以精确文案识别为 `unconfirmed`；
  `agent_install/mod.rs::cli_readiness_from_observation` 映射为 `Unknown`。
- 安全：只有「helper 没运行」的三个 code 会变成 unknown；MayHaveLaunched、pipe/协议错误保持原映射，
  Claude 注释里的「不确定结果不得触发二次安装」保持成立。Install 按钮仍经过 helper preflight 重新观测。

## (b) PackageBridge 目录身份

- `NativeFileIdentity::same_object(current, kind)`：Directory 比较 volume serial + file index；RegularFile 全字段。
- 只改 `HeldObject::recheck`（ProgramData 祖先、bridge root/version 目录）。orphan 清理和 leaf 比较不动。
- 替换目录会改变 file index，因此防替换能力不变；ACL/descriptor 校验保留。

## (c) OpenCode Windows ARM64

- `agent_install/fetch.rs::fetch_redirect_location`：一次 GET，不跟随，要求 3xx + Location，经
  `resolve_redirect` 转绝对 URL 并检查 allowlist。
- `agent_install/sources/opencode.rs::resolve_opencode_windows_arm64_from_x64_redirect`：严格解析版本，拼 ARM64 URL，
  构造 `ResolvedDesktopSource`（版本进入 release ID）。
- `agent_install/desktop.rs::resolve_opencode`：只有 (Windows, Aarch64) 走这条路，其余不变。
- 下游 `windows.rs` 的 ARM64 PE 准入与签名者检查已存在，不改。

## Spec 更新

- `external-agent-sources.md`、`frontend/agent-directory.md`：OpenCode Windows ARM64 由「不支持」改为版本化官方文件。
- `codex-desktop-installer.md`：目录祖先身份规则。
- `windows-agent-runtime-security.md`：helper 队列、未运行 code 与 readiness unknown。
