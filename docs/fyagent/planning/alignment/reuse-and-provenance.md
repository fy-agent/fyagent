# 9 张迭代票与 CC Switch v4.0.0：来源分层和具体复用路径

日期：2026-10-04。结论：**应以 v4 固定实现为主要复用材料，减少重新设计与重写；同时把“v4 继承已有能力”“FyAgent 已有能力”“FyAgent 自己增加的要求”标出来。** 不能把 9 张工作票都包装成 4.0 新增功能，也不能因为完整页面不能直接编译就退回从头设计。

本轮只读源码及规划，只新写本报告。未改代码、配置、Issue、Git ref；未启动应用、编译或运行测试。复用级别是代码审计判断，不是完成迁移或性能承诺。

## 固定来源与判断方法

- 上游工作副本：`research/upstream-source`，实际 HEAD `a189980f35a4568f8cf3585747895c748b8cec6a`。本审计一律使用 `git show/grep 412579f9ad81ed3c045476198aa9f8d1de36a2ae`，不把多出来的 main 文档提交混进版本判断。
- v4.0.0 固定源码：`412579f9ad81ed3c045476198aa9f8d1de36a2ae`；按既有官方来源核查为 prerelease。稳定比较点：v3.20.4 `43e1d99084ed9b2f5dc252fd35c5adaf29d6876e`。本地两个 commit 均可读；本轮没有新增本地 tag。
- FyAgent：`origin/main@5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798`。当前 checkout 较旧，不用它推断 main 功能。
- 用稳定点与 v4 的文件存在性/差异、实际函数、导入依赖，结合 `github-issues.json`、`plan.md`、已核主干审计判断。文件新出现只能证明新模块，不能单凭它宣称某业务第一次存在。
- 下文 `U:` 指固定 v4 仓库路径，`F:` 指上述固定 FyAgent main 路径；新增 FyAgent 落点会标“拟新增”。上游源码可按 `https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/<path>` 打开。

## 逐票分层：这些能力在 v4 中都可以存在，但不都是 v4 新增

