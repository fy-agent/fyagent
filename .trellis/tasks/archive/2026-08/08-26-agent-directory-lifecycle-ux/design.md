# Technical Design — AI 软件目录生命周期 UX

## 1. Design Intent

本设计不新增一个“Agent 管理框架”，而是把现有四层权威状态组合成一个渐进 UI：

1. **Catalog layer** — 决定有哪些软件、顺序、名称和描述。
2. **Readiness layer** — 决定某个软件是否已安装、是否可更新、哪些 lifecycle action 被允许。
3. **Action/job layer** — 决定安装/更新正在做什么以及最终是否结束。
4. **Configuration layer** — 只有在目录确认软件存在后，才允许进入现有模型/Skills/MCP/提示词配置 shell。

核心原则是单向投影，不做反向猜测：目录不从配置文件推断安装，不从按钮状态推断 action 成功，不从 display name 推断 installer。

## 2. Recommended Architecture (not a frozen file contract)

### 2.1 Directory projection

`AgentDirectory` 应从 catalog entries 直接渲染完整列表。scan controller 只提供每一行的 readiness overlay/state，不再决定“这一行是否存在”。

推荐把 scan controller 的页面消费模型从“installed entries list”调整为“per-agent observation”：

```text
catalog entry
  + no result yet            -> queued/scanning presentation
  + readiness installed      -> configurable presentation
  + readiness not_installed  -> install/update presentation as allowed
  + readiness unknown/error  -> blocked/error presentation
```

具体 TypeScript type、reducer action 名称和是否增加 selector helper 不固定，只要求 derived state 有一个清晰 owner，避免卡片组件散落条件分支。

### 2.2 First-entry progressive scan

保留 TanStack Query 作为 readiness cache/transport owner。现有 `useAgentInstallReadiness(agentId, false)` + `refetch()` 可继续使用；首次目录 mount/active 后由一个一次性 effect 调用现有 `start()` 或等价入口。

“渐进”指每个 promise settle 后立即进入 reducer/cache，而不是等 `Promise.all` 的最终完成才渲染。现有 reducer 已做到 per-item settle，因此优先扩展而非重写。

是否把 7 个读取完全并发、分批、或有限并发属于可替换细节。默认保持简单；只有 profiling 或 backend 证据证明并发造成问题时再加调度。

### 2.3 Lifecycle action adapter

目录需要一个小的 action view model，而不是直接知道两个 installer domain 的内部细节。

推荐组合：

- **Generic Agent path**：从现有 `AgentInstallReadinessSection` 抽取 start/poll/terminal/reread 流程，继续调用 `FeaturePorts.agentInstallReadiness`。
- **Codex path**：直接复用 `useCodexDesktopInstaller` / `FeaturePorts.codexDesktop`，只做目录行需要的紧凑投影。

页面可消费的 view model 至少应表达：当前可见 primary action（install/update/none）、是否 busy、真实 stage/progress、动作错误，以及 run/retry/cancel（如果当前 backend 真正支持）。字段名和组件拆分允许实现时调整。

不得创建 `startAgentInstall()` 之类绕过现有 FeaturePorts 的第二套 Tauri adapter。

### 2.4 Progress semantics

统一的是“用户知道现在在进行什么”，不是强制统一数据形态：

- Generic Agent snapshot：stage 是唯一事实，显示 stage + spinner/indeterminate feedback。
- Codex snapshot：沿用既有 percent / bytes / speed；目录可以只显示 percent + concise stage，但来源仍是现有 view model。
- Immediate CLI action：调用 pending 时显示处理中；返回后马上 authoritative reread。

终态永远触发权威 refresh。UI action success 与 readiness success 是两个不同事件。

### 2.5 Side navigation motion

当前 SelectionLens 的 `selectionLensTransition` 是项目已经调校过的 motion signature。推荐把这个 spring 作为 shared motion token/owner，让 collapsible 内容和 caret 在同源参数下运动。

语义层优先使用已安装的 Radix Collapsible，而不是手写 disclosure：它提供 controlled `open`, `onOpenChange`, `data-state` 和内容尺寸变量。由于 V2 禁止 import legacy UI wrapper，实现应在 V2 shared 层使用 package 或薄 adapter，而不是引用 `src/components/ui/collapsible.tsx`。

现行 V2 shell spec 把 `framer-motion` 限制在 SelectionLens 文件。为了本需求，可以选择以下任一小范围演进：

