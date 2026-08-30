## 2026-08-31 迭代决策（Grok 切片）

#43 仍是「官方订阅逐厂商准入」的总单，本迭代只做 Grok 这一刀，**不关闭本 Issue**。

对齐本 Issue 的验收：

- 官方态：`grok login` / `grok logout`，没有已审查的结构化 status，标为 assisted / handoff，不把打开终端写成已验证。
- 设备码：继续走 FyAgent 认证中心的 xAI OAuth，不读、不写 `~/.grok/auth.json`。
- API Key：第三条路，不和上面两条抢文案。
- 没有官方依据就不做 token relay，也不用额度接口反推登录成功。

产品意图见 [#106](https://github.com/fy-agent/fyagent/discussions/106)。同一订阅分别投到 Claude Code、Claude Desktop、Codex、WorkBuddy 见 [#42](https://github.com/fy-agent/fyagent/issues/42)。