| Issue | v4 新增 / 重做 | v4 继承 3.20 已有 | FyAgent 已等效 / 增强要求 | 非上游功能与票的正确定位 |
| --- | --- | --- | --- | --- |
| #205 稳定来源与兼容修复 | 此票主体不是 v4 新功能。部分旧路径在 v4 被 live/mode 重构替代，候选线使用 v4 最终实现，避免照搬已经退役的旧 writer。 | Codex 0.149/0.154、图片与工具输入、账号/workspace、重登录、WSL 写入、备份校验/同步锁等稳定段修复；v4 保含这些成果。 | 已有 SecretRef、Codex 字段 patch、备份/同步/协议转换，需按差值吸收并保留本地约束。 | 来源证明、two-parent、测试是交付流程。定位为“继承稳定能力并建立正确集成基础”，不是新产品模块。 |
| #206 配置所有权与候选恢复 | `live/{engine,patch,project}`、`mode/{state,operation,controller}`、首写备份、模式退出重投影等结构是 v4 重做。 | 原有 Provider、代理、账号、备份并非 v4 首创。 | Codex `source_switch` 已有无损 patch，ChangePlan 已有确认/幂等/回读。跨所有应用的恢复和旧新同步限制是下游组合要求，不能产生两套写入权威。 | 候选分支与跨版本验收是交付手段。复用上游 patch/project 和模式算法，明确接入 FyAgent ChangePlan 的边界。 |
| #207 Skills / MCP / Prompt | MCP 草稿多格式识别、批量导入与逐应用重试，Codex strict MCP 字段适配，资源页交互重做、批量更新、Prompt 复制/撤销/手写保护等。 | 安装、发现、分配、备份、Skill ID/缺文件重装与大仓库读取，Prompt 外部回采等基础早已有；auto/symlink/copy 也不是 FyAgent 的新能力。 | **通用全部目标快照成功再写、外部漂移使确认失效、完整逐目标回滚结果，是 FyAgent 增强。v4 通用 update_skill 仍存在旧风险，不能用页面迁入代替补齐。** | 此票应分“上游资源功能迁入”和“FyAgent 更新保证”两段，不把后者说成上游免费附送；也不另造资源平台。 |
| #208 客户端适配 | Apps 页集中升级及来源冲突呈现、非 npm 安装不补 npm、MiniMax 安装检测/升级，OpenCode JSONC/JSON5 格式保持等 v4 差值。 | MiniMax Code 的 Provider/MCP/Skills/Prompt/会话/用量、Pi 全链路在 3.20.4 源码已存在，v4 继承并修改。 | FyAgent 已有 AgentDirectory、受管安装、WorkBuddy/QoderWork/TRAE Work；mcode/Pi 在本轮 main 审计中尚无 AppType，宜直接适配上游模块。 | 平台支持表、真实安装与 ownership 验证是交付要求。不能把一个卡片或一个 enum 当完整适配。 |
| #209 原生模型聚合 | `mode/stack.rs`、模式 controller、前缀模型 ID、稳定 key、名单与目录投影是 v4 新路径。 | 当前 provider 路由、协议转换、failover、模型拉取是旧基础。 | FyAgent 已有 catalog/代理/凭据/ChangePlan，但没有等价通用聚合路由；应移植 stack 算法，接既有凭据/状态，不自行重新发明 ID 路由。 | 原生客户端选择→请求→回读是验收。不是“FyAgent 自研聚合平台”，也不是重命名 failover。 |
| #210 会话阅读与统计 | typed blocks、按轮结构、reader、块分页/图片、搜索命中展开，2.x parser 差值、热力图/范围查询/TPS 等。 | 会话扫描/消息读取、Pi 用量去重、字节游标、已有统计与定价来自旧版；不是所有 usage 改进都在 4.0 才出现。 | FyAgent 已有迁移包、native restore/receipt，应保留；采用上游阅读 DTO 和 parser，不把 native 恢复纳入重写。 | 定量性能与多客户端夹具是验收流程。重点可直接继承 reader/parser，而非自创 timeline 规范。 |
| #211 可用性 | `modelMetadataFill`、`quotaRules`、mode 页/托盘状态、新 auth/侧栏呈现等 v4 改进。 | `FetchedModelPicker.tsx` 在稳定点已存在；搜索/模型列表/预设/价格也有旧基础。 | 已有蓝色主题、FeaturePorts、Query、通知、软件目录与账户界面。复用上游具体表单/行/矩阵/规则，适配组件和接口；不是仅借“设计理念”后全部重画。 | 上游配色/品牌/商业推广不属于功能合同；不因此否定上游布局和交互代码的复用。 |
| #212 原生受限工具入口 | 未发现对应上游外部工具服务器或双目标宿主扩展面板可直接移植。 | 上游 MCP 管理是配置别的 server，不等于它自己暴露服务。 | 复用 #207 服务和 ChangePlan；桥接、确认合同、宿主调用是 FyAgent 探索。 | **不是 v4 对齐必需项。** 若本轮以少自研为目标，可作为独立可选探索，不阻塞迁移主体。 |
| #213 跨设备验收 | 没有单独的“v4 跨设备验收功能”。 | 上游已有测试、备份/同步及平台代码可作素材。 | 需验证 FyAgent schema26、SecretRef、native install、WorkBuddy 等组合后的实际行为。 | 这是交付验收，不是自研产品功能；可以复用上游夹具/失败场景，仍须本地平台证据。 |

直接存在性核验：稳定点已有 `mcode_config.rs`、`pi_config/mod.rs`、`session_usage_{pi,mcode}.rs`、`FetchedModelPicker.tsx`、`SkillsPage.tsx`；不存在新 `live/engine.rs`、`mode/stack.rs`、`jsonc_document.rs`、`session_manager/model.rs`、`modelMetadataFill.ts`、`mcpDraft.ts`、`quotaRules.ts`、`AppsPage.tsx`、reader/turns.ts。这与上述分层一致。

## 高价值复用路径：先继承代码，再做有证据的适配

级别含义：**接近原样**＝核心算法/模块及原测试优先保留，做导入/类型/必要身份适配，尚未编译证明；**服务适配**＝保持算法与测试但换本地状态、锁、凭据、写入边界；**交互结构复用**＝优先保留组件拆分、布局和交互流程，替换数据与视觉基础组件；**不要重复造**＝FyAgent 已有权威，新增能力接入它。

