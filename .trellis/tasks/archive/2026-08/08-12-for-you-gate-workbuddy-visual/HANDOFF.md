# FyAgent 控制面原型跨设备交接

## 继续方式

```powershell
git fetch origin
git switch codex/前端设计
git pull --ff-only
```

本分支只承载视觉原型、研究证据、提示词和项目连续性记录，不包含产品前端实现。不要合并到 `main`，待用户完成 v4 原型评审后再决定是否创建新的实现任务。

## 当前结论

- 当前推荐版本：v4。
- 当前视觉：深蓝灰毛玻璃、warm off-white 文字、signal blue 主动作、clear cyan 活动状态。
- 当前结构：`Agent 目录 / 模型 / Skills / MCP / 提示词 / 记忆` 六个一级入口；240px 列表、流式工作区、280px 检查器。
- 当前边界：Agent 目录表达 CAP-100 候选愿景；模型、Skills、MCP、提示词、记忆必须使用现有前端真实数据契约。
- Windows 使用右上窗口控件；macOS 使用原生标题栏，不在共享内容中复制红黄绿按钮。

## 版本历史

| 版本 | 状态 | 入口 |
| --- | --- | --- |
| v1 | 已替代 | [prototype-v1.md](./research/prototype-v1.md)、[workbuddy-for-you-gate-prototype-v1.png](./research/workbuddy-for-you-gate-prototype-v1.png) |
| v2 | 已替代 | [prototype-v2.md](./research/prototype-v2.md)、[fyagent-control-plane-prototype-v2.png](./research/fyagent-control-plane-prototype-v2.png) |
| v3 | 已替代但保留布局证据 | [prototype-v3-review.md](./research/prototype-v3-review.md) |
| v4 | 当前评审版 | [prototype-v4-review.md](./research/prototype-v4-review.md) |

## v4 六页

- [01 Agent 目录](./research/01-agent-catalog-v4.png)
- [02 模型](./research/02-models-v4.png)
- [03 Skills](./research/03-skills-v4.png)
- [04 MCP](./research/04-mcp-v4.png)
- [05 提示词](./research/05-prompts-v4.png)
- [06 记忆](./research/06-memory-v4.png)

## 设计与提示词

- [v4 Design DNA](./research/control-plane-v4-design-dna.json)
- [v4 页面内容合同](./research/control-plane-v4-page-contract.md)
- [v4 生成提示词](./research/control-plane-v4-generation-prompts.md)
- [v4 A/B/C 校准](./research/control-plane-v4-visual-variants.md)
- [v3 Design DNA](./research/control-plane-v3-design-dna.json)
- [v3 页面规范](./research/control-plane-v3-page-spec.md)
- [v3 生成提示词](./research/control-plane-v3-generation-prompts.md)
- [v1/v2 生成依据](./research/historical-generation-notes.md)

## 研究依据

- [飞书视觉评审证据](./research/feishu-visual-review-evidence.md)
- [当前前端能力盘点](./research/frontend-capability-inventory.md)
- [v4 三方专家评审](./research/v4-expert-review-synthesis.md)
- 原始飞书素材与评审包：`../../../.tmp/fyagent-visual-review/`

## 下一步

1. 逐页评审 v4，优先确认模型页字段密度、Skills/MCP 支持应用范围、提示词单条启用表达和记忆来源分组。
2. 若原型获批，在 Trellis 中完成最终规划摘要，并由用户另行明确批准进入前端实现。
3. 实现时不要直接把生成图当像素规范；以 v4 Design DNA、页面合同和当前代码数据模型为权威输入。
