## 2026-08-31 迭代决策（回写，已扩范围）

本轮不新开「把 Grok 收完」的平行 Issue。产品意图继续挂在本讨论。

**本迭代做：**

1. 把官方 `grok login`、FyAgent 自管 xAI 设备码、API Key 三条路在界面上拆开（落地 [#43](https://github.com/fy-agent/fyagent/issues/43)）。官方态没有结构化 status，就保持 handoff，不读 `~/.grok/auth.json` 冒充已登录。ChatGPT 登录是另一把钥匙，这轮不做。
2. 用同一份 SuperGrok 设备码，分别投到 Claude Code、Claude Desktop、Codex、WorkBuddy（落地 [#42](https://github.com/fy-agent/fyagent/issues/42)；Codex 写入复用 [#41](https://github.com/fy-agent/fyagent/issues/41) / [#63](https://github.com/fy-agent/fyagent/issues/63)；WorkBuddy 走自己的保存）。

**本迭代不做：** Grok 安装/升级（[#31](https://github.com/fy-agent/fyagent/issues/31)、[#32](https://github.com/fy-agent/fyagent/issues/32)）、V2 额度看板、ChatGPT 登录、Qoder / TRAE 模型写入、总门卫（#133）。

不关闭 #42 / #43 整张工单，只回写 Grok 这一刀。
