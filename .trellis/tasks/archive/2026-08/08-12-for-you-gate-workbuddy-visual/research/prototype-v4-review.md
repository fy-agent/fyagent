# FyAgent 跨平台控制面高保真原型 v4

## 评审入口

| 页码 | 一级页面 | 原型文件 | 当前能力依据 |
| --- | --- | --- | --- |
| 01 | Agent 目录 | [01-agent-catalog-v4.png](./01-agent-catalog-v4.png) | CAP-100 候选目录；P0/候选/兼容与运行时状态分离 |
| 02 | 模型 | [02-models-v4.png](./02-models-v4.png) | Provider、认证、API 格式、当前配置、代理、故障转移、用量 |
| 03 | Skills | [03-skills-v4.png](./03-skills-v4.png) | InstalledSkill、仓库/本地来源、更新状态、真实支持应用 |
| 04 | MCP | [04-mcp-v4.png](./04-mcp-v4.png) | stdio/HTTP/SSE、命令/参数/元数据、真实支持应用 |
| 05 | 提示词 | [05-prompts-v4.png](./05-prompts-v4.png) | 按 App CRUD、单条启用、Markdown、目标文件映射 |
| 06 | 记忆 | [06-memory-v4.png](./06-memory-v4.png) | OpenClaw 工作区/每日 Markdown 与 Hermes MEMORY/USER |

## 与 v3 的关键差异

- 浅雾白玻璃改为较深蓝灰毛玻璃，主/次/辅助文字保持三档高可读对比。
- 保留 v3 的六个一级入口、三栏布局、胶囊导航和跨平台 Windows 壳层示意。
- 模型页删除五候选 Agent 全局路由，改为当前 App 的 Provider 接入源与状态。
- Skills 删除“读取/编辑/附件”，改为描述、目录、仓库、分支、更新状态和真实应用开关。
- MCP 删除权限识别和连接健康，改为真实传输配置、元数据和应用投放。
- 提示词删除共享模板/优先级/跨 Agent 分配，明确当前应用、目标文件和一次启用一条。
- 记忆删除跨 Agent 可见范围，改为 OpenClaw 文件、Markdown 编辑器和本地来源详情。

## 专家与技能影响

- 产品经理评审决定了“候选 Agent”与“当前应用”的术语和能力边界。
- 代码审计为每页列表/详情字段提供组件、类型和 API 锚点。
- 视觉评审把参考图拆成深蓝灰基底、三档 pane、warm off-white 文本和 signal blue/cyan 语义。
- `claude-design-principles` 促成 A/B/C 三种模型页校准；A 被选为六页母版。
- `design-dna` 固化可复用 token；`imagegen` 负责逐页生成、单点编辑和工作区保存。

## A/B/C 校准

- [A｜参考忠实版](./v4-calibration-model-a.png)：采用，最接近用户参考且信息层级清晰。
- [B｜深石墨版](./v4-calibration-model-b.png)：未采用，过于接近传统开发工具。
- [C｜雾蓝版](./v4-calibration-model-c.png)：未采用，长内容页存在再次泛白风险。

## 输出验证

- 六张最终原型均为 1586×992，尺寸和比例一致。
- 生成方式：Codex 内置 `image_gen`；v4 模型页为视觉母版，其余页面逐张生成。
- 证据等级：`generated_prototype`，不是 `runtime_screenshot` 或 `pixel_diff`。
- `git diff -- src assets/fyagent.png` 无输出；本轮未修改产品前端和品牌主文件。

## 文件校验

| 文件 | SHA-256 |
| --- | --- |
| 01-agent-catalog-v4.png | `DCB40F7DB1EC1D768CD3C070760FA066684D52F2B3E704D467A33EAC45CCC581` |
| 02-models-v4.png | `89660B217959440561DAAE015DC5AD24DD27DABC1D5CD3A6BF9C39F5FA45079D` |
| 03-skills-v4.png | `6E08A9BAE97D8D5D7B8593E380622E125CC9A9EFC79B0C05AAF5A9D128021192` |
| 04-mcp-v4.png | `0E9A0AEF2CED4864592C231CCA2E9665BFFF1B560DEE56D6EA8C35CEDB73D4FB` |
| 05-prompts-v4.png | `DBED503F7F77C557A7553DAFA07D68A35296177CEDFB6F85A4F216D288028780` |
| 06-memory-v4.png | `7E0B34BACC18946CCE05508C86DC277B2BEE3229E6853747B7CF846ADAC339DC` |

## 相关依据

- [三方专家评审](./v4-expert-review-synthesis.md)
- [v4 页面内容合同](./control-plane-v4-page-contract.md)
- [v4 Design DNA](./control-plane-v4-design-dna.json)
- [v4 三种视觉方向](./control-plane-v4-visual-variants.md)
- [v4 生成提示词](./control-plane-v4-generation-prompts.md)
