# FyAgent 用户手册

这份手册按实际任务组织。第一次使用，从第一章顺着做即可；已经知道自己要解决什么，也可以从下面的“按问题找入口”直接跳转。

> 部分旧截图仍在等待用真实 FyAgent 界面重拍，操作名称和正文以当前版本为准，不要把图里的 `CC Switch` 当成当前产品身份。

## 按问题找入口

- 还没装好 FyAgent：看[安装说明](./1-getting-started/1.2-installation.md)。
- 想先跑通一个供应商：看[五分钟快速上手](./1-getting-started/1.4-quickstart.md)。
- 需要安装或升级 Claude Code 等工具：看[工具安装](./2-agent-tools/2.1-install.md)和[冲突诊断](./2-agent-tools/2.2-update-diagnose.md)。
- 想集中管理接口和模型：从[添加供应商](./3-providers/3.1-add.md)开始。
- 想使用 MCP、Prompts 或 Skills：进入[扩展功能](#4-扩展功能)。
- WorkBuddy 模型列表需要写入本地配置：看[WorkBuddy 模型配置](./4-extensions/4.6-workbuddy.md)。
- 请求不稳定或需要统计用量：进入[代理与高可用](#5-代理与高可用)。
- 配置没生效：先看[常见问题](./6-faq/6.2-questions.md)和[环境变量冲突](./6-faq/6.4-env-conflict.md)。

## 1. 快速入门

- [1.1 认识 FyAgent](./1-getting-started/1.1-introduction.md)
- [1.2 下载与安装](./1-getting-started/1.2-installation.md)
- [1.3 界面说明](./1-getting-started/1.3-interface.md)
- [1.4 五分钟快速上手](./1-getting-started/1.4-quickstart.md)
- [1.5 个性化设置](./1-getting-started/1.5-settings.md)

## 2. Agent 工具

- [2.1 安装 Agent 工具并查看版本](./2-agent-tools/2.1-install.md)
- [2.2 升级工具并诊断安装冲突](./2-agent-tools/2.2-update-diagnose.md)

## 3. 供应商

- [3.1 添加供应商](./3-providers/3.1-add.md)
- [3.2 切换供应商](./3-providers/3.2-switch.md)
- [3.3 编辑供应商](./3-providers/3.3-edit.md)
- [3.4 排序、复制与删除](./3-providers/3.4-sort-duplicate.md)
- [3.5 用量查询](./3-providers/3.5-usage-query.md)
- [3.6 Claude Desktop](./3-providers/3.6-claude-desktop.md)

## 4. 扩展功能

- [4.1 MCP 服务](./4-extensions/4.1-mcp.md)
- [4.2 Prompts](./4-extensions/4.2-prompts.md)
- [4.3 Skills](./4-extensions/4.3-skills.md)
- [4.4 会话](./4-extensions/4.4-sessions.md)
- [4.5 工作区与记忆](./4-extensions/4.5-workspace.md)
- [4.6 WorkBuddy 模型配置](./4-extensions/4.6-workbuddy.md)

## 5. 代理与高可用

- [5.1 本地代理服务](./5-proxy/5.1-service.md)
- [5.2 应用路由](./5-proxy/5.2-routing.md)
- [5.3 故障转移](./5-proxy/5.3-failover.md)
- [5.4 用量统计](./5-proxy/5.4-usage.md)
- [5.5 模型测试](./5-proxy/5.5-model-test.md)

## 6. 常见问题

- [6.1 配置文件与存储位置](./6-faq/6.1-config-files.md)
- [6.2 常见问题解答](./6-faq/6.2-questions.md)
- [6.3 Deep Link 导入](./6-faq/6.3-deeplink.md)
- [6.4 环境变量冲突](./6-faq/6.4-env-conflict.md)

这份手册描述当前仓库里的实际行为。安装包名称、签名和可信度会随发布变化，请以对应的 [GitHub Release](https://github.com/fy-agent/fyagent/releases) 及其证据为准。
