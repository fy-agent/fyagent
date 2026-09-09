# 历史计划归档说明

本任务按“被替代”结案，当前有效范围以父任务的 2026-09-08 修订为准。

Claude Code 和 Codex CLI 的订阅来源绑定由当前 Managed Auth、内置代理
和已有保存/Change Plan 流程承接。原计划要求通过旧界面验收 Claude
Desktop；该旧界面已经退役，当前修订没有宣称完成这一入口或四目标验收。

本计划来自 `feat/grok-first-class-iteration` 的
`b8b15dbaf141f7c7fbd7816914fda59a07a2208a`（PR #172），后续有效实现
和测试证据见父任务及 PR #185。当前合同由 frontend/grok-subscription、frontend/models、
backend/managed-auth、backend/proxy-runtime 和 backend/change-plan-executor
等已有 SPEC 承载，不恢复历史执行方案中的 V1/V2 路径。

保留原始 PRD 和未勾选的旧验收项。真实订阅、真实 CLI 及 Windows/Mac
双机真人验收不由模拟测试或这次行政归档替代。元数据明确标记
`resolution: superseded` 和 `historical_acceptance_completed: false`。
