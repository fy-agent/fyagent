## 2026-08-31 迭代决策（Grok 切片，已扩范围）

#42 仍是「同一接入源投给多个 Agent」的总单，**不关闭本 Issue**。

本迭代把同一份 SuperGrok 扫码，分别投到现在能写的地方：

- Claude Code
- Claude Desktop（旧界面完成即可）
- Codex（新界面 Change Plan 开窄口）
- WorkBuddy（走它自己的模型保存，不是 Codex 那扇门）

每一家一张独立预览/保存。一家失败不谎报另一家已应用。

不做：Qoder / TRAE 模型写入、ChatGPT 登录、安装升级 Grok。

登录边界见 [#43](https://github.com/fy-agent/fyagent/issues/43)。产品意图见 [#106](https://github.com/fy-agent/fyagent/discussions/106)。
