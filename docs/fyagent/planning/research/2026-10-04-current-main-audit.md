# FyAgent 当前主干能力与迁移落点审计

审计日期：2026-10-04。证据级别：`code_audit`。本轮未切换分支、合并、改动源文件、启动正式或开发版、连接外部 MCP、更新任何 Skill，也未运行测试；下文的测试项表示已读到的代码覆盖，不表示本轮通过。

规划基线固定为 `origin/main@5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798`。工作区当前仍为用户分支 `dev/laiyongjie@ada2dc988a664713c160ea8cdcf9f66b69d152f6`，不能用工作区文件行号替代主干事实。以下路径及行号均来自 `git show origin/main:<path>` 或 `git grep origin/main`；审阅时应打开固定 SHA 的内容。工作区的未跟踪文件未触碰。

## 最影响本轮取舍的结论

1. **已有 ChangePlan v2、执行 job、持久化事件和共享原生 UI，不能新造第二套变更引擎。** 但其当前操作只有 Codex 供应商切换、Codex 保存并切换、WorkBuddy 模型保存，Skills 未接入。
2. **9 月 30 日发现的 Skills 更新风险仍成立。** 更新前备份失败被忽略；主库替换后逐目标同步失败只写日志，函数最终返回成功；备份仅选一个来源，主库优先，不是每个复制目标各一份快照。
3. **Codex 配置关键字段 patch 与 profile 保留已有实际实现。** 迁移上游相似能力应做差异与回归用例吸收，不应重新创建配置写入层。
4. **现有模型目录/代理不等于多供应商聚合。** 当前目录服务读取 live 配置指向的 FyAgent catalog，路由器按应用选当前供应商或 failover 队列；未发现按带供应商前缀的模型 ID 选择多个供应商的通用入口。
5. **会话只读展示与会话迁移/恢复已有不同模块。** 当前展示 DTO 仍为 role/content/ts，工具调用部分被压成文本；结构化只读 timeline 可在 parser/DTO/展示上增量实现，不应重写 native restore。主 Agent 提供的 PR #198 状态为未合并，本审计未把它算入 main 已实现。
6. **MiniMax Code 的 `mcode` 应作为新增应用适配评估。** main 的 AppType 无此变体，src、src-tauri/src、.trellis/spec 对 `mcode` / `MiniMax Code` 搜索无命中。现有 MiniMax 模型/厂商素材不能等价于支持此客户端。

## 已吸收上游基线

`docs/upstream/cc-switch-v3.19.2.md` 记载 CC Switch v3.19.2 的来源：上游 `farion1231/cc-switch`，tag peeled commit `43eaf07355af145aebfee301801779e824d4c221`，FyAgent 两父合并 `f4462765e9b3a2efd1deb13aabf3ce349166a058`，整合日期 2026-08-08。此次用 `git merge-base --is-ancestor 43eaf073… origin/main` 得到退出码 0，确认该基线仍在当前主干祖先中。

账本强调保留 FyAgent 身份、`~/.fyagent`、数据库、Windows 正式安装、安全边界、WorkBuddy、原生壳与许可证区别。账本写的 schema 16 是当时整合边界，不是今天数据库版本；当前 ChangePlan 迁移已另有 schema v20 相关代码。不能直接覆盖上游目录再修品牌字段。

## 当前能力与可复用模块

