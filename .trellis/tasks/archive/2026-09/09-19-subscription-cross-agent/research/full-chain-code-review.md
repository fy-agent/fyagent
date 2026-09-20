# 订阅账号跨 Agent 全链代码评审

日期：2026-09-19。工作树：`subscription-cross-agent-20260919/fyagent`。基线：`64f4d8f6`。原缺陷行号保留首次发现位置；修复按当前函数和实际代码复核。

本报告区分源码核对、单测/集成测试和真实订阅调用。未读取真实账号秘密，未向厂商发请求，未启动 Cargo；测试统一由主控运行。OpenCode 按当前 `design.md` 独立 loopback 方案评审，旧 native credential lineage 复用建议已 superseded。

## 当前关闭状态

| 项目                           | 静态复核                                                                                  | 运行证据                                                                                    |
| ------------------------------ | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| F1 OpenAI 请求/原生非流响应    | 已关闭：请求策略共用，SSE 聚合和 incomplete 保留                                          | managed_responses 5/5、transform_responses 76/76；listener 回归在最终 subscription 49/49 内 |
| F2 正常停止/崩溃恢复           | 已关闭：receipt 意图 hash、raw preimage、重绑/逐文件恢复 gate、0.4.5 无 proof 两阶段准入  | proof-window/rebind/shutdown/restart/旧版升级/逐文件漂移回归在最终 subscription 49/49 内    |
| F3 失败补偿                    | 已关闭：逐快照 receipt 检查，写边界再次 guard，全部恢复成功后才清 backup；Codex auth 排除 | 补偿、sidecar 和新增 common guard 回归均在最终 subscription 49/49 内                        |
| F4 多账号 overview/移除影响    | 已修复并复核                                                                              | 3 项新 overview 回归已通过                                                                  |
| F5 OpenCode API Key 冲突与退出 | reserved-ID 保护、明确恢复入口、双 owner 回读、API 编辑隔离已核对                         | 后端隔离回归通过；reviewer 恢复 UI 11/11、ESLint、Prettier 通过                             |
| F6 既有三目标重启绕过托管事务  | 四目标统一 resume，generic managed enable 拒绝，启动/命令路由已核对                       | 四目标 restart 回归已通过                                                                   |

最终功能源码已冻结，F1–F6 无剩余已确认代码缺陷。Reviewer 直接核对定向日志；主控随后完成全库 prearchive gate（exit 0），真实订阅账号/厂商额度/目标 CLI UAT 尚未验证。

## 已发现并修复

### F1 — P1：OpenAI 原生 Responses 未应用完整 ChatGPT 请求规范

- 原位置：`src-tauri/src/proxy/providers/managed_responses.rs:25-44`、`src-tauri/src/proxy/forwarder.rs:1549-1551`、`src-tauri/src/proxy/handlers.rs:901-909`。
- 触发：通过 Grok Build / OpenCode 的原生 Responses route 使用 OpenAI 订阅，发送缺少 `instructions/tools/parallel_tool_calls` 或带 `temperature/top_p`、`stream:false` 的请求。
- 证据：原最终 wire policy 仅设置 `store=false`、删除 `max_output_tokens`、补 encrypted reasoning include；完整规范在 `transform_responses.rs:396-442`，但只有 Claude Messages 转换调用。原生 Responses 不经过该转换。后者已有明确的字段/强制流单测；现有 fake upstream 未严格检查这些条件，掩盖原生路径差异。该结论是代码与既有协议契约核对，未将其描述为真实厂商重现。
- 请求侧修复：按主控授权，在 `managed_responses.rs` 提取 `prepare_openai_generation`，两种入口复用同一个幂等策略；补全部必需缺省字段、移除不支持的采样字段并统一上游 SSE。显式工具、调用结果、reasoning 历史、指令、parallel 值和 service tier 保留。Claude FAST 开关仍只在原转换 owner 决定。Compact、原生官方账号/API Key 和 xAI 路径不扩大该策略。
- 新/更新单测：`openai_final_policy_is_idempotent_and_does_not_touch_native_or_api_key`、`openai_native_responses_preserves_tools_history_and_explicit_policies`、`openai_minimal_native_request_gets_generation_defaults_without_fast_mode`。原 Claude FAST 开/关、采样字段删除与缺省字段测试继续保留。
- 本 reviewer 已完成请求侧修复和 `rustfmt`。后端已在 `handlers.rs:890-914` 接入原生非流 SSE 聚合，并增加 `subscription_opencode_responses_stream_json_tool_roundtrip_and_no_api_key_fallback`：临时 listener 对 OpenAI 必需字段执行严格校验，覆盖两厂商、stream/JSON、function call/output、账号 header 与禁止 API Key failover。该测试在最终 subscription 49 项中通过。
- 最终复核补充：原聚合 helper 忽略合法 terminal `response.incomplete`，会把非流原生 partial 结果变成通用 422（P2）。后端现已在 `responses_sse_to_response_value` 同时接受 completed/incomplete，保留 response payload；新增 fixture 已通过。

