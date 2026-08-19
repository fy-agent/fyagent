# 统一配置变更前端与 UX 详细设计

## 1. 决策摘要

采用“共享体验壳 + 领域 projection”模式：前端共享 Change Plan 展示、一次确认、job 进度、回读结果和异常恢复组件；Provider 与 WorkBuddy 分别把后端返回的脱敏领域数据渲染为内容区。前端不实现事务规则，也不把两类配置压成同一种资源模型。

不新增 XState、Redux 或全局 reducer。服务端事实由 TanStack Query 管理；表单草稿与当前弹层视图保留在入口组件本地；执行状态来自后端持久化 job snapshot 和 Tauri event。

## 2. 现状锚点

- WorkBuddy 页面当前把表单、远程模型获取、保存和覆盖确认集中在一个组件中：[WorkBuddyPage.tsx](/Users/serendipity/.codex/worktrees/c282/fyagent/src/components/workbuddy/WorkBuddyPage.tsx:118)。
- WorkBuddy 表单已把 `expectedRevision` 放入保存请求：[WorkBuddyPage.tsx](/Users/serendipity/.codex/worktrees/c282/fyagent/src/components/workbuddy/WorkBuddyPage.tsx:274)。
- 当前保存直接调用 `saveModels`，成功后刷新并 toast：[WorkBuddyPage.tsx](/Users/serendipity/.codex/worktrees/c282/fyagent/src/components/workbuddy/WorkBuddyPage.tsx:299)。
- 当前 `overwrite_confirmation_required` 会打开专用二次确认：[WorkBuddyPage.tsx](/Users/serendipity/.codex/worktrees/c282/fyagent/src/components/workbuddy/WorkBuddyPage.tsx:312)。新设计移除该用户路径；内部 capability 由后端 job 自行管理。
- WorkBuddy query cache 只保存脱敏 status 和 model IDs，这是应保留的边界：[workbuddy.ts](/Users/serendipity/.codex/worktrees/c282/fyagent/src/lib/query/workbuddy.ts:18)。
- Provider API 当前 add/update/switch 直接 invoke mutation：[providers.ts](/Users/serendipity/.codex/worktrees/c282/fyagent/src/lib/api/providers.ts:108)、[providers.ts](/Users/serendipity/.codex/worktrees/c282/fyagent/src/lib/api/providers.ts:140)、[providers.ts](/Users/serendipity/.codex/worktrees/c282/fyagent/src/lib/api/providers.ts:175)。
- 当前 Provider 返回值只有 `liveConfigChanged` 和 warnings，不足以表达 plan/job/readback：[providers.ts](/Users/serendipity/.codex/worktrees/c282/fyagent/src/lib/api/providers.ts:24)。
- Provider 用户动作集中在 hook 中，适合作为入口迁移点：[useProviderActions.ts](/Users/serendipity/.codex/worktrees/c282/fyagent/src/hooks/useProviderActions.ts:61)。
- Provider switch 当前直接发 mutation：[useProviderActions.ts](/Users/serendipity/.codex/worktrees/c282/fyagent/src/hooks/useProviderActions.ts:216)。
- WorkBuddy API 当前保存 DTO 仍暴露 renderer overwrite token：[workbuddy.ts](/Users/serendipity/.codex/worktrees/c282/fyagent/src/lib/api/workbuddy.ts:37)。迁移完成后该 token 不应进入 renderer。

## 3. 信息架构与组件树

保持现有 Provider Dialog 与 WorkBuddy Page 为编辑入口。提交编辑时打开统一 `ChangePlanFlow`，桌面优先用既有大尺寸 Dialog；窄屏使用同一组件的全屏布局，不另建业务页面。

```text
Provider Add/Edit Dialog or Provider Card action
WorkBuddy Page save action
└── ChangePlanFlow
    ├── ChangePlanHeader
    │   ├── domain icon + operation title
    │   └── plan freshness badge
    ├── ChangePlanBody
    │   ├── ChangePlanSummary
    │   ├── DomainPlanProjection
    │   │   ├── ProviderPlanDetails
    │   │   └── WorkBuddyPlanDetails
    │   ├── ChangePlanRiskList
    │   ├── ChangePlanRecoveryNote
    │   └── ChangePlanEvidenceNote
    ├── ApplyJobProgress
    │   ├── ApplyJobStepList
    │   └── CurrentStepAnnouncement
    ├── ApplyJobResult
    │   ├── ReadbackSummary
    │   ├── ResourceResultList
    │   ├── UsageEvidenceStatus
    │   └── RecoveryResult
    └── ChangePlanFooter
        ├── cancel/close
        ├── refresh plan
        └── confirm and apply
```