| 领域 | 当前主干事实与证据 | 迁移规划含义 |
| --- | --- | --- |
| Providers / 模型 | `src-tauri/src/commands/provider.rs:514` get_providers；`:622/769/813` add/update/delete；`:925` switch_provider_with_result；`:841/857` analyze/patch_codex_provider_features；`:1737–1781` universal provider CRUD/sync；`commands/model_fetch.rs:95` fetch_models_for_config；`services/model_fetch.rs:54/139/238` 拉取、候选 URL、可选认证。 | 供应商管理、当前选择、模型探测、配置投影是已有资产；避免新造模型管理后台。聚合目录可复用抓取与 ProviderCredentials，但需新的目录命名/路由契约。 |
| 应用边界 | `src-tauri/src/app_config.rs:627–640` AppType 为 Claude、ClaudeDesktop、Codex、Gemini、GrokBuild、OpenCode、OpenClaw、Hermes；`:210` SkillTargetId 另含 QoderWork、TraeWork、WorkBuddy；`:253` requires_copy 对后三者为真；McpTargetId 也单独建模（`:30` 起）。 | mcode 若立项，应分别声明 Provider、Skill、MCP、session 哪些域支持，不能增加一个枚举就宣称全域支持；保留直达目标与 AppType 的分离。 |
| Codex 配置 | `src-tauri/src/codex_config/source_switch.rs:73` patch_source、`:162` project_source；只修改 owned 顶层字段、选中 provider 表，保留无关表；`:110` 缺失模型专属字段不视为重置许可；`:125` 只移除 FyAgent 自有 catalog 引用；`:182` 明确启用的公共 snippet 叠加；`codex_config.rs:756` Quick Setup 独立字段 patch。 | profile/MCP/权限/注释保护已经有实现；应映射上游新回归，比较覆盖缺口，不重复造“安全 patch”。仍需实测当前客户端加载结果。 |
| Claude / Gemini 配置 | `services/provider/live.rs:802` write_quick_setup_claude_live 在现有 JSON env 内更新指定键；`:1218` LiveSnapshot 区分 Claude settings、Codex auth/config、Gemini env/config；`:1540` read_live_settings；Gemini 认证辅助位于 `services/provider/gemini_auth.rs`。 | 沿现有配置 writer 与备份/恢复边界吸收兼容修复。不能把 Codex 的无损 TOML patch 覆盖结论外推给所有应用和所有保存路径。 |
| 代理 / failover | `commands/proxy.rs:13/40/54/63` 启动、停止并恢复、恢复预览、逐应用接管；`:334/351/435/446` 健康与熔断；`commands/failover.rs:13/37/50/79` 队列与开关；`proxy/provider_router.rs:37` select_providers；`services/proxy/restore_preview.rs:22` 恢复预览。 | 保留当前接管/恢复/凭据/熔断体系；聚合路由是新选择语义，不能直接把 failover 改名。 |
| Skills 安装 / 分配 | `commands/skill.rs:53/90/112/137/172/185` 安装、分配、导入、发现分页、更新检查与更新；`services/skill.rs:498` target 路径解析；`services/skill/assignment.rs:14` toggle_target 先同步/移除再更新 DB；`skill.rs:1769` sync_to_app_dir 分 Auto/Symlink/Copy；`src/pages/skills/Page.tsx` 已有列表/详情/分配。 | 可复用路径、目标身份、归档检查、发现、安装记录、现有页面；新能力聚焦可解释的检查/预览/逐目标写入结果。 |
| Skills 备份 / 恢复 | `commands/skill.rs:37/42/78` 列表、删除、恢复；`services/skill.rs:1373/1435` list_backups / restore_from_backup_for_target；`:3286` create_uninstall_backup。 | 现存备份是单 Skill 单来源恢复包，不是多目标事务快照。不能直接承诺保留所有副本个人修改。 |
| MCP | `commands/mcp.rs:161/169/178/184/196` CRUD、目标开关、从各应用导入；`services/mcp.rs` 与 `src-tauri/src/mcp/{claude,codex,gemini,grokbuild,hermes,opencode,qoderwork,traework,workbuddy}.rs` 已有目标适配；`src/shared/features/mcp.ts` 定义配置端口。 | 这是把外部 MCP 配置分配给目标软件的管理链。此次审计未发现可据以宣称“FyAgent 已向 Codex 暴露受限 Skills 工具服务器”的证据。若做连接层，先复用服务，另立受限传输适配。 |
| 会话 | `session_manager/mod.rs:59` 扫描七类来源，`:97` 读取消息；`commands/session_manager.rs:62/89/106` 终端恢复命令、删除；`commands/session_migration.rs:53/63/98/107/124/136/161` 迁移预览、导出、restore、回读、reconcile、人工确认、打开；`session_manager/migrate/*` 是独立模块。 | 只读 timeline 与写回迁移不同。保留现有恢复收据与 native writer，不把新增工具调用卡片变成新恢复系统。 |
| Usage | `commands/usage.rs:11/47/66/85/104/115` 汇总、趋势、供应商/模型统计、请求日志与详情；`:175/200/208/215` 价格与 models.dev 同步设置；`:251/277/296` 日志同步、Codex 重建、数据来源；`services/session_usage.rs:71` sync_all_unlocked。 | 保留统计底座及来源区别。增加聚合路由时需验 provider/model 归因，不能用 UI 汇总掩盖重复计数。 |
| WebDAV | `services/webdav_sync.rs:32/37` 互斥；`:64` 上传 db.sql + skills.zip、最后上传 manifest；`:110` 下载、兼容与哈希校验、apply_snapshot；`:164` 远端信息；`commands/webdav_sync.rs:85/105/116/142/163` 现有命令。 | manifest 协议与同步层已存在。上游 WebDAV 改进应落到对应协议/归档/兼容用例，避免重造同步中心；此协议也不等于每目标本地改动备份。 |
| 原生软件目录 | `src/pages/agents/AgentDirectory.tsx:599` AgentDirectory，`src/pages/agents/Page.tsx:62` 现有扫描控制器、`:171` 页面入口。 | 沿软件详情与现有 Skills 页面接入上下文入口，无需另造总仪表盘或统一任务平台。 |