### F4 — P2：非默认账号的真实 Proxy 连接在 overview/删除预览中缺失

- 原位置：`src-tauri/src/services/managed_auth/service.rs:1025-1032,1120-1127`。
- 触发：登录 A，再登录 B 使 B 成为默认；显式把 A 绑定到任一 Agent。
- 证据：请求 resolver 确实解析绑定的 A；原 `upsert_proxy_connections` 却只为 provider 的 default credential 建 slot，`overview_inner` 又以 `row.is_default` 筛选观察。完整 overview 只能显示 B，A 的 `connectedConsumerCount` 为 0；删除 A 的影响预览也漏掉真实依赖。
- 主控确认方案后授权 reviewer 修复：`service.rs:1016-1084` 为所有 OpenAI/xAI proxy-purpose credential 建稳定私有 slot，overview 按 persisted credential 精确观察，`target_id` 仍为空，不伪装 lifecycle target；公开 DTO 不变。Copilot 默认选择保持既有语义。删除预览先同步这些连接。
- `database/dao/managed_auth.rs:883` 的清理仅处理可确认属于旧托管 proxy 且空 target 的 vendor slot、以及找不到匹配 proxy-purpose credential 的保留前缀 slot；保留用户自定义 slot、其他 consumer、非空 target。按 consumer 去重的计数不变。
- `proxy_overview_tests.rs` 新增三个回归：A 非默认/B 默认同时服务两个 target，两账号完整 overview/删除预览正确；superseded/orphan 清理不影响用户连接；同 identity 多 credential 只计一个 consumer。测试使用临时 home、内存 SecretBackend 和真实本机 listener 状态，无真实 token/上游请求。
- `rustfmt` 已通过；已直接核对 `backend-subscription.log:26-28`，三项新回归全部通过。原“需要公共连接模型决定、未修复”说明已 superseded。

## 跨 owner 修复与最终复核

### F2 — P1：托管订阅停止/崩溃恢复覆盖绑定后的外部修改

- 原位置：`src-tauri/src/services/proxy.rs:2238-2261`；调用入口为 `stop_with_restore_keep_state`、`stop_with_restore`、`recover_from_crash`。
- 触发：成功绑定 Claude/Codex/Grok 后，外部 CLI 或用户修改同一配置文件的权限/MCP/模型字段，再退出 FyAgent 或走 crash recovery。
- 证据：只要 DB live backup 存在且备份没有占位符，恢复即直接写回完整旧备份；没有当前 postimage/receipt 准入。之后正常流程删除 live backup，外部内容被静默覆盖。通用 atomic writer 的写前备份不是对这次恢复的外部变更准入。
- 建议：托管订阅保存/校验其实际投影 postimage 或权威 receipt；漂移时保留文件和 recovery evidence，返回冲突，不能从 DB backup 无条件覆盖。其他非托管 legacy 路径不要借此扩大重构。
- 回归：两种 provider × 三个既有目标，在 bind 后写入不同字节，再分别 stop / crash recovery；断言文件保留、backup 保留、不报告恢复成功。正常未漂移恢复仍应完成。
- 第一轮 proof 修复又发现两处：写后 readback 到登记 proof 之间可能采样外部内容；重复 bind 会把旧 proof 外的修改重新承认为己有。后端现已以 `file_mutation_expected_hash` 读取本次 synchronous operation 的 publication receipt；`mark_managed_restore_proof` 只比较 expected 与 current，不从新采样构造 ownership，pending 前验证旧 proof。`subscription_managed_proof_window_and_rebind_refuse_external_edits` 已在 45 项日志中通过。
- 旧三目标现保存 raw preimages（包含原先不存在的文件），Codex 只拥有 config/catalog，原生 auth 不在恢复集合。OpenCode 将 projected 的 canonical 写入 bytes 预先哈希，恢复/再绑定对实际 bytes 比对；单纯空白漂移也不能当 intact projection。
- 最终补充的同项边界已关闭：`restore_verified_managed_config` 逐 path 将原 proof 的 expected hash 传给 `restore_file_preimage_if_owned`，并在写边界重验；OpenCode 在允许的 current pre/postimage 核验后也调用同一 guard。`subscription_managed_restore_preserves_catalog_edited_after_first_file` 受控地在 Codex 第一个文件恢复后改 catalog，确认后者保留、backup 不清，最终回归通过。
- 0.4.5 升级边界已关闭：`lib.rs:1863-1877` 确实先执行 crash recovery；两个 restore 入口现在均调用 `require_managed_restore_proof`，当前明确 managed 的旧 plain backup 拒绝恢复，保留 bytes/backup；普通 API 旧备份保留 legacy 路径。备份缺失而 live 仍有占位符时，同一 gate 阻止 SSOT 猜测重建。
- 自动 resume/再次 bind 的第二阶段也已关闭：`prepare_managed_takeover` 区分已有 backup 与本次新建 backup，已有项必须通过 `validate_existing_managed_backup` 的 marker/hash 检查，不能在 recover 失败后把旧 plain backup 重新提升为 proof。`subscription_upgrade_refuses_unproven_legacy_backup_but_keeps_recovery_evidence` 覆盖三旧目标恢复与 resume 均拒绝、证据保留，最终通过。
- 从已处于普通 API 接管的 plain backup 直接切 managed 会要求先关闭原接管，不将当前 placeholder 存成 raw original。`subscription_does_not_adopt_legacy_proxy_placeholders_as_native_preimage` 已通过。该保守升级行为是明确的保护边界：不能宣称历史无 proof 状态会自动恢复成功。

