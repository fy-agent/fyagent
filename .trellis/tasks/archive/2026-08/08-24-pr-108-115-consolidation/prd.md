# 六条旧 PR 成果整合与单一替代 PR

## Goal

在最新 `origin/main` 上重构 #108、#112、#113、#114、#115 中仍成立的成果，保留 #109 的关闭来源记录，形成一个 commit、一个替代 PR；完成本地、跨平台 CI、外部审查与治理回读后进入 `awaiting_human_review`，不合并 `main`、不发布。

## Frozen facts

- 执行基线：`e94307cd810d7c5157b3791da2a8d7ef6a01b8a7`。
- 基线数据库版本：Schema v19；本任务使用 v20。
- 隔离分支：`codex/pr-108-115-consolidation`。
- #109 已关闭且未合并，只保留来源留痕。
- #112/#114 在本任务治理动作前已关闭且未合并；#108/#113/#115 与执行中纳入的 #130 仅在替代 PR 初轮 `CI / Required` 全绿后关闭。#112/#114 在门禁后补充迁移说明。
- 独立 SecretRef Draft #132 与独立 executor Draft #134 不在本任务授权范围，本任务不修改其状态。
- 明日唯一人工 reviewer：`python-rust`。

## Product requirements

1. Change Plan 是唯一正式变更账本，含 `change_plans`、`change_jobs`、`change_job_events`，并保存 DB current 与 device current 两套基线。
2. 创建计划零网络、零业务写；Apply 在现有 Provider mutation lock 内校验 digest、TTL、消费状态、基线与 Secret capability，writer 最多一次；reconcile 只读回，禁止重放 writer。
3. 首批仅支持“不引入新凭据材料”的已保存 Codex Provider 切换；无法证明时返回 `secret_dependency_unavailable`，确认禁用且 writer=0。
4. `projectionDigest` 只作为未来合同记录：RFC8785、domain-separated、64 位小写 hex、无 `sha256:`、排除自身字段；本任务不加入未消费的 SecretRef、Keychain、明文 fallback 或 Secret UI。
5. Windows 名称脱敏使用跨平台纯词法规则覆盖 Unix、盘符、UNC、反斜杠、`file:`、控制字符、中文和长度边界。
6. Apply UI 只消费真实 `ChangePlan` / `ChangeJobSnapshot`，不存在 fake coordinator、scenario、Cancel、Backup、Restore 或第二状态机；成功和 warning 都明确“尚无真实使用证据”。
7. #108 的 Grok Official/xAI 文案、四语言资源与测试完整迁移。
8. Agent Install 不创建第二份 catalog，只接受当前七个 canonical IDs；只新增 `get_agent_install_readiness` 一个只读命令和 `/agents` 详情只读区块。
9. 所有 generic automation 为 unavailable；Codex 为 `managed_by_codex_desktop` 并保持现有真实安装面；不加入 installer/executor/job/cancel/fake doctor/helper。

## Scope exclusions

- 不实现 Secret Backend、Keychain、通用 Agent 自动安装、真实 doctor、Windows helper HIL、UAT 或发布。
- 不迁移旧 V1 UI、旧 schema、旧 Agent IDs、独立 registry、fake runtime 或未经重新核验的许可/签名声称。
- 不合并 `main`；人工 Approve 和后续合并属于明日门禁。

## Acceptance criteria

- [x] 相对冻结基线只有预期范围，且 `origin/main` 漂移已显式重基线或阻断。
- [x] Schema 0→20、19→20、未来版本拒绝、memory DB、同步跳过/本地保留通过。
- [x] Change Plan 的零副作用、最多一次 writer、并发、失败/恢复、Secret 阻断、Windows 脱敏均有自动化证据。
- [x] V2 严格 IPC、native-only、StrictMode/双击、失败不绿、无 fake/cancel/backup/restore 通过。
- [x] Agent readiness 七 ID、单只读命令、无敏感 DTO、无按钮、Codex 安装面不回归通过。
- [x] Grok i18n、Provider selector、Subscription footer 测试通过。
- [x] V2、Rust、Repository Contracts 全部本地门禁通过。
- [x] Grok 与独立 `trellis-check` 回执无未解决 P0/P1；Gemini 若因外部账号/客户端阻断，必须保留真实 BLOCKED 回执且不得冒充 PASS。
- [x] 最终分支相对最新 `main` 恰好一个 conventional commit。
- [x] 替代 Draft PR 初轮 Required CI 全绿后，仍开放的来源/派生 PR 均独立说明并关闭，#108/#109/#112/#113/#114/#115/#130 回读均 `closed + merged=false`。
- [ ] 治理证据 amend 后 product diff digest 不变，最终 Required CI 再次全绿，PR 转 Ready；最终状态-only amend 的最新-head CI 与 Ready 回读以 PR #135 为权威。
- [ ] 结束状态为 `awaiting_human_review`，`main` 未变化、未发布；最终 GitHub 回读后生效。

## Completion truth

本任务今天的完成定义是“替代 PR Ready、CI 全绿、旧 PR 关闭、分支冻结、等待 `python-rust` 人工 Review”，不是“已合并”或“已发布”。