## Skills：旧结论复核及必要修正

### 仍成立的三个风险

`src-tauri/src/services/skill.rs:1194–1202`：更新在下载并重新读取安装记录后，执行 `let _ = Self::create_uninstall_backup(&skill)`，随后删旧 SSOT 再复制新内容。备份返回 Err 不会中止，复制新主库失败也没有该函数级别的自动恢复。

`src-tauri/src/services/skill.rs:1233–1243`：metadata 先持久化，然后遍历 `updated_skill.apps.enabled_targets()`；任一 `sync_to_app_dir` 出错只 `log::warn!`，最后 `Ok(updated_skill)`。现有 `SkillsPort.update` 直接调用 `update_skill`（`src/shared/platform/tauri/feature-ports/simple.ts:46`），没有逐目标结果 DTO。

`src-tauri/src/services/skill.rs:3198–3216`：备份源先取 SSOT；不存在才遍历目标并返回第一个存在目录。`:3305–3312` 只复制该 `source_path` 到一个 `skill/`，metadata 存一个来源。复制目标上不同个人修改没有逐个保存。

### 不能漏掉的现有保护

- 更新前会校验 directory，下载后重新确认记录存在、目录/仓库/安装时间未变（`skill.rs:1108–1111`、`:1177–1192`）；不能写成毫无并发保护。
- metadata 更新函数只更新现有行，并重新读取 DB 的权威 app flags，避免网络等待期间覆盖开关或重新插入已卸载 Skill（`:1085–1099`）。这保护的是记录身份/分配状态，不是内容预览的 digest。
- QoderWork、TRAE Work、WorkBuddy 固定 copy 目标使用专门 vendor writer（`:1781–1785`），其中有临时目录、前像、rename 与读取校验；不能用 legacy Copy 的弱点概括每个目标。它仍不能补足 update_skill 的全目标备份与结果语义。
- legacy `replace_dest_with_copy` 先复制到 tmp，但之后移除 dest 再 rename（`:1882–1915`）；它不构成整项更新的补偿事务。
- `InstalledSkill` 记录 repo branch、一个 content_hash、timestamps（`app_config.rs:447–481`）；`check_updates` 优先用 DB hash 与远端目录 hash 比较（`skill.rs:1044–1077`），不读取各复制目标的改动。没有旧版完整内容时，不能只凭 hash 断言哪段是个人修改，也不能做可靠三方合并。

### 最小可接手的安全更新边界

建立 Skill 专用 inspection/snapshot/writer，而不改下载发现业务的身份语义。inspection 至少返回主库及所有启用目标的实际根、Copy/Symlink/共享关系、整目录摘要、旧发布基线是否可得、读写可用性；“本地不同”与“确定属于个人修改”必须分开。先支持统一确认版，持续分叉为多个版本不是默认隐式行为。

