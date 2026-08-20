# FyAgent 控制面 v4 页面内容合同

## 共同壳层

- 一级导航固定：`Agent 目录 / 模型 / Skills / MCP / 提示词 / 记忆`。
- 结构固定：240px 左侧连续列表、流式中间工作区、280px 右侧检查器。
- 列表行只承担名称与一个关键状态；详情字段进入中栏；范围、状态和上下文动作进入右栏。
- 页面最多一个实心蓝主动作；危险操作和高级字段进入二级面板。
- “Agent”只指 CAP-100 候选目录；“应用”指当前已有 AppId，二者不混用。

## 01 Agent 目录

- 左栏：Qwen Code（P0 / 待适配）、WorkBuddy（首发候选 / 可配置）、QClaw（首发候选 / 待适配）、Codex（首发兼容 / 可配置）、Claude Code（首发兼容 / 可配置）。
- 中栏：候选的目录身份、接入方式、当前支持状态、规划能力；不使用伪运行指标。
- 右栏：目录状态、接入要求和下一步。Qwen Code/QClaw 的动作是 `查看接入要求`，不是伪装可执行的 `开始配置`。
- 低频：来源、完整权限、依赖、会话与高级配置进入二级详情。

## 02 模型

- 顶部应用上下文：当前选择 `Codex`；页内视图 `接入源 / 路由 / 用量`，默认 `接入源`。
- 左栏列表必显：供应商名称、认证方式或分类、真实配置状态；仅在有真实数据时显示健康/用量。
- 中栏详情：名称、分类、认证方式、Base URL（可脱敏/缩略）、当前模型、API 格式；App 专属的模型映射按当前上下文显示。
- 右栏状态：当前应用、当前使用/已加入配置、代理接管、故障转移、用量摘要；动作 `启用/切换`、`测试`、`编辑`、`复制`。
- 主动作：`添加接入源`。
- 低频：密钥、完整模型清单、请求日志、价格、端点自动选择、请求覆盖和高级推理元数据。

## 03 Skills

- 页内视图：`已安装 / 发现 / 来源 / 备份`，默认 `已安装`。
- 左栏列表必显：名称、来源（本地或 owner/repo）、更新状态、启用应用数量。
- 中栏详情：名称、描述、目录、仓库、分支、README、安装/更新时间、更新状态。
- 右栏：`启用于应用`，只显示 Claude、Codex、Gemini、Grok Build、OpenCode、Hermes 的连续开关行。
- 主动作：`发现 Skills`。
- 次要操作：检查更新、更新、卸载；扫描未管理、ZIP、恢复备份进入溢出菜单。
- 禁止：读取/编辑/附件等不存在的能力标签；Qwen/WorkBuddy/QClaw 分配开关。

## 04 MCP

- 页内视图：`服务 / 分配`，默认 `服务`。
- 左栏列表必显：名称、传输方式、描述或标签、启用应用数量。
- 中栏详情：ID、名称、描述、传输方式；stdio 显示 command/args/cwd/env 键名，HTTP/SSE 显示 URL/headers 键名；补充 tags、homepage、docs。
- 右栏：`启用于应用`，只显示 Claude、Codex、Gemini、Grok Build、OpenCode、Hermes。
- 主动作：`添加 MCP`；次动作 `导入现有`、`编辑`；删除进入溢出菜单。
- 禁止：连接健康、工具数量、读取/写入权限、候选 Agent 开关。

## 05 提示词

- 顶部应用上下文：当前选择 `Codex`。
- 左栏列表必显：启用开关、名称、描述、更新时间；同一应用最多一条启用。
- 中栏编辑：名称、描述、Markdown 正文；保存为中栏次操作。
- 右栏：当前应用、目标文件 `AGENTS.md`、启用状态、更新时间；内部 ID、创建时间和删除进入二级。
- 主动作：`新建提示词`。
- 目标文件映射：Claude/Claude Desktop → `CLAUDE.md`；Codex/Grok Build/OpenCode/OpenClaw → `AGENTS.md`；Gemini → `GEMINI.md`；Hermes → `SOUL.md`。
- 禁止：共享模板、基础/项目/临时优先级、版本历史、跨 Agent 分配。

## 06 记忆

- 页内视图：`OpenClaw / 每日记录 / Hermes`。
- 左栏分组：OpenClaw 固定文件（AGENTS/SOUL/USER/IDENTITY/TOOLS/MEMORY/HEARTBEAT/BOOTSTRAP/BOOT）、Daily Memory、Hermes Agent/User。
- 列表必显：名称、来源类型和一个真实状态；Daily 显示日期/大小，Hermes 显示启用状态。
- 中栏：Markdown 编辑/预览；Daily 可搜索/创建/保存/删除；Hermes 显示 Agent/User 内容。
- 右栏：来源归属、本地位置、是否存在或启用状态；Daily 的日期/大小/修改时间；Hermes 的字符用量/上限和开关。
- 主动作：`新建今日记录`，只作用于 OpenClaw Daily Memory。
- 禁止：跨 Agent 可见范围、向量检索、自动抽取、云同步、跨设备或结构化实体。