共享组件放在建议目录 `src/components/change-plan/`，领域 projection 可放在 `src/components/change-plan/domains/`。首版不要建立动态 plugin registry；以受控 union 的 `switch (plan.domain)` 渲染两个 projection，避免过早抽象。

## 4. 页面阶段与 UI 状态

前端只需要一个轻量 view phase，不复制后端 job 状态机：

```ts
type ChangePlanView =
  | { phase: "planning" }
  | { phase: "preview"; planId: string }
  | { phase: "applying"; jobId: string }
  | { phase: "result"; jobId: string };
```

后端事实独立表达：

- Plan：`ready | stale | expired`。
- Job：`planned | running | succeeded | warning | failed | cancelled`。
- Readback：`matched | mismatched | unavailable`。
- Recovery：`not_needed | succeeded | failed | not_attempted`。
- Usage evidence：`observed | not_observed | unsupported`；本组默认 `not_observed` 或 `unsupported`。

前端不得组合字段推导新业务状态。例如 `job=succeeded + readback=unavailable` 必须由后端给出最终 `warning`，renderer 不自行升级或降级。

### 状态到界面

| 状态 | 主标题 | 主操作 | 次操作 |
|---|---|---|---|
| planning | 正在生成变更预览 | 禁用 | 取消 |
| ready | 请确认这次配置变更 | 确认并应用 | 返回修改 |
| stale/expired | 配置已变化，需要重新预览 | 重新读取并预览 | 返回修改 |
| running | 正在应用配置 | 无 | 关闭但保持后台执行（若合同允许） |
| succeeded | 配置已应用并完成回读 | 完成 | 查看详情 |
| warning | 配置已应用，但有事项需留意 | 完成 | 查看详情 |
| failed + recovered | 应用失败，已恢复原配置 | 返回修改 | 查看详情 |
| partial | 部分变更未完成 | 查看处理建议 | 关闭 |
| recovery failed | 恢复未完成，需要处理 | 查看处理建议 | 复制脱敏诊断编号 |

`partial` 不作为前端自行维护的顶层枚举；由后端 `warning/failed` snapshot 携带逐资源结果，UI 根据后端提供的 presentation code 显示“部分变更未完成”。

## 5. 统一预览布局

预览首屏按用户决策顺序排列：

1. “你正在做什么”：目标产品、操作名称、影响范围。
2. “将发生的变化”：语义 diff，不显示原始配置文件 diff。
3. “需要留意”：风险、重启要求、计划时效。
4. “如果失败”：备份与恢复方式。
5. “证据边界”：应用成功不等于已观察到真实使用。
6. Footer：一次“确认并应用”。

不使用危险操作红色确认样式来表达普通配置保存。只有可能清除已有凭据、替换当前路由等实际风险使用 warning Alert；恢复失败才使用 destructive 语义。

## 6. Provider 内容表达

### 新建

- 标题：“添加 Codex Provider”。
- 变化：“保存 Provider ‘名称’”；若本次同时设为当前，则另列“将 Codex 当前 Provider 从 A 切换为 B”。
- 凭据：“API Key 将保存到本机安全后端”或“沿用现有 secretRef”；绝不显示值。
- live config：“将更新 Codex 本机配置”或“仅保存到 FyAgent，当前路由不变”。

### 编辑

- 按语义字段分组展示：身份、API 地址、模型配置、原生能力、凭据状态。
- 未变化字段默认不展开；提供“查看未变化项”而不是原始 JSON/TOML。
- Provider ID 变化或当前路由受影响时必须单独列出。

### 切换

- 标题：“切换 Codex Provider”。
- 核心句：“当前 Provider：A -> B”。
- 展示 live config 是否变化、是否建议重启 Codex。
- 重启是 apply 后的独立可信动作；不能把“已请求重启”写成“已生效”。

Provider warning 使用稳定 code 映射本地 i18n 文案。现有 WebSocket 警告 code 可继续进入计划风险区，而不是保存后 toast：[providers.ts](/Users/serendipity/.codex/worktrees/c282/fyagent/src/lib/api/providers.ts:34)。