writer 必须在任意写入前完成所有实际受影响位置的快照；应用前检查 plan digest、所有目标摘要和链接关系；任一目标失败给逐目标 partial result，支持按已记录前像恢复并回读。若共享 symlink 指向 SSOT，修改一次即影响多目标，不能列成两份独立副本。若缺基线只展示差异和未知来源，不编造合并依据。

## ChangePlan 的复用范围和不能直接照搬之处

`src-tauri/src/services/change_plan/domain.rs:3` 的协议版本为 `fyagent-change-plan/v2`，`:32–36` 只有三个 operation。`:53–62` 明确 Precheck → Snapshot → ManagedWrite → Readback → Finalize；`:219–224` partial result 可表达已完成、已补偿、未验证、剩余影响与人工动作。`:229–247` plan 带 plan/baseline digest 和 expires_at。`:252–275` job 带 revision/event_seq、资源结果、恢复状态与 usageEvidence。

`commands/change_plan.rs:15/30/48/63/108/123/141` 已有 plan、apply、cancel、get、recoverable list；`:74–100` writer 仍明确绑定 ProviderService 与 WorkBuddy。`services/change_plan/service.rs:726` 做 precheck；`:1693` plan digest；`:1885/2056` 按真实回读分类结果；测试函数包含 stale/过期/重放不调用 writer（`:2913/3045`）、并发一次消费（`:2969`）、中断后读取恢复不重放（`:3333`）。这些仅为已读的代码和测试内容。

前端复用点：`src/shared/features/change-plans.ts:185` ChangePlansPort、`src/shared/platform/tauri/feature-ports/changePlans.ts:104` Tauri port、`src/shared/features/change-plans-ui/ApplyWorkspace.tsx` 共享执行呈现、`ChangePlanWorkspace.tsx:18` 现有切换预览、`useChangeJob.ts` job 观察；`src/pages/models/apply/SavePlanWorkspace.tsx:43/101/154` 已支持泛型 Request → create → ApplyWorkspace。DB 的 `create_change_plan_tables_on_conn` 位于 `database/schema.rs:1826`。

现有 DTO 的 `targetProviderId/currentProviderCode/targetProviderCode` 和资源 enum 带模型领域假设；它不是任意操作插件注册总线。合理工作是给现有框架增加明确的 Skill operation/资源以及适配字段，保留旧操作兼容、幂等与验证语义；不是复制这些表/API/状态机到 skills，也不是把 `update_skill` 简单包成一个成功/失败 bool adapter。

## 上游新增能力的定点核验

### 多供应商聚合目录和模型 ID 路由

已存在：单供应商 model fetch、Codex model catalog 生成与自有路径识别、代理 API 转换、ProviderCredentials、故障转移与 usage 记录。`proxy/handlers.rs:74–111` 的 `handle_models` 读取当前 Codex 配置所指向的 FyAgent catalog；`proxy/provider_router.rs:32–37/61–105` 的 `select_providers(app_type)` 返回当前供应商或按 failover 队列优先级返回候选。

缺口：没有在该路由契约上看到“模型目录聚合多个 Provider，并由 provider 前缀模型 ID 定向选中 Provider”的通用实现。目录中出现 OpenRouter 风格的 `vendor/model`、协议转换前缀归一化、Claude Desktop 模型映射，均不能单独作为此能力的证据。

推荐任务边界：单独的聚合目录/路由兼容试验；先固定模型 ID 命名、冲突、无前缀默认路由、显式路由是否允许 failover、key 隔离、目录缓存失效、usage 归因，再连接现有 router/forwarder。首轮可标为 prerelease 评估，不直接宣称生产切换。

### 配置 patch / profile 保护

现有 source_switch 已按字段所有权 patch。`source_switch.rs:252` 测试夹具含 user header、features、MCP、unrelated provider、profiles.personal；`:255/351` 有保留配置及 snippet overlay 用例；`codex_config.rs:2257` 测试 provider ID / profile 引用保留。这是一项已具基础、待针对上游新增边界进行差异审计的能力，而非空白。

