# SUPERSEDED_DO_NOT_EXECUTE — V3.1 Gemini 独占执行路由（历史）

> 本文件是较早一轮 Gemini 路由记录，因经历过后续覆盖而仍保留 `SUPERSEDED_DO_NOT_EXECUTE` 历史标记，不得直接用于派工。用户最新消息再次明确 Gemini 持有全部前端实施与 UI 返工；当前唯一权威为 `executor-authority-latest-gemini-primary.md`。

## Authority

- 日期：2026-08-26
- 权威来源：用户消息开头、分工段和结尾重复给出的直接指令。
- 前端实现 owner：Antigravity / Gemini 3.7。
- 强制评审门槛：Cursor / Grok 4.6；Cursor CLI 未认证时使用用户已指定的 Grok Build / `grok-4.6` 候补。
- Codex：总调度、任务与分支管理、验证、桌面操作、截图、透明底图资产、复现包、最终交接与经单独授权的对外发布。
- Grok Bot App：桌面观察员与监工。

## Superseded Clause

`source-feishu-verbatim.md` 中嵌入式提示词的 Codex 前端执行主体段标记为 `SUPERSEDED_DO_NOT_EXECUTE`。该覆盖只改变执行分工；V3.1 的产品要求、原型、交互规则、真实数据约束、Grok 门槛、分支边界与禁止事项全部继续有效。

## Enforceable Routing

1. Pages 01-06、Pages 07-11、局部 shared variant、前端状态投影和相关前端测试由 Gemini 3.7 实施。
2. Grok 4.6 在 Wave A 后与完整 diff 后各执行一次强制门槛；未通过则 UI 项退回 Gemini。
3. Codex 不直接修改页面 JSX/CSS，不把未过门实现冻结为候选。
4. 当前无后端扩展计划；如新出现后端或组件边界问题，由 Grok 给出 owner、最短合同与复杂度结论，再按用户路由处理。
5. `main`、旧 Windows 等待、旧证据收口、对外图文、push、PR、merge 与发布继续锁定。