### F3 — P1：激活失败补偿同样缺少外部 postimage 准入

- 原位置：`src-tauri/src/services/provider/mod.rs:359-360,4962-4966` → `config.rs:417-418` → `config/recovery.rs:347-351`。
- 触发：本次 native projection 之后，外部 writer 修改文件；随后 DB persistence/readback 失败进入 activation compensation。
- 证据：`QuickSetupFileSnapshot::restore` 只传原始 bytes，底层 `restore_preimage` 加 writer mutex 后直接 `replace`。它不检查当前文件是否仍是本次写入的 postimage。全部 captured 文件被无条件回写，随后只检查是否等于旧快照，因此可以覆盖外部字节并报告 `apply_failed_rolled_back`。
- 建议：事务 owner 记录其写入 receipt/postimage；未改文件只允许与原快照一致，已改文件必须匹配本次投影。无法证明 ownership 返回 `rollback_partial_state_unknown` 并保留外部内容和恢复证据。不能把低层 mutex 当成跨进程保护。
- 回归：native projection 后用受控 hook 插入外部修改，再触发后续 DB/readback failure，断言外部内容保留且返回 unknown；无外部修改的补偿继续返回 confirmed rollback。
- 后端最终补偿 gate 遍历每个 `QuickSetupFileSnapshot::is_owned_by_current_operation`，仅当 current 等于原快照或本次 receipt 的 expected hash 才允许。`restore_owned` 再在 common writer 边界验证，避免逻辑检查后变化；全部文件恢复成功才清理/恢复 DB backup。原只验证 OpenCode 主文件、却恢复 `.backup` 的 sidecar 缺口也关闭。
- Codex native auth 在 admission/restoration/final verification 都被明确排除，允许原生 refresh 轮换；没有把 config/catalog 的 proof 扩大为 auth 所有权。
- 已通过的回归包含四目标 projection 后外部编辑/事务失败、只改 OpenCode `.backup` 的补偿，以及 `subscription_compensation_rechecks_postimage_at_the_writer_boundary`。已删除被替代的 dead helper，最终后端日志无 warning。

### F5 — P1：OpenCode 普通 API Key writer 误覆盖唯一的订阅 Provider

