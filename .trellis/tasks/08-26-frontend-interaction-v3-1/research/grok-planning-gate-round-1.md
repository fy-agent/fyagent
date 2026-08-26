# Grok 4.6 Planning Gate — Round 1

> 历史门槛记录。视觉、状态、负面断言和几何要求继续有效。最新用户决定再次确认 UI 返工归 Gemini、Codex 不接管页面修复；当前唯一路由见 `executor-authority-latest-gemini-primary.md`。本文件中间曾出现的 Codex-primary 解释均已失效。

## Route

- Requested primary: Cursor / Grok 4.6
- Cursor CLI: authentication required
- Authorized fallback used: Grok Build / `grok-4.6`
- Session: `01a03abd-e8cd-7b60-b00f-88eb2f7380aa`
- Termination: `max turns reached`

## Verdict

`CHANGES_REQUIRED`

该轮在最大轮次用尽前形成了明确阻塞项，因此没有有效 `PASS`。

## Required Fixes

1. 先固化 01-11 逐页差异表，再进入实现。
2. 明确扫描 `idle / scanning / complete / error` 四种 UI 合同。
3. 将旧测试里的自造状态与解释正向断言改为缺席断言。
4. 为未安装、未知、读取失败与环境不可用增加 DOM 级负面断言。
5. 为 Prompts / Memory 增加结构几何验收，覆盖 pane 数量、顺序、宽度、主动作位置和 overflow。
6. Codex 发现 UI 缺陷后生成复现包并退回 Gemini，Codex 不接管页面修复；后端、共享组件边界、数据语义与复杂度问题由 Grok 给出精确合同和 owner。

## Evidence

- `src/v2/pages/agents/AgentDirectory.tsx:200-257`：额外眉题、说明、进度计数、取消说明、自造状态与全目录渲染。
- `src/v2/pages/agents/AgentConfiguration.tsx:57-78`：额外 identity 说明与扩展返回文案。
- `tests/v2/pages/agents/Page.test.tsx:397-423`：旧状态和未安装项仍由测试正向锁定。
- `tests/v2-browser/agents-v3.spec.ts:154-155`：旧解释仍由浏览器测试正向锁定。
- `design.md` 与 `implement.md` 第一版缺少冻结 idle/error UI 与 Prompts/Memory 几何门槛。

## Applied Plan Changes

- PRD 增加负面 DOM 断言与页面 10/11 几何验收。
- Design 固化四种扫描 UI 状态、目标视口几何检查与 Gemini UI 返工责任；Codex 只负责复现、调度和验证。
- Implement 增加 11 页差异表硬门、旧测试反转和结构几何测试。

Round 2 需在 Gemini 逐页审计落盘后重新执行。