| # / 功能 | 上游具体文件 / 入口 | FyAgent 落点 | 级别、原因与本地约束 |
| --- | --- | --- | --- |
| #208 JSONC/JSON5 保真 | `U:src-tauri/src/jsonc_document.rs::JsoncDocument::parse`（21行）、round-trip merge；注释明示文件选择/锁/快照归调用者。 | `F:src-tauri/src/jsonc_document.rs` 拟新增，供已有 `opencode_config.rs` / `mcp/opencode.rs` 使用。 | **接近原样。** FyAgent 已有相同 `json-five=0.3.1`、`json5=0.4`；无需另写注释 parser。保留本地路径选择、锁及 rollback，不把 parser当事务。 |
| #206 格式 patch | `U:src-tauri/src/live/patch/{json,toml,dotenv}.rs` 的 JsonPatch/TomlPatch/DotenvPatch；`patch/mod.rs` KeyPath/LiveWriteError。 | 作为本地配置 writer 的纯补丁层候选；接 `services/provider/live.rs`、`codex_config/source_switch.rs`。 | **接近原样（纯 patch）+ 服务适配。** 两边已有 serde_json preserve_order / toml_edit 0.22。Codex 现有 source_switch 不能同时被另一 writer 管同字段。 |
| #206 应用字段投影 | `U:src-tauri/src/live/project/{claude,codex,gemini,grok}.rs`；Claude direct_patch:89；Codex CodexConfigPatch:525；`live/engine.rs` + `mode/operation.rs`。 | `F:services/provider/` 与 `services/change_plan/adapter.rs`；其他应用逐个扩展。 | **服务适配 / 不要重复造。** 复用字段集合、格式检查、首次快照算法和恢复用例；维持 FyAgent SecretRef、设备目录、ChangePlan确认与回读，不能并行放一个 `.cc-switch/live-state.json`。 |
| #209 聚合模型 ID | `U:src-tauri/src/mode/stack.rs` allocate_key:57、encode:103、decode:130、members:387、resolve:549。 | `F:proxy/provider_router.rs`、`proxy/handlers.rs`、catalog；拟新增聚合模块。 | **服务适配，算法尽量原样。** 直接沿用稳定key登记、未知保留前缀拒绝、无前缀默认provider、不混failover的规则。上游 `ccs-` 前缀包含客户端特定要求，品牌调整须显式兼容设计，不能全局字符串替换。凭据仍走ProviderCredentials/SecretRef。 |
| #209 目录发布与失效 | `U:services/provider/{codex_client_catalog,codex_official_models}.rs`、`mode/controller.rs::enter:543/exit:657/set_stack_member`。 | 既有 `F:codex_config/catalog.rs` 与 ChangePlan、模型模式入口。 | **服务适配。** 采用上游目录和 stale client 判断，不从零创建重载提示；本地 source ownership、官方登录和已有catalog格式必须保留。 |
| #210 typed 会话及延迟内容 | `U:session_manager/model.rs::SessionMessage:13/SessionBlock:126/ContentRef:336`；`content.rs::validate_source:78/paginate:181/resolve_content_ref:197/load_image:471`；`providers/{blocks,codex_items,opencode_blocks,pi_blocks}.rs`。 | 在 `F:session_manager/` 新增阅读 DTO/parser；现有 `commands/session_manager.rs` 与 feature port 接口扩展。 | **接近原样 DTO/解析算法 + 服务适配文件能力。** 不重新定义每个工具块。保留32MiB文本/20MiB图像/每页512Ki字符等上游限制并按FyAgent审查；native migrate 的去tool/reasoning提取合同不变。 |
| #210 按轮/搜索/折叠 | `U:components/sessions/reader/turns.ts::buildTurns:537/findSearchHits:635/flattenRows:950`、`toolSummary.ts`、`exportMarkdown.ts::turnToMarkdown:106/transcriptToMarkdown:139`。 | `F:pages/sessions/components/` 下 reader模块（拟新增），逐步替换 ConversationStream 阅读区。 | **接近原样核心算法 + 交互结构复用。** 连同 turns/toolSummary tests 与 fixtures迁入。exportMarkdown仍导入SessionEventRow函数，需抽出小型文本依赖或连带适配，不能误称完全零UI依赖。保留现有迁移/恢复菜单。 |
| #207 MCP 粘贴与掩码 | `U:components/mcp/mcpDraft.ts::recognizePaste:267/parseJsonText:323/validateDraft:371/specOf:102`。 | `F:shared/features/mcp.ts` 或相邻 mcp-draft模块（拟新增）；Page/InstallDialog复用。 | **接近原样。** 仅依赖smol-toml、类型与normalizeTomlText，FyAgent已有兼容McpServerSpec。保留未知extra、掩码不能写成真密钥、校验错误码。不要再写一套JSON/TOML识别器。 |
| #207 MCP 逐应用结果 | `U:services/mcp.rs::resync_app:302`、`resync_targets:279`、`U:mcp/codex.rs::json_server_to_toml_table:650`；`U:hooks/useMcp.ts` 批量顺序执行与onSettled刷新。 | `F:services/mcp.rs`、`mcp/codex.rs`、`shared/platform/tauri/feature-ports/simple.ts` / `shared/features/queries.ts`。 | **服务适配。** 复用转换和结果代码，但上游AppType目标模型须映射到FyAgent McpTargetId，保留WorkBuddy/QoderWork/TRAE Work及信任流程；查询key须统一，不导入第二套缓存。 |
| #207 Skill/Pi/Mcode写入 | `U:services/skill.rs::update_mcode_skill_files:1692`、目标检查/staging、目录定位/缺文件修复；`U:components/skills/repoFailures.ts`、`useSkillInstallTargets.ts`。 | `F:services/skill.rs` 和 assignment/repository；新Skill ChangePlan adapter。 | **服务适配 / 不要重复造。** 采用已写好的本地目录检查和Mcode保留旧副本到DB提交的做法；扩到通用目标需要明确补强，不能照搬 update_skill 的弱结果。 |
| #208 Mcode 与 Pi | `U:mcode_config.rs::data_dir/write_and_commit`；`mcp/mcode.rs`、`session_manager/providers/mcode.rs`、`services/session_usage_mcode.rs`；Pi相应模块。 | `F:app_config.rs`、Provider/target适配、session/usage各域拟新增对应模块。 | **服务适配。** 直接复用MINIMAX_DATA_DIR/MAVIS_DATA_DIR、native lock、custom provider写法和原测试；不可碰原生default模型/账号，连接FyAgent凭据与schema迁移，不复制上游迁移编号。 |
| #211 模型只补空值 | `U:components/providers/forms/modelMetadataFill.ts::fillCodexCatalogModel:27/fillOpenCodeModel:90`、`lib/modelMetadata.ts`、`hooks/useModelMetadataFill.ts`。 | `F:pages/models/` 与 `domain/configuration/` / model fetch ports。 | **接近原样补全规则；hook服务适配。** 上游已经编码用户已填值保留、reasoning档位过滤、模态处理；迁入modelMetadataFill tests。Pi字段仅随Pi支持接入，不能把跨网关同名能力当一致。 |
| #211 模型选择器 | `U:components/providers/forms/FetchedModelPicker.tsx`。 | `F:pages/models/` 模型添加区/现有模型表单。 | **交互结构复用，低耦合组件优先搬。** 只有models/configuredModelIds/onAdd等props和本地搜索/选择，不依赖全局store；换Button/Checkbox/Input/ScrollArea/t函数就可保持布局和行为。保留IME与Enter不误提交规则。 |
| #211 额度规则 | `U:components/quota/quotaRules.ts::tierLine:103/countdownStr:70/resetCreditsLine:175/cardRows:340`。 | `F:pages/auth/presentation.ts` 或相邻额度规则模块。 | **接近原样规则 + DTO/t函数适配。** 复用已写好的剩余量、过期、未知、合并两行规则；不重写第二套倒计时算法。reset仅展示，不自动兑换；FyAgent的账户身份与workspace来源保留。 |
| #205/#206 流式兼容 | v4新增 `U:proxy/providers/responses_late_arguments.rs::create_late_arguments_repair_stream:195`、`inline_think.rs::split_leading_think_block:96`；稳定协议修复在既有transform模块。 | `F:proxy/providers/`、response_processor。 | **接近原样局部算法 + 服务适配流生命周期。** 与上游单元用例一起迁入，确认FyAgent既有转换分支/压缩/headers/usage收尾；不能仅测HTTP200。 |

