# Frontend Interaction V3 End-to-End Implementation

## 目标

在分支 `codex/frontend-interaction-v3-20260825` 上完成 FyAgent 前端交互重构 v3：保留现有蓝色 liquid-glass 主题与真实业务能力，按照已批准的 11 张高保真原型调整导航、按钮位置、入口层级和状态反馈；完成本地检查、构建、桌面包 UAT，并把 Windows 原生验证作为独立证据层闭环。不得触碰、合并或发布 `main`。

## 事实基线与优先级

出现冲突时按以下顺序裁决：

1. 飞书产品讨论与人类明确确认；
2. 已批准的 11 张高保真原型中的信息架构和交互位置；
3. 当前 V2 真实业务契约与平台能力；
4. 当前代码布局；
5. 实现推断。

高保真图不能授权伪造平台能力。若视觉稿展示了当前后端不支持的写入动作，运行态必须使用禁用态、只读态或明确说明，而不能制造“保存成功”。

## 成功标准

- 用户能在稳定的左侧导航中分清 `AI软件配置`、可展开的 `配置管理` 与 `记忆模块`。
- 用户能扫描 AI 软件、查看扫描中与扫描完成状态，并进入单个 Agent 的四段选配流程。
- 单 Agent 内的 `模型 / Skills / MCP / 提示词` 只展示来自相应管理页与真实配置源的资源，并提供搜索、选择或启停、返回和进入全局管理的路径。
- 现有模型、Skills、MCP、提示词和记忆能力在重装壳后不回退。
- 不支持、只读或 assisted-only 的 Agent 能力被如实表达。
- 代码、运行态、打包态和 Windows 原生证据分层记录，不互相替代。

## 需求

### R1｜分支与交付边界

- 所有修改只发生在指定 worktree 与分支。
- 本任务允许创建本地提交，但不包含 push、PR、合并、tag、Release 或生产发布；这些动作需要单独授权。
- 保留用户主工作树与无关脏改动。

### R2｜V3 导航壳层

- 将当前顶部六个等权主导航重排为左侧三组：
  - `AI软件配置` -> `/agents`
  - `配置管理`（可展开）-> `/models`、`/skills`、`/mcp`、`/prompts`
  - `记忆模块` -> `/memory`
- 保留现有六个 hash route 与 `PersistentPrimaryOutlet` keep-alive 行为，除非实现审计证明必须变更。
- 顶栏继续承载品牌和工具动作，不重复主导航。
- active、expanded、hover、focus 与 keyboard 状态必须清楚。

### R3｜AI 软件扫描

- `/agents` 支持扫描开始、扫描中、扫描完成、无结果与错误状态。
- 扫描复用现有 Agent Install Readiness 查询，进度按七个查询的 settled 数量聚合；不得新增后台扫描协议或事件总线。
- 扫描中有进度或阶段反馈，互斥动作禁用。当前没有真实 cancellation contract，本轮只提供等待，不实现“假取消”。
- 首次运行显示“开始扫描”；已有成功结果时显示“重新扫描”。`unknown` 必须单独呈现，不得写成“未安装”。
- 完成态显示软件图标、名称、两行描述、完整描述访问方式、发现状态、上次扫描时间和“进行配置”。
- 重新扫描不得破坏当前已保存配置。

### R4｜单 Agent 四段选配

- 进入 Agent 后固定显示 `模型 / Skills / MCP / 提示词` 四段页签、Agent 身份、返回列表与进入对应管理页的动作。
- Skills/MCP 使用现有 per-Agent assignment owner；筛选、开关和回读必须一致。
- 模型 Tab 是 capability-aware projection：显示已观测/已配置模型与管理入口；只有已有原生 owner 的直接路径才允许委托写入，不新增通用 model assignment port。只读、assisted 或 unsupported 目标显示真实限制。
- 提示词必须复用现有 `PromptAppId` 与 prompt port；无 prompt owner 的 Agent 显示不支持或尚未接入。
- 不新增与供应商配置脱节的第二套 FyAgent-only 假状态。

### R5｜全局管理页

- `/models`、`/skills`、`/mcp`、`/prompts` 保留现有业务能力，以原型定义的层级和按钮位置重新组织。
- Skills/MCP 保留已安装、发现、导入/安装、详情和应用分配。
- 提示词保留应用筛选、库、编辑、导入/新建、启用、保存和删除。
- 模型管理保留连接设置、密钥掩码、连通测试、模型读取与受支持的保存/应用路径。
- 已有失败反馈、loading、empty、disabled 与 destructive confirmation 不得丢失。

### R6｜记忆模块