## 7. WorkBuddy 内容表达

- 标题：“更新 WorkBuddy 模型配置”。
- 目标摘要：规范化后的服务地址仅显示 origin/安全可展示形式；不展示 URL 中可能存在的凭据或 query。
- 模型变化：新增 N、更新 N；默认展示前若干 model ID，并支持在当前弹层展开完整脱敏 ID 列表。
- 凭据：“为 N 个模型设置 API Key”“保留现有 API Key”或“清除 N 个既有 API Key”；不显示值。
- 文件状态：“基于刚刚读取的配置版本生成”；普通用户不需要看到 revision hash，诊断详情可显示截断标识。
- 恢复：“写入前创建本机备份；写后重新读取确认；不一致时自动恢复”。
- 远程“获取模型”仍留在编辑阶段，不能出现在执行步骤中。

现有 `WorkBuddyDuplicateConflictDialog` 不再承担第二次用户确认。重复目标应在 plan 中成为明确的“将更新已有模型”，后端内部处理 capability；若 baseline 改变则返回 stale，而不是询问是否强制覆盖。

## 8. 执行进度与事件

共享步骤使用面向用户的稳定阶段，不暴露内部函数名：

1. “正在确认配置未被其他程序修改” (`precheck`)
2. “正在保存安全备份” (`backup`，Provider 不适用时隐藏)
3. “正在应用配置” (`apply`)
4. “正在重新读取配置” (`readback`)
5. “正在恢复原配置” (`compensate`，仅失败后出现)

步骤状态为 `pending | running | succeeded | failed | skipped`，由 snapshot 给出。事件只用于降低延迟；收到 event 后更新对应 job query，窗口重载或丢事件时以 query snapshot 为准。

进度不是百分比。使用步骤列表、当前步骤 spinner 和已完成 check，避免虚假的线性进度条。运行超过阈值时显示“仍在本机处理，请勿关闭应用”，阈值属于展示策略，不改变 job 结果。

## 9. 回读、证据与恢复结果

结果卡分三层，不能合并：

- 应用结果：后端 job 最终状态。
- 本机回读：目标配置是否与计划匹配。
- 真实使用证据：是否存在独立、稳定的机器观测。

默认成功文案：“配置已应用，并已从本机配置重新读取确认。”

无使用证据文案：“尚未观察到这项配置被真实使用。这不影响本次本机配置应用结果。”

恢复结果：

- 已恢复：“应用没有完成，FyAgent 已恢复到确认前的本机配置。”
- 恢复失败：“应用没有完成，自动恢复也未完成。请保持应用开启，并按下方建议处理。”
- partial：逐资源列表显示成功/失败/已恢复，不用一个绿色 toast 掩盖部分失败。

诊断详情仅显示稳定错误 code、job ID 截断值、步骤名和脱敏摘要。完整路径、secret、原始配置及 backend 任意字符串不直接进入 UI。

## 10. Query / mutation / event 边界

建议前端合同：

```ts
type ChangePlanDomain = "codex_provider" | "workbuddy";

type ChangePlanRequest =
  | { domain: "codex_provider"; operation: "create" | "update" | "switch"; draft: CodexProviderPlanDraft }
  | { domain: "workbuddy"; operation: "update_models"; draft: WorkBuddyPlanDraft };

interface ChangePlanSummary {
  planId: string;
  planDigest: string;
  baselineDigest: string;
  domain: ChangePlanDomain;
  status: "ready" | "stale" | "expired";
  operationCode: string;
  projection: ProviderPlanProjection | WorkBuddyPlanProjection;
  risks: Array<{ code: string; severity: "info" | "warning" }>;
  recovery: { available: boolean; messageCode: string };
}
```

边界规则：

- `useCreateChangePlanMutation`：提交临时 draft，返回脱敏 plan；成功后不 invalidation 业务缓存，因为 plan 无副作用。
- `useApplyChangePlanMutation`：只提交 `planId + planDigest`，不再次提交表单 draft 或 secret；返回 `jobId`。
- `useApplyJobQuery(jobId)`：权威 snapshot，运行中按适度间隔轮询作为 event fallback。
- `useApplyJobEvents(jobId)`：订阅 Tauri event；只接受匹配 job ID 的序列化 snapshot/version，不在 hook 内翻译业务状态。
- job terminal 后按 snapshot 的 `affectedQueryKeys` 或受控 domain mapping 精准 invalidate；不要由通用后端返回任意 query key 字符串。
- renderer 不持久化 plan body、secret 或 capability；计划刷新后以新 `planId` 替换旧引用。
- 关闭弹层后若 job 仍运行，在应用级轻量 job indicator 提供返回入口；该 indicator 只存 job ID，不复制 job 状态。