上游迁移任务宜列成：收集新增边界案例 → 映射现有 projection/writer → 补缺失用例/最小修复。不可无差别替换这些 writer，因为 FyAgent 已叠加 vault/managed proxy、source ownership 和 ChangePlan 投影一致性。

### 只读结构化会话 timeline

`session_manager/mod.rs:33–38` SessionMessage 仅 role/content/ts。`providers/codex.rs:232–257` 把 function_call 降成 `[Tool: name]`，function_call_output 为纯字符串；其他事件默认跳过。`src/pages/sessions/components/ConversationStream.tsx:162–163` 渲染 msg.content。因而能看见某些工具输出不等于具结构化工具调用、参数、结果关联、附件及 usage 时间线。

合理边界是新只读 timeline DTO/parser + 展示组件，保留既有 message fallback；原始日志解析失败按事件降级、避免一个未知块丢整段；大日志需分页/读取预算。迁移抽取会故意剔除工具和 reasoning（`src/shared/features/session-migration.ts:77–87/457`），不能为了 timeline 改掉迁移内容规则。既有 native restore 模块保留；PR #198 的恢复门控和安装 ownership 工作不得重复立项，也不得算作当前主干已验证。

### MiniMax Code / mcode

当前检索未见 `mcode` app_type、client config adapter、Skill/MCP 目标或会话解析器。若选择迁移，应先做客户端配置位置、版本、恢复方式及目标域能力矩阵；然后沿 AppType 与 target 模型按需增加模块，禁止先把旧 provider preset 命名改成“已支持客户端”。本轮未安装或运行 MiniMax Code。

## 给后续 Issue 的模块划分建议

| 建议边界 | 所有权 | 最低验收证据 |
| --- | --- | --- |
| Skills 检查和多目标快照 | `services/skill`、必要 DTO/DAO；避免与 UI 并行改同文件 | Copy/Symlink/主库缺失/目标个性修改；备份任一失败必须零写入；目录/链接越界拒绝；本轮真实 fixture 测试 |
| Skills ChangePlan adapter / writer | `services/change_plan` + Skill writer；协议兼容、阶段、digest、partial result | 预览后外部修改 → stale 零写入；写到第二目标失败 → 逐目标状态和恢复；中断后读回；重复 apply 不重放 |
| 原生更新审阅入口 | 现有 Skills 页 + shared change-plans UI/port；不新建仪表盘 | 能辨认源、主库、目标与链接关系；明确备份/冲突/未知基线/过期预览；只对实际回读成功显示成功；`runtime_screenshot` 验收 |
| 上游安全兼容回归吸收 | Codex/source_switch、Provider writer、WebDAV 对应 owner；按修复主题拆 | 现有个人 profile/MCP/注释/keyring保留；非法配置零写入；回滚及下游读取；不能用上游 release note 代替本地测试 |
| 多供应商聚合目录试验 | Model fetch/catalog + proxy router/forwarder + usage | 双 provider 同名模型、前缀选择、无前缀行为、failover语义、授权隔离、归因、禁用回退 |
| 只读 session timeline | `session_manager/providers`、专用 DTO、ConversationStream | 多来源真实脱敏夹具、调用/结果关联、未知块降级、大文件、只读无写入；不改变 native restore |
| mcode 客户端适配评估 | AppType / config / target / session 对应 owner | 先确认支持域；至少一个真实版本的读写/恢复/软件重载证据才标支持 |

优先顺序建议：先收口 Skills 安全更新底座与现有 ChangePlan 复用，配置兼容吸收可独立推进；结构化 session timeline 与聚合模型目录按产品取舍独立排期，mcode 先做需求和客户端合同核验。不要把所有内容合成一个“上游升级”大 Issue，也不要为上游已存在的下载/代理/目录/同步模块重建第二套系统。

## 验证边界

当前仅固定 main 的代码事实和风险分析。未取得本轮正式安装版本、原生 UI、账号授权、软件重载、真实跨目标更新、WebDAV 远端同步、聚合模型执行或 native session 恢复的运行证据。未来交付应分别报告单元/集成测试、原生启动身份、`runtime_screenshot`、真实目标软件回读或重载，不能把其中一个扩大成全部通过。
