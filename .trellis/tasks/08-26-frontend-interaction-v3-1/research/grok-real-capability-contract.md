# Grok 4.6 Page 03 / 06 真实能力合同

## 执行记录

- Route：Grok Build fallback
- Model：`grok-4.6`
- Session：`01a03ad3-cb0f-7851-94e1-cdb984b83acf`
- Mode：read-only planning audit
- Verdict：`LIMIT_TO_EXISTING`

## Page 03｜Agent 模型段

Owner：`src/v2/pages/agents/AgentModelsSection.tsx` 负责只读投影；模型写入继续由模型管理页及其原有 owner 负责。

允许：

- WorkBuddy 读取 `useWorkBuddyStatus` 与 `useWorkBuddyModelIds`。
- OpenCode 读取 `useOpenCodeModelSnapshot`。
- TRAE Work 读取 `useTraeWorkModelIds`，保持 assisted 能力语义。
- Qoderwork 保持 unsupported，不发模型 query。
- Provider 类目标只投影当前或已配置 Provider 摘要。
- 提供搜索、选中详情和 `进入模型管理` 路由。

禁止：

- Agent 页模型列表不得渲染可写 Switch，不调用模型 mutation。
- WorkBuddy/OpenCode 的 `saveModels` 不得降格为缺凭证、revision、overwrite 或 concurrent 处理的单项开关。
- TRAE Work 的读取、校验与连通测试不得映射成保存成功。
- Qoderwork 不得出现可配置模型或成功状态。
- 不得伪造多模型同时启用、硬编码模型行或假数据。

回归合同：

- Qoderwork：unsupported、无 Switch、无模型 query。
- TRAE Work：assisted、显示已观测模型、无 Switch。
- WorkBuddy、OpenCode 与 Provider 投影：无 `role=switch`，零 mutation。

## Page 06｜Agent 提示词段

Owner：`src/v2/pages/agents/AgentPromptsSection.tsx` 负责 Agent 级投影与启用；Page 10 `PromptsPage` 继续持有 CRUD、导入、live file、dirty guard 与 write lock。

允许：

- 仅对有 `promptAppId` 的 Agent 调用 `usePrompts(appId)`，提供列表、搜索与只读正文。
- 使用 `ports.prompts.enable(appId, id)`，随后 `query.refetch()`；目标项回读为 `enabled === true` 后才显示成功。
- 提供 `进入提示词管理` 路由到 `/prompts`。
- `promptAppId=null` 的 Agent 显示未接入 owner，不调用 `prompts.getAll`。

禁止：

- Agent 页不得复制导入、新建、编辑、保存、删除、停用、live file 或脏稿拦截。
- 不得创建第二套 draft/writeLock，不跨层依赖 Page 10 内部 editor。
- qoderwork、trae-work、workbuddy 不得生成本地提示词库。
- 缺少 disable 专用 port 时不得用 Switch 模拟停用。

回归合同：

- qoderwork 提示词段不调用 `prompts.getAll`。
- codex 调用 `enable("codex", "review")` 后必须 refetch 并从真实配置确认 enabled。
- 回读失败时清理乐观成功状态。
- 导入、新建、保存与删除测试继续归属 Page 10。

## 实施边界

1. Page 03 与 Page 06 接受上述已冻结的真实能力边界。
2. 原型中的不可闭环交互不进入产品 DOM，并记录为工程边界差异。
3. 不新增 API、后端协议、依赖或静态假数据。
4. Gemini Wave A 违反任一合同即停止并退回 Gemini 修订。