首版可在 `src/lib/api/change-plan.ts`、`src/lib/query/change-plan.ts` 建立薄 API/query 层。不要把现有 Provider 与 WorkBuddy 所有查询迁入统一 namespace。

## 11. 与现有入口的集成

- Provider Add/Edit Dialog：现有校验通过后不再调用 add/update，改为生成 plan；返回修改时保留本地 draft。
- Provider Card switch：点击后直接生成小型 switch plan，仍需显示统一预览和一次确认。
- WorkBuddy Page：`handleSave` 改为 generate plan；移除 renderer 中 `PendingOverwriteSave` 与专用冲突 dialog。
- `fetchModels` 保持现有独立 imperative action，不复用 apply mutation。
- 老 hosts 不支持 Change Plan 时不得静默回退直接写；显示“当前后端版本不支持安全应用流程”，避免绕过确认。

## 12. 焦点与无障碍

- 打开预览后焦点落在标题；关闭或返回修改后恢复到触发按钮。
- Dialog 使用既有 focus trap；执行中不因步骤 event 抢焦点。
- 当前步骤变化通过独立 `aria-live="polite"` 区域播报；最终失败/恢复失败用 `role="alert"`，同一内容不重复播报。
- 步骤列表使用有序列表，图标不是唯一状态信号，附“等待/进行中/完成/失败”文本。
- 风险项与字段变化不用仅靠颜色区分。
- “确认并应用”和“返回修改”标签保持明确，不使用泛化“确定/取消”。
- 运行中禁用重复提交；禁用原因可被辅助技术读取。
- 减少动态效果偏好下关闭步骤切换动画；普通模式也只使用既有短淡入，不新增持续脉冲。
- 窄屏 footer 固定在底部但不遮挡内容；详情列表允许换行，model ID 与诊断 code 支持安全断行。

## 13. 文案字典

| 场景 | 推荐文案 | 禁止文案 |
|---|---|---|
| 入口 | 预览变更 | 保存并验证 |
| 确认 | 确认并应用 | 强制覆盖、我已验证 |
| stale | 配置已被其他程序修改，请重新读取并预览 | 是否仍然覆盖？ |
| apply 成功 | 配置已应用并完成本机回读 | 配置可用、验证成功 |
| 无证据 | 尚未观察到真实使用 | 未验证所以失败 |
| warning | 配置已应用，但有事项需留意 | 大概成功 |
| recovered | 应用失败，已恢复原配置 | 操作失败（无恢复说明） |
| recovery failed | 自动恢复未完成，需要处理 | 未知错误 |

所有 backend code 映射到 i18n key；未知 code 使用安全通用文案并展示脱敏诊断编号，不透传错误 message。

## 14. 兼容、渐进落地与回滚

- 第一阶段只接 Codex create/update/switch，其他 AppType 保持原路径，但 UI 明确首版范围。
- 第二阶段接 WorkBuddy，移除二次确认仅在后端 plan/apply/readback 合同完整后进行。
- 共用组件可以 feature gate 按 domain 启用；关闭 gate 应回到上一个已知稳定入口，但不得在用户无感知时绕过统一确认直接写。
- 每个领域保留原入口表单，回滚共享展示层不要求回滚表单结构。
- 不用 `liveConfigChanged` 兼容模拟 readback；缺少新合同即视为不支持。

## 15. 需要跨任务确认的未决点

1. 后端 job 是否允许应用关闭后继续；这决定运行中关闭按钮是“后台继续”还是必须阻止关闭。
2. `affectedQueryKeys` 由前端固定 domain mapping 还是后端返回受控 resource codes；推荐 resource codes，前端映射 query keys。
3. Provider create 是否允许“保存但不设为当前”和“保存并设为当前”在一个 plan 内；推荐允许，但必须呈现为两个资源步骤和可能 partial。
4. 恢复失败的产品支持入口尚未定义；首版至少提供脱敏诊断编号与本机排障文档入口，不提供复制原始配置。