1. 把 motion token/primitive 抽到一个 V2 shared motion owner，并把静态规则收敛为“Motion 只允许在 reviewed shared motion owners”；或
2. 在不破坏模块职责的前提下让现有 shared owner 提供 collapsible motion adapter。

推荐 1，因为职责更清楚，但不把文件名写成长期产品契约。

不要用一个新的手调 `cubic-bezier(...)` 假装等价 spring；也不要新增动画依赖。

### 2.6 Accessibility / focus

Radix/trigger 继续承担 disclosure 的 `aria-expanded`/state。现有 SideNavigation 的自定义 ArrowRight/ArrowLeft/Escape/Home/End 语义保留。

关闭动画期间需要保证即将关闭的 leaf 不进入键盘导航计算；实现可依赖 Radix presence/focus semantics，或在 shared adapter 中提供明确的 closed/inert filtering。具体 DOM 结构可变，但测试必须冻结行为而非实现细节。

`prefers-reduced-motion` 直接切换最终布局，spring token 不执行 layout travel。

## 3. State and Action Matrix

| Readiness / job | Left lifecycle slot | Configure | Notes |
| --- | --- | --- | --- |
| first scan pending | `正在扫描…` | disabled | row is already visible |
| read failed / unknown / unavailable | blocked/error status | disabled | never convert to not-installed |
| not installed + `install` allowed | `一键安装` | disabled | click uses backend-authorized action |
| not installed + install not allowed | no fake install; honest unavailable state | disabled | optional reason copy |
| installed + update available + `update` allowed | `一键更新` | enabled | update is optional; config remains valid unless action is running |
| installed + up-to-date | none | enabled | no redundant install control |
| action busy | real stage / progress | normally disabled during the mutation | avoid configuring against changing install state |
| action terminal | refreshing/readback | gated by new authoritative result | no optimistic success |

`installed_not_runnable` 保留当前目录把它视为“存在”的兼容语义；实现若发现某个配置动作实际要求 runnable，应在相应配置 owner 中做更窄门禁，而不是把整个目录状态重新定义。

## 4. Serial Implementation Boundaries

本任务在一个开发分支中串行推进。阶段边界用于控制复杂度和验证范围，不是独立模块契约：

1. **Side navigation motion** — 先收敛 shared motion/collapsible owner 与 SideNavigation 行为，保证键盘、ARIA、reduced-motion 稳定。
2. **Progressive scan** — 再演进 `useAgentDirectoryScan`，建立 catalog-first、per-row settle、首次自动扫描和 retained result 语义。
3. **Lifecycle actions** — 抽取/复用 generic action runner，并为 Codex 建立最薄的目录投影；不改 backend contract。
4. **Directory composition** — 将前三阶段结果接入 `AgentDirectory` 和 `Page.css`，统一 configure gate、安装/更新按钮、busy/progress/error/readback。
5. **Integration / QA / SPEC** — 更新 unit/browser tests，完成 shell 联动、响应式、能力真实性与最终 spec 收口。

阶段之间允许根据最新代码做小幅合并或调整顺序，只要不改变 PRD 的用户可见结果和安全边界。不要为了阶段形式制造临时 API，也不要保留只服务于“未来 Integration”的包装层。

## 5. Compatibility / Rollback

- 不改 Agent catalog wire contract。
- 不改 install readiness/action wire contract，除非实现发现真实 backend bug；任何 contract 扩大必须回到规划评审。
- 不改 Codex installer wire contract。
- 目录变化是 renderer behavior；按串行阶段保留清晰 diff 和 focused validation，出现回归时优先回退最近阶段的接线，而不是破坏后端安全 contract。
- 现有 dirty V3 工作需要在开始实现前识别和保护；本任务继续在单一分支中工作，不创建额外 Worktree 来绕开这些改动。

## 6. Open-source / Reuse Decision

本轮无需新依赖：

- Radix Collapsible 已在仓库中，并提供标准 disclosure、controlled state、`data-state` 与内容尺寸变量。
- Motion/Framer Motion 已在仓库中，physics spring 原生支持 stiffness/damping/mass；现有 SelectionLens 已有满意参数。
- TanStack Query 已在仓库中，disabled/lazy query + manual refetch 足以支持现有 readiness transport。

因此“再找一个 collapse animation library / lazy loader / installer progress library”都属于重复造轮子。
