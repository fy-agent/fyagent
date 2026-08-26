# Grok 强制过程门补充

## Authority

- 日期：2026-08-26
- 来源：用户最新目标补充。
- 核心规则：Grok 不是只在最终收口时抽查，而是规划、Wave A、Wave B/全量 diff 的强制过程门。没有 Grok 明确 `PASS`，对应阶段不得继续、不得冻结候选、不得截图汇报、不得对外宣称高保真完成。

## Mandatory Checks

Grok 每个实施门槛至少检查：

1. 原型中不存在的大标题、小标题、副标题和解释文字。
2. 为填满页面而增加的卡片、分组、说明、状态标签和操作入口。
3. “未确认”及同类自造状态与解释。
4. 左右位置颠倒、横向长条被压成短条或短卡片。
5. 记忆与提示词模块只做表面换色或微调间距，没有完成结构级重构。

任一项存在即为 `CHANGES_REQUIRED`；不接受带 Blocking/Major 的条件通过，也不允许以测试通过代替原型一致性审查。

## Stage Locks

1. Planning Gate：规划与真实能力合同通过后，才可请求用户批准进入实现。
2. Gate A：Pages 01–06 完成后立即停下并由 Grok 评审；`PASS` 前禁止开始 Pages 07–11。
3. Final Gate：Pages 07–11 与完整 diff 完成后由 Grok 评审；`PASS` 前禁止进入 Codex 最终验证、候选冻结、截图汇报或完成口径。
4. 任何返工后必须重新进入同一门槛，不允许沿用旧 verdict。

## Rework Routing

- UI、布局、样式、文案、前端状态和 UI 测试问题：退回 Antigravity / Gemini 3.7。
- Codex 自有的调度、桌面、截图、透明资产、证据、交接或消息问题：退回 Codex / `gpt-5.6-sol` / `max`。
- 后端、共享组件边界、数据语义或复杂度问题：由 Grok 给出精确合同和明确 owner。
- Codex 不得借返工名义接管页面 JSX/CSS。

## Evidence Contract

每次 Grok 门槛输入必须包含：原型编号到页面/文件映射、diff 文件清单、原型外文案与状态的负面扫描、目标视口运行截图或几何证据、相关测试结果、未决项和实际模型路由。Grok verdict 必须包含 `Blocking / Major / Complexity / Evidence / Required fixes`。