## 两个不能被“直接复用”掩盖的差异

### v4 Skills 仍不是整项冲突安全事务

固定 v4 `services/skill.rs:1618` 仍是 `let _ = Self::create_uninstall_backup(&skill)`；`:1676–1687` 对除Pi/Mcode以外已启用目标同步错误只warn，随后返回Ok。`:3931–3963` 的卸载备份仍选择一个source_path写一个skill目录。Pi/Mcode有专门保护，不能说所有目标都同样不安全，也不能说专项保护已覆盖Claude/Codex全部副本。

因此 #207 可直接拿上游更新检查、目录与目标安全、错误显示、安装分配、批量UI等代码；FyAgent额外承诺的全目标快照、用户确认digest、外部漂移拒绝、逐目标补偿仍需本地增量。缩小增量的办法是扩展现有ChangePlan并复用上游staging，不是创建新引擎，也不是删掉必要保证。

### v4 live engine 的外部漂移策略与已确认计划不同

`U:live/engine.rs:1–10` 明确写到：最终重读比对发现变化时，以新内容重算；也明确外部进程不持其锁，检查到rename仍有窗口。这适合其内部写入合同，但FyAgent在“用户确认P1，只应用P1”路径要求漂移使预览失效。引入底层engine必须把自动重算接到重新预览/确认，不能在确认后自行重算继续写。保持事实限制，不承诺跨外部软件绝对原子性。

