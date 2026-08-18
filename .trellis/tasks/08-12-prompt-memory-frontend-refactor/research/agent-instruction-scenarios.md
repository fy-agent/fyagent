# Agent 长期指导文件与内置规则场景调研

## 1. 结论

FyAgent 的“提示词”不是一次聊天的 Prompt，而是写入 Agent 长期指导入口的**可组合规则模块**。内置规则应满足三个条件：

1. 跨任务反复生效，值得每次会话加载。
2. 不依赖某个仓库的虚构命令、目录或架构。
3. 可以单独启停和分配目标，不需要塞成一个巨型文档。

本轮先从本机已有 Codex `AGENTS.md`、Claude Code `CLAUDE.md`、OpenClaw workspace 和 Hermes `SOUL.md` 的规则域提炼匿名化结构，再用官方文档校准。没有复制用户私人原文。

## 2. 本机已有规则域

| 规则域 | 本机证据 | 适合成为内置规则 |
| --- | --- | --- |
| 沟通与输出 | Codex/Claude 全局文件均有语言、结论优先、提问方式 | 中文与回复风格 |
| 目标与完成 | 现有规则强调范围、依赖、验收、真实证据 | 目标、边界与完成证据 |
| 上下文加载 | 多工具存在全局/项目规则和 memory routing | 先读项目上下文 |
| 规划设计 | 现有规则要求方案比较、依赖和验证路线 | 规划与方案对比 |
| 实现纪律 | 现有规则强调复用、最小改动、保护无关内容 | 最小改动与真实验证 |
| 审查与故障 | 本机规则区分 code review、根因诊断和回归 | 代码审查；工具失败先定位根因 |
| 记忆连续性 | Codex、Claude、OpenClaw 均有读/写回长期记忆规则 | 记忆读取与写回 |
| 自动运行 | OpenClaw 有 HEARTBEAT 与多实例运行上下文 | 定时任务与心跳边界 |

## 3. 官方合同对照

| 来源 | 官方合同 | 对 FyAgent 的直接含义 |
| --- | --- | --- |
| [AGENTS.md 开放格式](https://agents.md/) | 项目可写概览、构建、测试、风格和 PR 规则；嵌套文件让就近规则生效 | 通用库提供规则骨架，项目事实必须由真实仓库补充 |
| [OpenAI Codex AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md) | 启动时按层级加载全局与项目指导；就近规则优先 | 目标需要 scope 和稳定组合，不能用单条覆盖模拟 |
| [Claude Code Memory](https://code.claude.com/docs/en/memory) | `CLAUDE.md` 保存持续指令；自动记忆保存模型积累的事实；局部流程可使用 rules/Skills | Prompt 与 Memory 必须分开，常驻规则与工作结果不能混写 |
| [Gemini CLI GEMINI.md](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/gemini-md.md) | 支持全局、工作区与按需上下文，并合并发现的上下文 | 需要组合预览、顺序和文件未创建状态 |
| [GitHub Copilot repository instructions](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions) | 区分仓库级、路径级和 Agent 指令；重点覆盖理解、构建、测试、验证 | 数据模型要预留 scope，通用种子不能伪造项目命令 |
| [GitHub customization library](https://docs.github.com/en/copilot/tutorials/customization-library) | 示例把调试、审查、测试、PR、文档等拆成独立能力 | 专项场景应是可组合规则，不是一条全能 Prompt |

## 4. 首批内置规则

| ID | 名称 | 类型 | 来源 | 默认目标 |
| --- | --- | --- | --- | --- |
| `chinese-communication` | 中文与回复风格 | 角色与表达 | 本机规则提炼 | 7 个目标文件 |
| `goal-boundary-evidence` | 目标、边界与完成证据 | 验收规则 | 本机规则提炼 | 7 个目标文件 |
| `context-first` | 先读项目上下文 | 通用规则 | 本机规则提炼 | 7 个目标文件 |
| `planning-design` | 规划与方案对比 | 执行流程 | 本机规则提炼 | Codex、Claude Code |
| `implementation-discipline` | 最小改动与真实验证 | 执行流程 | 本机规则提炼 | 4 个编码目标 |
| `code-review` | 代码审查 | 验收规则 | 官方基础模板 | 4 个编码目标 |
| `memory-continuity` | 记忆读取与写回 | 记忆规则 | 本机规则提炼 | 6 个已发现记忆/历史来源的目标 |
| `tool-troubleshooting` | 工具失败先定位根因 | 执行流程 | 本机规则提炼 | 7 个目标文件 |
| `heartbeat-boundary` | 定时任务与心跳边界 | 自动运行 | 本机规则提炼 | 2 个 OpenClaw workspace |

默认只启用“中文与回复风格”和“目标、边界与完成证据”，用于演示多条并存。其余规则已经有推荐目标，但默认关闭，由用户决定是否加入组合。

## 5. 不进入常驻规则库

- 某个仓库的真实构建命令、目录、端口、依赖版本和架构事实。
- 只用于一次任务的长步骤、素材或脚本。
- 密钥、凭据、私人记忆正文和完整会话。
- 需要平台硬执行的权限或生命周期动作；这些属于产品设置、Hook 或 adapter。
- 大段教程与参考知识；应该按需检索或放入 Skill。

## 6. 目标与组合

当前本机目标不是“七个应用”，而是 7 个真实资源：

- Codex 全局 `~/.codex/AGENTS.md`。
- Claude Code 全局 `~/.claude/CLAUDE.md`。
- Gemini CLI 全局 `~/.gemini/GEMINI.md`（当前未创建）。
- OpenCode 全局 `~/.config/opencode/AGENTS.md`（当前未创建）。
- OpenClaw 默认 workspace `AGENTS.md`，覆盖 `main + utility`。
- OpenClaw 群聊 workspace `AGENTS.md`，覆盖 `group_liaison`。
- Hermes 全局 `~/.hermes/SOUL.md`。

同一目标允许多条规则。建议稳定顺序：通用规则 → 角色与表达 → 执行流程 → 验收规则 → 记忆/自动运行 → 自定义。真实写入只替换 FyAgent 受管区块，目标文件其他内容保留。

## 7. 对页面和后端的约束

- 页面右栏名称为“注入目标”，展示实例/workspace、路径和存在状态。
- 路径共享比应用名更重要；OpenClaw `main + utility` 只生成一个写入任务。
- 新建规则默认关闭且无目标；启用规则至少有一个目标。
- compose 需要预览、稳定顺序、来源规则、hash 冲突和逐目标结果。
- Gemini/OpenCode 目标文件未创建时不能显示成“已配置”；只在用户确认同步时创建。
