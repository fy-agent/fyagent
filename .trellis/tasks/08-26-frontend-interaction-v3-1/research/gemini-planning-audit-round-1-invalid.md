# Gemini 3.7 Planning Audit — Round 1 Invalid

## Route

- Antigravity model: `gemini-3.7-flash-high`
- Conversation: `a2804f6e-bdc1-4427-bc62-a8ac1f0a0150`
- Mode: plan / read-only
- Product-code writes: none

## Verdict

`REJECTED_INVALID_CONTEXT`

该输出没有进入 PRD、Design、Implement 或页面差异表。

## Reason

模型将 FyAgent 的 11 张原型误认成另一套 Dashboard 页面，并引用了当前仓库中不存在的路径，例如：

- `src/v2/views/Overview.tsx`
- `src/v2/views/Tasks.tsx`
- `src/v2/views/Sessions.tsx`
- `src/v2/components/MetricCard.tsx`
- `src/v2/stores/modelStore.ts`

现场检查结果：`src/v2/views` 与 `src/v2/components` 目录均不存在。当前页面 owner 位于 `src/v2/pages/**` 与 `src/v2/shared/**`。

## Recovery

1. 放弃本轮全部页面判断。
2. 新建短会话，拆为 Pages 01-06 与 Pages 07-11。
3. 每个引用路径必须先通过仓库存在性检查。
4. 只允许使用明确列出的 PNG 与页面 owner。
5. 图片不可读时输出 `IMAGE_UNREADABLE`，禁止推断另一套产品。