## 页面能否直接搬：具体依赖和最少替换点

### 已兼容的基础

两边均使用React18、TypeScript、React Query5、Tauri2、部分Radix、smol-toml、zod。FyAgent Vite已有 `@` 指向src。Rust侧serde_json preserve_order、toml_edit0.22、rusqlite0.31、json5/json-five版本亦已匹配关键模块。**没有理由把上述纯规则和UI结构全部重写。**

### 实际缺口，不是品牌颜色一项

| 层 | 上游页面实际依赖 | FyAgent现状与建议替换 |
| --- | --- | --- |
| primitives / CSS | `@/components/ui/*`、Tailwind3、`bg-surface/text-fg-*/rounded-panel/shadow-v7-*`等token。 | `src/shared/ui/*` + 页面CSS，无Tailwind依赖。可搬组件布局并映射class/token；若选择保留受限上游样式，须隔离作用域。不能只复制TSX声称页面可跑。无需因CSS不同重新讨论用户流程。 |
| 图标与通知 | lucide-react、`@/lib/toast`（上游通知层）。 | @phosphor-icons/react、useFeatures().notify / ToastViewport。替换图标/通知调用，保持动作与反馈时机。不要再挂第二个全局toast。 |
| 翻译 | react-i18next/i18next；页面遍布t(key, defaultValue)。 | package无此两库，现有页面直接中文。可使用页面局部文本适配函数/复用中文词条，或明确决定引入i18n；不能默认把整套国际化作为迁移前置。quotaRules的TFunction是类型/注入参数，可适配。 |
| 状态/API | `lib/api/*`直接invoke、上游AppId/types；hooks的query keys与settingsApi。 | FeaturePorts / useFeatures / featureKeys。例如上游MCP是 `[mcp,all]`，本地需沿featureKeys；保持onSettled刷新避免DB已写但live失败时列表过期。按本地TargetId扩展DTO，保留只属于FyAgent的目标。 |
| 大列表/全文检索 | @tanstack/react-virtual（Skills/SessionReader）、flexsearch（session search）。 | 当前package无上述依赖。可明确引入所需小依赖并保留上游列表算法；或按现有分页实现适配，禁止“删虚拟化后仍称同等性能”。 |
| 编辑器/图表/拖拽 | CodeMirror套件、recharts、dnd-kit、react-hook-form、sonner等存在于上游package，FyAgent未装。 | 按实际被迁组件的依赖闭包引入或映射已有组件。不要为了一个ModelPicker整包引入未使用依赖，也不要把需要这些依赖的复杂页描述成零成本复制。 |
|测试运行器 | 上游Vitest2、当前测试目录/配置、cross-fetch测试层。 | FyAgentVitest4、config目录和native Fetch/MSW合同。迁入fixtures/断言与组件测试，调整运行入口；不回退宿主工具链或恢复cross-fetch。 |

