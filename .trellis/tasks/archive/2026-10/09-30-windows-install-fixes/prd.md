# Windows helper 队列、Codex PackageBridge 目录身份与 OpenCode ARM64 修复

## 授权与来源

- 2026-09-30 19:20（UTC+8）用户要求按 `vm-smoke/FIX-PROPOSALS.md` 实施三项修复，建 Trellis 任务，
  写单元测试，用 fork 构建的安装包在虚拟机上复测，然后向 `fy-agent/fyagent` main 开一个 PR。
- 2026-09-30 19:29（UTC+8）用户批准：CI 全绿且虚拟机复测通过后，按合并规则合并，并单独发布 0.4.10。
  任一复测项失败则停在合并前。
- 2026-09-30 19:35（UTC+8）团队评审：ARM64 上 helper 未能拉起（`FindWindowSW` 返回 `S_FALSE`）
  更可能是 GitHub ARM runner 桌面会话不完整，本任务只为它补诊断，不作为产品修复；
  ARM 上 Grok/Claude 的状态在发布门禁中记为「环境受限，不作门禁」。
- 相关历史：fy-agent/fyagent#29 于 9/21 关闭时 4 项验收未勾选，最后评论说明 helper 崩溃/不可用场景未测。
  本任务覆盖其中漏测的部分（PR 中写 `Refs #29 (missed coverage)`，不操作该 issue）。

## 用户结果

1. Windows x64 上，Agent 目录页并行扫描时，Claude Code 和 Grok Build 显示真实状态（未安装可一键安装，
   已安装显示版本），不再因为 helper 正忙显示「当前不可用」。
2. helper 没有运行（忙到超时、Explorer 无桌面视图、启动未发出）时，卡片显示「状态未知」且保留一键安装，
   不显示「当前不可用」，也不当成未安装；日志里有一行去敏的原因。
3. Codex Desktop 在 ProgramData 目录索引恰好跨 4 KiB 块时也能安装；如果确实失败，界面提示可重试。
4. Windows ARM64 上 OpenCode Desktop 可以一键安装官方 ARM64 安装包（带版本号的官方文件）。

## 验收标准

- A1 `HelperGateLease::acquire` 在 Active 时有界等待（30 s），finish/drop 时唤醒；Quarantined 仍立即失败；
  超时返回独立的 `helper_busy` platform code。单元测试覆盖三种情况。
- A2 helper 未拉起时带独立 platform code：`shell_desktop_unavailable`（FindWindowSW 无桌面视图）或
  `helper_launch_not_invoked`；启动前失败写 `log::warn!`，只含已去敏的 code 与 message。
- A3 Grok/Claude 把这三个 code 映射为统一的「暂时无法读取」文案；`CliObservation.unconfirmed`
  使 readiness 为 `unknown`，不带 `interactive_user_unavailable`，保留 Install。单元测试覆盖。
- A4 桌面视图查找只在 `ShellDesktopUnavailable` 时重试（3 次，间隔 500 ms），发生在任何 ShellExecute 之前；
  IPC 公共错误码不变。标注为诊断/加固。
- B1 `HeldObject::recheck` 对目录只比较 volume serial + file index，普通文件保持完整比较；
  祖先变化错误设为可重试。单元测试：目录下新建 400 个条目后 recheck 通过；另一个目录对象被拒绝。
- B2 `installerErrorCopy.ts` 为 `PACKAGE_IDENTITY_MISMATCH`、`helper_busy`、`shell_desktop_unavailable` 提供文案。
- C1 Windows ARM64：对 x64 stable 别名发一次不跟随跳转的 GET，只接受
  `https://opencode.ai/files/bin/<x.y.z>/opencode-desktop-win-x64.exe`（无 query/fragment/端口/userinfo），
  拼出同版本 `opencode-desktop-win-arm64.exe`；`versionless_latest=false`，`display_version` 为该版本，
  release ID 含版本。ARM64 路径规则要求以 `-win-arm64.exe` 结尾且不含 x64/darwin/`/zh/`。正反例单测。
- 本地 TS 单测、fork 上 CI（Windows x64/ARM、macOS、frontend）全绿。
- 虚拟机复测（fork 构建、非 release 包）对照 run4/run7：x64 Grok/Claude 检测正确；Codex Desktop
  在 x64 与 ARM 上安装成功（多次）；ARM OpenCode 安装的是 arm64 构建（PE 0xAA64）；三方向导无回归。
  ARM Grok/Claude 状态记录但不作门禁。

## 排除范围

- 不新增第二条 helper 启动路由，不改提权边界，不改 Codex/Agent 安装协议。
- 不修 macOS TRAE 失败（只定位并报告）。
- 不改版本号；版本与 release notes 在独立 PR 中完成。