- 位置：`src/pages/models/OpenCodeModelsPanel.tsx:104-106`；`services/opencode_models.rs:443-455,476-535`。
- 触发：初始 OpenCode 无 provider → 应用订阅（唯一 reserved managed provider）→ 按原有模型页面保存普通 API Key。
- 证据：页面选首 provider 作为普通来源；`resolve_provider_id` 的单 provider fallback 无条件复用 reserved managed ID，writer 随后改 baseURL/apiKey，却保留 `@ai-sdk/openai`（普通 API Key 默认应为 `@ai-sdk/openai-compatible`）。全局默认 model 仍引用托管 ID；DB current/backup 则仍认为订阅受托管。保存返回成功，之后 bind/restore 因漂移拒绝。
- 建议：普通 writer 排除/拒绝 reserved managed 项；同时提供明确的停止订阅入口，复用现有 managed restore/disable owner，成功后重读 snapshot+overview 再开放普通保存。单纯禁用普通保存无法完成切回；正常退出的 keep-state 会在下次启动自动恢复，不等于停用。
- 后端现拒绝普通 writer 在存在 managed reserved 项时写入，并禁止普通 providerName 创建 reserved ID；前端 `OpenCodeModelsPanel` 过滤 managed 项，不将其名称/模型喂入 API Key 编辑，同时封住保存/删除。
- `OpenCodeSubscriptionRestore` 提供确认后的恢复入口，窄 port 使用既有 literal `set_proxy_takeover_for_app`（opencode,false），共享锁和恢复 owner。只在 snapshot/overview 均重新读取且无 managed 残留后调用 `onRestored`；失败/残留以及卸载后的迟到失败继续向父级发 block。成功恢复后重挂 picker 清除旧“已应用订阅”提示。
- 配置回读 P2 也关闭：安全 snapshot 只补 `selectedModel` 引用；bind 后必须与 providerId/modelId 精确匹配，并确认 provider 的模型集合。没有把该字段描述为持续 listener 监控或真实调用证据。
- Reviewer 新 `OpenCodeSubscriptionRestore.test.tsx` 最终 11/11 通过，涵盖两 owner 顺序、所有错误、source/target 隔离、父级写锁，以及 snapshot 先完成导致控件卸载而 overview 后失败的真实时序。后端 API writer/backup isolation 回归已通过。

### F6 — P1：既有三目标在重启后绕开 managed activation/proof

- 位置：`lib.rs:2912-2937`、`services/proxy.rs:1201-1211,1390-1650,1855-1890,2022-2088`。
- 触发：首次订阅 bind 成功 → FyAgent 正常退出（keep_state 恢复 original 并删除 backup）→ 再启动。
- 证据：只有 OpenCode 使用内部 managed resume；Claude/Codex/Grok 走 generic `set_takeover_for_app(true)`。它新建未带 managed proof 的 legacy backup，并把恢复后的原 API Key 回填当前 managed Provider（三个分支均未跳过 subscription）；随后 generic takeover 不建立 proof。Grok original 若是官方登录且无自定义 models，generic takeover 直接拒绝，订阅无法自动恢复。
- 建议：所有 managed current 的启动恢复复用专用 managed activation/prepare（内部 resume 不要求用户 revision），禁止从 old native config 回填 API Key；保持未知/漂移失败保留证据。不要仅补 OpenCode。
- 后端已统一 `ProviderService::resume_managed_proxy`，四目标启动都先走该路径；普通 API 返回 false 才回到旧接管。命令级 enable 也复用 resume，底层 generic `set_takeover_for_app(true)` 遇 managed current 拒绝，阻止其他调用者绕回 token backfill。
- `subscription_managed_restart_keeps_proof_exact_preimage_and_secret_free_provider` 已通过：四目标 keep_state 后原 bytes 恢复，resume 不改变 Provider（不回填 native-key），proof 重建，随后外部 drift 使再次 stop 失败且文件保留。旧无 proof 历史备份使用 F2 独立升级回归，未混用新格式证据。

## 已核对的边界

- 绑定从 public account identity 精确解析 ready `ProxyUpstream/FyagentProxy/Fyagent` lineage，并读取 SecretRef bundle/generation；没有将 OpenCode/Codex/Grok native purpose 交给 Proxy。
- managed Provider 只保存 account reference、供应商类型和选定模型；转发前取短期 token。新的 `fyagent-openai-*` / `fyagent-xai-*` 账号丢失后拒绝 legacy fallback。
- Claude/Codex adapter 为 managed provider 固定官方 upstream origin，并禁止 editable full URL 覆盖；xAI 最终 token-auth/model-override headers 覆盖客户端值。OAuth origin 校验与固定 OpenAI scope 已沿代码核对，没有推定所有 scope 都等于真实订阅 entitlement。
- refresh lock/CAS 对 credential/identity/purpose/consumer/owner/secret handle/auth epoch 做同 lineage 校验；401 至多同账号重放一次，重新登录/删除/转移 owner 后的迟到结果不提交。原生 token 不经 renderer 回传。
- 既有 Claude Responses 转换保留 function call/result、处理 `response.failed/error/incomplete`，OpenAI 非流 Claude 调用聚合 SSE；原生 Responses 新行为由 F1 集成回归补齐。
- 前端 pending OpenCode request 捕获确认时 revision；严格 port 校验 required keys/public account identity/model/result target/activation。未确认结果及 bind 后 owner-read 失败传播到 Models parent 的 target block，切换 target 不清除 block。API Key 与订阅共用同步写 guard。

