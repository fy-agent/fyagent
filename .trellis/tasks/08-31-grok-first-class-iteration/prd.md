# Finish Grok login and SuperGrok placement into supported tools

先读 [summary.md](./summary.md)。用例总表：[use-cases.md](./use-cases.md)。亲测勾选：[research/hil-matrix.md](./research/hil-matrix.md)。

## Goal

分清三种 Grok 登录；扫码一次 SuperGrok，能用到 Claude Code、Claude Desktop、Codex 和 WorkBuddy。William 在 Windows 和 Mac mini 上亲自走完才算完成。

## Background

- 意图：[Discussion #106](https://github.com/fy-agent/fyagent/discussions/106)。登录回写 [#43](https://github.com/fy-agent/fyagent/issues/43)，投放回写 [#42](https://github.com/fy-agent/fyagent/issues/42)。
- 2026-08-31：William 决定关联投放一起做，不拆成「先只做 Codex」。
- 子任务：登录路标；Claude/Desktop/Codex 投放；WorkBuddy 投放。

## Confirmed facts

登录（`08-31-grok-login-trichotomy/research/current-login-surfaces.md`）：

- 三条路散在三处。官方登录只交接，不验证。没有 Grok 登录状态命令。不能用 `~/.grok/auth.json` 证明已登录。
- ChatGPT 登录是 `codex_oauth`，和 SuperGrok 扫码不是一把钥匙。

投放（`08-31-grok-supergrok-to-codex/research/current-supergrok-codex-path.md`）：

- SuperGrok 扫码是共用认证中心。旧界面已能绑 Claude Code / Claude Desktop / Codex 的 `xai_oauth` 预设。
- 新界面 Change Plan / Quick Setup 只认 API 钥匙，会拒绝托管扫码。有没有账号，页面长得一样。
- Claude Desktop 不在新界面 Agent 目录里，亲测走旧界面。

WorkBuddy：

- 目录允许自己换模型。保存走自己的 Change Plan（地址 + 钥匙 + 模型名），不是 Provider Quick Setup。
- 现在没有 `xai_oauth` 预设。Qoder 不能配第三方模型；TRAE 不能代写模型。

## Requirements

- R1. 三种登录路标分开。官方登录不说已登录。
- R2. SuperGrok 扫码仍只在认证中心。
- R3. 同一份已登录 SuperGrok，能分别写进 Claude Code、Claude Desktop、Codex。每家一张独立预览/保存。失败不连累别人。
- R4. 同一份脑子能写进 WorkBuddy。优先用已扫码账号，不要无故再要一把钥匙。走 WorkBuddy 自己的保存。
- R5. Qoder / TRAE 不写第三方模型。ChatGPT 登录这轮不做。
- R6. 双机亲测全部路径。密码不进仓库。

## Acceptance Criteria

- [ ] AC1. 人能分清官方登录、扫码、API 钥匙。
- [ ] AC2. 官方登录不出现「已验证」。Claude 原来能验证的路还在。
- [ ] AC3. SuperGrok → Claude Code、Claude Desktop、Codex 都能先看再改再检查（Desktop 可在旧界面完成）。
- [ ] AC4. SuperGrok → WorkBuddy 能保存并回读。
- [ ] AC5. 一家失败不谎报另一家成功。
- [ ] AC6. 不关 #42 / #43 整张工单。#141 B7 按有没有改草稿标记。
- [ ] AC7. Windows 和 Mac mini 都按 `research/hil-matrix.md` 走完。

## Out of scope

- 安装升级 Grok（#31、#32）
- 新界面额度看板
- ChatGPT 登录
- Qoder / TRAE 模型写入
- 总门卫
- 写 `~/.grok/auth.json` 冒充登录