- `/memory` 保留长期记忆、每日记忆、Agent/文件选择、读取、复制、保存和打开工作区等现有能力。
- 原型要求的“复制”是复制当前记忆内容；现有仅复制路径的行为不能替代。
- 本轮只统一壳层与视觉层级，不改变记忆数据所有权或引入新存储格式。

### R7｜视觉与可访问性

- 锁定现有 Blue Ambient / Clear Glass 视觉语言与色板 `#324D69 / #567495 / #7B99B8 / #9DDCFF / #F6FBFF`。
- 本轮不重做品牌色、字体体系或营销式视觉。
- 复用既有 shared UI；新增样式优先使用 token，避免复制页面级 magic values。
- 维护视口最小宽度 900px；窄屏可压缩侧栏，但不能隐藏关键文字或造成主动作不可达。
- 所有交互具备可见 focus、语义化 label、键盘路径，并遵守 reduced-motion。

### R8｜协作与模型配置

- Codex 是唯一最终负责人，负责计划、线程拆分、证据核验、冲突裁决、整合和缺陷闭环。
- 所有 Codex 新任务固定使用 `gpt-5.6-sol / max`。
- Gemini 负责视觉与交互实现/审查；Grok 负责后端、组件与研究，并主动挑战不必要的抽象和测试；两者不能自行宣称最终验收。
- Grok/Gemini 的具体可用模型与 reasoning 必须在 dispatch 前由人类批准；不得静默替换不存在的模型。

### R9｜质量与证据

- 环境预检先执行；独立 worktree 需要先完成依赖 bootstrap。
- 至少运行 V2 lint、typecheck、unit、browser interaction、renderer build 与仓库总检查；若触碰 Rust/backend，再运行 Rust 全套检查。
- 本地打包应用必须覆盖 11 个代表页面/状态和关键返回、展开、搜索、选择、启停、保存/失败路径。
- Windows 原生验证必须在 Windows 运行环境内产生 fresh receipt、截图/日志和失败路径证据；macOS、浏览器或传包成功不能替代。
- 每次飞书里程碑汇报必须有 message ID 与 fresh readback；图片以真实消息附件发送，不以本地路径代替。

## 非目标

- 完全重做主题、品牌或设计系统。
- 把 V1 页面或组件导入 V2。
- 为不存在的供应商 API 发明写入能力。
- 以 browser mock 证明真实平台配置已写入。
- 默认执行严格 1:1 pixel diff；只有人类明确要求阈值后才作为额外门禁。
- 签名、公证、公开安装包、push、PR、合并或发布。

## 验收清单

- [ ] 11 张批准原型对应的页面或关键状态全部可达，布局与动作位置经运行态截图核验。
- [ ] 左侧三组导航、配置管理展开、六路由 keep-alive、返回路径和 active 状态通过交互测试。
- [ ] 扫描的 loading/success/empty/error/unknown/disabled 状态有测试和运行证据；不存在无后端语义的取消成功态。
- [ ] Agent 四段选配具备搜索、选择/开关、进入管理和 fresh readback；不支持能力无假成功。
- [ ] Models/Skills/MCP/Prompts/Memory 原有核心能力与失败反馈没有回退。
- [ ] `mise run lint:v2`、`typecheck:v2`、`test:v2`、`test:v2:browser`、`build:renderer` 与 `mise run check` fresh pass。
- [ ] 本地桌面包可启动，11 个代表状态完成 UAT，缺陷清零或明确列为阻塞。
- [ ] Windows 原生 fresh validation 完成并与本地证据分开记录。
- [ ] 飞书群 `Fuck you Agent` 的阶段与最终汇报均有 message ID、readback 和图片/文档证据。
- [ ] `main`、push、merge、release、production 均未发生。

## 当前待批准决策

本地/官方事实中不存在可用的 Grok 4.7 路由；当前最新可验证路径为 Grok 4.6。Gemini 3.7 Flash 已官方可用，但本机 OpenCode whitelist 尚未加入。建议 dispatch 配置为：

- Codex：`gpt-5.6-sol / max`（已批准）。
- Grok：`vibekey/grok-4.6 / high`（当前本地最高可用 reasoning）。
- Gemini：`antigravity/gemini-3.7-flash-high`，已于 2026-08-26 完成真实 probe。

用户要求的 Grok 4.7/max 在官方目录、本机 provider registry 和真实 dispatch 中均不可用。为避免项目停摆，执行侧透明使用当前最新可用的 `vibekey/grok-4.6 / high`；任务证据持续保留 `grok-4.7/max = UNAVAILABLE`，不得把替代写成已满足字面要求。