### 逐页可执行的迁入方式

- **模型批选**：`FetchedModelPicker`优先作为真实组件迁入，保留props/搜索/选择逻辑，映射本地四个基础组件与文案。它是最接近“直接搬”的可见功能。
- **MCP分配页**：保留`UnifiedMcpPanel → AppMatrix → McpFormModal/mcpDraft`结构与逐目标状态/重试逻辑。重接`useMcp`至FeaturePorts，替换shell header/dialog/notify与TargetId列表。现有WorkBuddyTrustDialog不能丢；本地已有搜索、顺序批处理、mask helpers可按函数差值合并，不两个helper互相调用绕圈。
- **Skills页**：保留`SkillsPage/UnifiedSkillsPanel/RepoManagerPanel/repoFailures/useSkillInstallTargets`的来源切换、失败展开和安装流程。替换上游skills.sh查询为本地明确支持的仓库/SkillHub入口；本地有分页discoverPage，不能无意改成全量下载。更新按钮进入#207的受保护应用流程，不能直接连上游弱update命令。
- **会话阅读**：保留`SessionReader`及其reader子组件作为阅读区域；新增本地typed transcript/block/image端口。`U:lib/api/sessions.ts`使用Tauri Channel及`get_session_block_content/get_session_image/reveal_session_path/export_session_markdown`，必须在本地命令注册、DTO/权限/内容限额中成套适配。`lib/query/sessions.ts`可保留分页/缓存逻辑，改注入本地port。不要覆盖FyAgent Page里的迁移包、恢复尝试、人工确认菜单。
- **Usage页**：`UsagePage`本身很薄但依赖`useSettings`，`UsageDashboard`再依赖查询/图表/表格；适配refreshInterval/sessionAutoSync等设置到本地端口，迁表格分页/热力图而非复制第二份统计store。是否保留recharts属于明确依赖决策。
- **Apps页**：迁入版本/来源/升级冲突的列表行与状态逻辑，对接AgentDirectory和既有安装readiness/ownership；不要让上游`useToolManagement`另管一个安装流程。最终页面布局由UI审查另行对照，本审计不预设全壳换或不换。

## 来源集成和验收：复用模块不是私自挑提交

遵循 `F:.trellis/spec/backend/upstream-sync.md`：确认完整tag object/peeled commit、隔离工作区、恢复ref、upstream push禁用；批准的来源集成是显式two-parent merge，保留祖先关系。上表是**语义冲突时优先保留哪些实现，以及后续适配提交的职责**，不是授权私自cherry-pick一堆文件冒充tag同步。

保留FyAgent的名称/协议/数据目录、MIT来源署名与混合许可、schema26及其后续迁移authority、SecretRef、Windows安装/当前用户边界、WorkBuddy等已有行为。工具链升级、依赖清理、广泛目录现代化在来源合并之后独立提交；不借来源合并重写历史。

优先直接带入已存在上游用例：`tests/components/mcpDraft.test.ts`、`modelMetadataFill.test.ts`、`quotaRules.test.ts`、`sessions/reader/{turns,toolSummary}.test.ts`、`tests/fixtures/sessions/*`、`src-tauri/tests/{mcode_commands,prompt_live_sync}.rs`、`src-tauri/tests/golden/*`，再补FyAgent本地差值。本轮只核对它们存在及相应实现，未运行这些测试。

对9票最小的措辞调整方向：#205写“稳定能力继承”；#206/#209/#210明确“采用v4对应模块作为实现起点”；#207拆清上游迁入与本地增强；#208把mcode/Pi基础标为继承，把安装/格式差值标为v4；#211把可迁组件与必须替换的依赖列清；#212单列可选探索；#213保留为交付验收。这样既对齐v4，也不会把已经存在的代码再做一遍。