## 未修复项与验证边界

- 无剩余已确认 P0/P1/P2 代码缺陷。0.4.5 无 proof 旧 managed 备份会停在“需复核”并保留文件，是预期的失败关闭行为；本次没有推定历史字节并自动覆盖。
- 主控最终全库 prearchive gate 已通过；下面 reviewer 定向检查保留其实际范围，最终总检查见 `../verification.md`。
- 真实账号登录、厂商模型 entitlement/额度、目标 CLI/Desktop 实际调用，留待用户 UAT。静态代码、临时 home、fake upstream、本机 listener 和浏览器 fixture 都不替代真实订阅证据。

### 并行分支合并与 UAT 目录边界补充

- 订阅功能提交 `61040031` 和并行 FDE 提交 `cd8b4ced` 都将 schema 21 升至 22；前者扩展 `proxy_config` 的 OpenCode 约束/默认行，后者增加客户项目、资源代际和验证记录结构。源码已核对，相同版本号不代表相同结构，两种候选不得交替读取同一真实目录。正式合并需另做统一版本与两种 22 结构的升级验证，当前未实施该迁移。
- 本订阅候选 UAT 必须使用独立 `FYAGENT_TEST_HOME`。macOS `config::get_app_config_dir` 优先采用 `app_store` 缓存的自定义目录；同应用标识的 `app_paths.json` 不受测试 home 自动隔离，现有外部覆盖可能把数据库重新指向正式目录。应在启动前核对，无法确认隔离时不得启动，也不修改公共 Store 来绕过此条件。
- 目标客户端的测试配置入口需与 FyAgent 的投影路径一致；测试 home 环境变量不会替其他 CLI 完成配置隔离。应用、ZIP 和功能代码保持已冻结版本；更新后的启动条件、源码对照及后续合并要求见 `../verification.md`。

## 验证状态

- Reviewer 修改范围：`managed_responses.rs`、`transform_responses.rs`、Managed Auth `service.rs`/`repository.rs`、DAO `managed_auth.rs`、新 `proxy_overview_tests.rs`、新 `OpenCodeSubscriptionRestore.test.tsx`、本报告；没有修改其他执行端独占的源文件/既有 tests。
- `rtk proxy rustfmt --edition 2021 --config skip_children=true <上述 Rust 文件>`：PASS；已清掉测试的 unused `AppType` import。
- Reviewer 定向 UI：Node 24.19.0 / Vitest 3.2.7，11 passed，日志 `.trellis/.runtime/verification/subscription-restore-ui-review-final.log`；单文件 ESLint 与 Prettier PASS。第一次默认 Node 24.13.0 被仓库精确版本 gate 拒绝，没有运行 tests；已改用本机 mise 的正确版本，不改 gate。
- 已直接回读最终后端日志：`backend-subscription-final.log` 49/49、`backend-proxy-final.log` 59/59、`backend-managed_responses.log` 5/5、`backend-transform_responses.log` 76/76。最终 subscription 结果包含 3 项 overview、正常恢复 writer guard、legacy 升级/placeholder、OpenCode route/config/恢复、incomplete 与错误回归；最终两份 `*-final.log` 无 warning/error。前序 45 项日志被最终 49 项 superseded。
- 初期 `frontend.log` 的 source inventory 失败已由最终总检查 superseded。主控核对精确源清单、test-only 目录 guard 及负例，并同步新增 IPC/迁移目标的固定合约断言；没有放宽产品约束。最终前端 1752 passed/1 skipped，Rust 3611 passed/6 ignored，严格编译/Clippy 和完整合同检查通过，见 `prearchive-final.log`。
- 已直接回读主控日志 `browser.log`：636 passed；`browser-subscription-final.log`：20 passed（覆盖最后恢复 notice 行为）。它们是 browser fixture 证据，未声称真实 CLI 使用。
- 统一执行 filters 已交 backend：`managed_responses`、`transform_responses`、`managed_auth::service::proxy_overview_tests`。本 reviewer 未抢 Cargo 锁。最后 Rust gate/整体 TypeCheck/源清单由主控负责。
- Reviewer 功能代码已冻结；主控总检查已完成，用户真实账号 UAT 仍待执行。
