# V3.1 当前执行主体权威 — Gemini 前端主实施

## Authority

- 日期：2026-08-26
- 权威来源：用户在最新消息中明确要求“前端实现必须交给 Antigravity / Gemini 3.7”，并逐项重申 A-to-A 分工。
- Antigravity / Gemini 3.7：01–11 前端页面、布局、层级、密度、位置、文案、状态反馈、前端状态逻辑、局部 UI variant 与相关前端测试的实施 owner；所有 UI 返工回到该路由。
- Cursor / Grok 4.6：后端、共享组件边界、技术调研、是否过度复杂的判断，以及 Gate A / Final Gate 强制评审；Grok Build / `grok-4.6` 为候补通道。
- Codex / `gpt-5.6-sol` / `max`：总调度、任务与分支治理、监工催办、运行验证、桌面操作、截图、透明底图资产、复现包、最终交接和经独立授权的对外发布；不直接编写页面 JSX/CSS，不接管 Gemini 的 UI 返工。
- Grok Bot App：桌面观察员和监工。
- 分支：只使用 `codex/frontend-interaction-v3-1-20260826`，不动 `main`。

## Superseded Executor Route

`executor-authority-2026-08-26-codex-primary.md` 及其派生规划中“Codex 主实施前端、Gemini 仅协作”的结论已被本次最新用户消息标记为 `SUPERSEDED_DO_NOT_EXECUTE`。

该覆盖只改变执行主体。V3.1 的全部产品要求、11 张高保真原型、扫描语义、真实数据合同、Grok 门槛、工程质量、分支边界与禁止事项继续完整有效。

## Planning State

- 本决定不授权产品代码修改。
- 上一轮基于 Codex 主实施路由的 Grok Round 5 `PASS` 标记为 `STALE_REVIEW_INPUT_DRIFT`。
- 活动规划更新后必须重新通过 Grok 4.6 只读评审；用户看到最新摘要后的下一条明确批准，才允许运行 `task.py start`。
