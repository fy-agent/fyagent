# CC Switch v3.20.4 Upstream Provenance

This ledger records the source identity and ancestry of the CC Switch v3.20.4
integration. It is not a FyAgent Release Note and does not change FyAgent's
product version from `0.4.10` to the upstream version.

**标签祖先关系已集成；行为移植按组列状态；Rust 验证待完成。**
2026-10-07 fix1 仅解除数据库/备份的已知源码阻断并明确数据库增量，
不是 S02/S03/S04/S23 的功能验收，也不表示整个 Rust 工程已可编译。
2026-10-07 fix2 清理剩余调用/定义、类型及测试接线阻断；Pi/Mcode 推迟到 #208，
检查证据仍为 `code_audit`，不宣称 Rust 类型检查或原生运行已通过。

## Verified source and graph

| Field                       | Verified value                                         |
| --------------------------- | ------------------------------------------------------ |
| Authorized FyAgent baseline | `5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798`             |
| Merge first parent          | `5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798`             |
| Upstream repository         | `https://github.com/farion1231/cc-switch.git`          |
| Annotated tag               | `v3.20.4`                                              |
| Tag object                  | `93994110505d4d4aae3a7a3582797aae48c2dc74`             |
| Peeled commit               | `43e1d99084ed9b2f5dc252fd35c5adaf29d6876e`             |
| Merge base                  | `43eaf07355af145aebfee301801779e824d4c221` (`v3.19.2`) |
| FyAgent two-parent merge    | `3d1ee4fe5f7d6b6da437cb89315393c510dcf0b7`             |
| Merge second parent         | `43e1d99084ed9b2f5dc252fd35c5adaf29d6876e`             |
| Integration date            | 2026-10-07 (Asia/Shanghai)                             |
| Database schema             | 27 after fix1; FyAgent migrations through 26 preserved   |

The annotated tag object and peeled commit match the approved v3.20.4 identity.
The merge commit has exactly those two parents, with the peeled upstream commit
as the second parent, and that commit is an ancestor of the merge. The merge
was created with explicit `--no-ff --no-commit` semantics. Conflict
resolution remains incomplete; ancestry does not prove behavioral integration. It was not squashed, rebased, or resolved with a repository-wide
ours/theirs strategy.

The shared repository remotes were not rewritten. 因账号原因，origin 暂为
`https://github.com/junshi-fy/fyagent.git`（fetch/push）；upstream fetch 实际为
`https://github.com/fy-agent/fyagent.git`，push 为 `DISABLED_no_push_to_main_repo`。
这与 upstream-sync.md 的 canonical remote 角色不一致；按本轮授权仅记录现状，
不修改配置、不推送、不创建 PR。 The local clone at
`/workspace/fy-maint-1007/migration/cc-switch` is a partial clone and could not
supply the tag's blobs, so `refs/tags/v3.20.4` was fetched from the upstream
URL above after `ls-remote` confirmed the tag object and peeled commit.

## Provenance and license boundary

CC Switch-derived code, history, notices, and attribution retain their MIT
ancestry. The upstream v3.20.0–v3.20.4 release-note bodies were not kept in the
FyAgent tree: they advertise unsupported desktop packages and distribution
install commands, which the supported-platform scanner rejects. They remain
reachable from the second parent. This ledger and the existing FyAgent
CHANGELOG preserve provenance without presenting upstream release marketing as
a FyAgent release.

FyAgent-owned components and modifications remain under the repository's
published PolyForm Noncommercial terms. See [LICENSE](../../LICENSE),
[LICENSING.md](../../LICENSING.md), and
[THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). No upstream partner,
sponsorship, affiliate, or tracking metadata became a FyAgent product claim.

## FyAgent contracts preserved through the merge

- product/runtime identity: `FyAgent`, `fyagent`, `fyagent_lib`,
  `com.fyagent.desktop`, and `fyagent://`;
- version only from `[workspace.package].version = "0.4.10"`;
- persistence correction: the merge kept `SCHEMA_VERSION = 26` but already
  added `skills.enabled_mcode`, `session_log_sync.last_byte_offset` /
  `last_tail_fingerprint`, and `session_usage_dedup`. It also mistakenly
  duplicated FyAgent v16→17/v17→18 method names with upstream migrations.
  Fix1 keeps those storage additions in one explicit FyAgent **v26→v27**
  migration; all historical FyAgent migration bodies through v26 stay intact.
  Keeping the fields supports session readers already present in the merge;
  removing them would require reverting their consumers too. No new Mcode
  skill/MCP product integration is enabled;
- the deleted production renderer stays deleted. New files that import the old
  module graph were not restored;
- capabilities stay on the FyAgent list. `process:allow-exit`,
  `process:allow-restart`, and `dialog:default` were not added;
- fix2 restores the pre-merge client list and removes reachable Pi/Mcode
  Provider, Prompt, Skills, MCP and session wiring. Dormant upstream client
  source files remain unregistered for #208; the retained Mcode storage column
  alone does not provide a usable target;
- native Fetch stays. `cross-fetch` was not reintroduced;
- new implicit broad-family cfg sites were rewritten to explicit macOS or
  Windows. The supported-platform structure manifest was refreshed after that
  rewrite.

## Conflict resolution

The merge started with 274 unmerged paths: 82 content conflicts, 143 paths
deleted by FyAgent and modified upstream, and 49 upstream additions inside
directories FyAgent had renamed. The original report counted 417 hunks in 82 files. These are historical
resolution counts, not behavioral acceptance evidence:

| Resolution | Hunks |
| --- | ---: |
| Overlapping region kept FyAgent data, identity, or behavior | 262 |
| Whole-file rule kept FyAgent (docs, CI, changelog, identity) | 58 |
| Upstream-only addition taken | 27 |
| FyAgent-only addition kept | 17 |
| Partner `isPartner` additions dropped | 14 |
| `utm_` tracking rejected | 10 |
| Unsupported-platform additions rejected | 10 |
| Partner `isPartner` edits rejected | 7 |
| Unsupported-platform additions dropped | 3 |
| `partnerPromotion` rejected | 1 |
| Unions (store, MCP module declaration, command and service modules) | 8 |

Deleted-by-us paths stayed deleted, including the old renderer, old manuals,
and root `vitest.config.ts`. File-location moves into the old UI or partner
assets were not accepted. Clean auto-merges were still scanned: partner banner
assets were removed, and `utm_`, `aff`, `affiliate`, `ic`, `ref`, and
`referral` query keys were stripped from presets. Campaign parameters `ac` and
`rc` on the Volcengine coding-plan URL were stripped because they were not on
the FyAgent baseline.

After the mechanical pass, these contract repairs were applied:

- OpenCode Go declares `apiKeyField: "ANTHROPIC_API_KEY"`, matching the env key
  the kept preset already writes.
- The duplicate AtlasCloud card (`zai-org/glm-5.2`) was removed. The kept card
  remains `zai-org/glm-5.1` with context window 200000. The Codex chat test
  expectation follows that kept card, and the Volcengine preset name stays
  `火山Agentplan`.
- Codex model-catalog helpers that had been copied back into
  `codex_config.rs` were removed. `codex_config/catalog.rs` remains the owner,
  including `resolve_fyagent_catalog_path`.
- The facade copy of `ToolInstallationReport` and `probe_tool_installations`
  was removed. `services/tooling/discovery.rs` remains the owner. The copied
  planner called a subsystem shell helper that does not exist here.
- Diagnostic strings no longer interpolate `{account_id}`.
- At the original merge, `session_usage_mcode` was made `pub(crate)`; fix2
  removes its module declaration and startup scan entry for #208.
- The newly added nightly workflow under `.github/workflows/` was not kept.
  The change classifier has no class for that path, and CI modernization is
  outside this merge.
- Helpers whose names or tests contain the forbidden subsystem token were not
  kept. The platform scanner rejects that token, and the version probe called
  a distro-name checker that is not defined. The Windows `ReplaceFileW`
  fallback for `ERROR_NOT_SUPPORTED` was kept without naming a subsystem.
- Qianwen API-key URLs that contained the retired home-directory path token
  were pointed at `https://platform.qianwenai.com/`.

## Fix1 behavior status and remaining work

Evidence level: `code_audit` plus Python SQLite execution of source DDL;
no Rust build, test execution, native runtime, or installer acceptance.

| Group | Current code / fix1 | Remaining work (not implemented in fix1) |
| --- | --- | --- |
| S02 | REAL/NUL/non-UTF8 SQL formatting, sequence dump, staging auto-vacuum and incomplete-transaction rejection are present. Restored the missing FyAgent `validate_basic_state` definition. | Wire core-table validation **before** create/migrate; finish import protection and validate header-only/truncated/missing-core-table cases, including valid empty backups. |
| S03 | Restored `complete_backup`, locked backup wrappers, connection/protected-path/publish-hook parameters, test imports and import/restore hooks. Restore reuses its held lock. Existing temporary publish code now has its required inputs. | Validate restore candidate integrity/core tables; protect the selected source together with the safety snapshot; complete atomic backup/restore and validation-before-mutation behavior. Exercise corrupt/future DBs, publish failures/collisions, concurrency and retention=1 in Rust. |
| S04 | Shared sync mutex, Skills write locks and post-restore live/config/cache sync remain present. | Add `session_log_sync` and `session_usage_dedup` to both skip/preserve sets; recapture all local state under the final connection lock. Existing late-write tests remain pending and can expose these gaps. |
| S23 (with S22 / #210) | Byte observation is wired; fix1 supplies the missing schema-26 upgrade path. | Wire leading/trailing UUID acceptance into real validation; verify Windows same-mtime growth, unchanged partial-line skip and append retry. SQL migration evidence is not Rust/session behavior evidence. |

S20's four reviewed proxy fixes and S01's error-50 fallback are present in
reachable source; Rust/native validation remains pending. Other groups are
unverified, not implicitly completed by tag ancestry. Re-merging this same
tag cannot restore the discarded behavior.

Fix1 found source blockers outside the database boundary: undeclared Pi/Mcode
config modules, missing Skills helpers/fields, and missing Provider OAuth/preflight
helpers. Fix2 addresses those examples and the additional concrete defects below.
This supersedes the earlier open source-blocker list, but does not establish a
whole-project compilation pass.

## Fix1 validation (2026-10-07)

- Node 24.19.0: `pnpm typecheck` and `pnpm lint` passed.
- Focused session-migration, Rust-module-boundary, native-security-ordering,
  DEP0040, native-Fetch/MSW/Tauri and provider-promotion tests: **12 files,
  67 tests passed**. The separate version-consistency suite failed 3 tests
  with child-process `spawnSync ... node EPERM`; direct
  `node scripts/version.mjs check` passed (`0.4.10`).
- Python SQLite reproduced the old v26 missing-byte-column failure, then
  verified the source v27 DDL: old rows/flags preserved, NULL cursor defaults,
  dedup PK/index, idempotence, fresh/upgraded table shape parity and late-failure
  rollback. Five v27 Rust regression tests and the original v26 DDL fixture
  have been added but have **not** been executed by Rust.
- Static scan: no duplicate Database methods; no missing Database associated
  method definitions referenced by `backup.rs` / `schema.rs`. All 25 historical
  production migration/helper bodies and dispatch arms 0–25 match the pre-merge
  baseline. This is a bounded name scan, not type checking.
- Local `rustc` is 1.85.1; repository toolchain is 1.97.1; `rustfmt` is absent.
  No cargo build/check/test was attempted. Native validation and the additional
  source blockers listed above remain open. No stable-baseline acceptance.

## Fix2：底座编译接线修复（2026-10-07）

基线 `86ea5c74`；客户端接线参照父提交 `5b1a334b`，补函数来源为本地
`v3.20.4`（`43e1d990`）。不增加客户端模块、不补空桩、不改数据库 v27，
不执行 cargo、不更新 Cargo.lock、不修改 remote、不推送或创建 PR。

### 按能力记录回退

| 上游能力 | 本轮处置 | 后续归属 |
| --- | --- | --- |
| Pi 客户端枚举、可见性、目录设置、deeplink 与代理选择 | 恢复 FyAgent 父提交支持列表，移除可达的 Pi 模块引用 | **推迟到客户端适配包 #208** |
| Pi Provider 导入、启动扫描、live 读写及统一配置接线 | 移除依赖 `pi` / `pi_config` 的调用 | **推迟到客户端适配包 #208** |
| Pi Prompt/AGENTS 管理、激活状态推导和文件协调锁 | 移除命令、服务与启动入口；既有客户端 Prompt live 回填保留 | **推迟到客户端适配包 #208** |
| Pi Skills 部署哈希、归属判断、更新迁移、卸载保留路径与响应字段 | 回到已有目标的安装/卸载/备份流程；通用锁及路径校验保留 | **推迟到客户端适配包 #208** |
| Pi 会话发现、删除和用量同步 | 取消 provider 模块声明、scan/delete 与同步任务 | **推迟到客户端适配包 #208** |
| Mcode 枚举、可见性、Provider/config/deeplink 接线 | 恢复父提交支持列表，移除 `mcode_config` 调用 | **推迟到客户端适配包 #208** |
| Mcode MCP 导入、事务写入和启停 | 取消模块声明和服务分支；移除 3 个专属集成测试 | **推迟到客户端适配包 #208** |
| Mcode Prompt 写入协调 | 回退服务分支，撤回专属 `mcode_commands.rs` 测试入口 | **推迟到客户端适配包 #208** |
| Mcode Skills 分配、更新事务及数据库参数 | 删除未声明字段访问和无 SQL 占位符的参数；v27 `enabled_mcode` 列保留 | **推迟到客户端适配包 #208** |
| Mcode 会话读取与用量扫描 | 取消模块声明和调度入口 | **推迟到客户端适配包 #208** |

未注册的客户端源文件保留在仓库及上游历史中，不能据此认定功能可用；#208
接入时需一并恢复/适配上述测试。没有 ignore 已有客户端的测试。

### 已有能力补齐及必要适配

- Skills：补 `skill_state_lock/read_guard/write_guard`、`paths_alias`、
  `paths_overlap`、`ensure_distinct_skill_roots`、`get_distinct_app_skills_dir`、
  `validate_skill_storage_destination`；目录目标使用 FyAgent `SkillTargetId`。
  `resolve_uninstall_backup_source` / `create_uninstall_backup` 恢复父提交单参数
  调用链；Pi 专用 excluding/preserving 路径随客户端推迟，不伪造函数。
  补真实下载测试夹具字段，保留归档限制、网络后再检查、临时目录生命周期和路径回归。
- Codex：补上游 `CodexLiveWritePlan`、`plan_codex_live_write`、
  `preflight_codex_live_write`、managed token bundle 和 live-auth 构建辅助函数；
  补齐丢失的刷新重试、generation/持久化锁、登录提交参数及账号 workspace 查询。
  上游 `account_id`/可选 workspace 适配 FyAgent `credential_id`/String 模型，
  旧存储与测试初始化补 `id_token`、时间戳字段。并消除 facade 与 auth/storage 的重复定义。
- Provider：合并损坏的 update/switch 主接线恢复 FyAgent config-only/Managed Auth
  写入归属，保留上游纯预检、陈旧备份判断、统一 Provider 当前子项重投影及错误聚合。
  **上游 Provider 保存/切换直接管理 OAuth 登录文件的整套事务没有作为新入口启用**；
  这是既有认证架构的适配限制，不归为 Pi/Mcode 的 #208 客户端能力。
  实际代理/兼容调用需要的 OAuth helper 使用真实实现；回滚、CAS guard 保留。
- Proxy：补 `CodexStandaloneEndpoint` 及 full-URL 改写、原生 Responses URL 判断；
  补 5 个 SSE 测试辅助函数。修复 backup writer 参数数量、auth-guard writer 缺参数/局部变量。
- Tooling：补版本哨兵、npm dist-tags URL、GitHub 版本解析和旧 latest 过滤；
  保留 FyAgent semver、Hermes metadata owner、Windows shell-user 环境边界，
  修复 Windows command builder/runner 签名和测试调用。
- 自动同步：删除误粘回 facade 的另一套调度状态/Drop/计时函数，复用既有
  `AutoSyncController`；补 suppression 查询供上游回归测试读取真实状态。
- 测试与重复声明：删除重复 `auth_cancel_login` 及 invoke 注册、UniversalProvider
  facade 副本；修复残留 `cc_switch_lib` 引用及不匹配的结构体初始化。
  `database/backup.rs` **仅**将旧测试的 `CC_SWITCH_SQL_EXPORT_HEADER` 引用改成
  现有 `FYAGENT_SQL_EXPORT_HEADER`（blame 为上游 `dfb2e5235`，非 fix1 改动）；
  所有 fix1 实现、v27 schema/分派/夹具均未变。

### 检查与剩余边界

本轮最终 `pnpm typecheck`、`pnpm lint` 均以 exit 0 通过；相关 `pnpm test:unit`
通过 **30 个文件、248 个测试**。静态脚本覆盖 **455 个文件**，所有断言及
rustfmt 语法解析通过；`git diff --check` 通过。证据级别：`code_audit`。

最终检查结果见本地 `report-205-fix2.md`；可重复静态脚本为同目录的
`verify-205-fix2.py`。该脚本覆盖模块声明可达源文件与集成测试，检查客户端边界、
35 个补齐 helper 的唯一性、上游调用名、受保护文件一致性，并逐文件做 rustfmt
语法解析。名称扫描经过人工排除外部方法/闭包误报，不等于 Rust 名称解析或类型检查。
系统 PATH 的 `rustc` 为 1.85.1；额外发现的 1.97.1 工具链目录仅调用了独立
`rustfmt` 做无写入语法解析，没有运行 cargo。

本轮本地 `git add` 被实际只读挂载阻断（`index.lock: Read-only file system`），
因此未形成 fix2 提交；修改保留于工作树，补丁及恢复命令见本地报告。

仍需目标平台 Rust check/test、Cargo.lock/依赖一致性验证，以及上表 S02/S03/S04/S23
剩余行为闭环。保留测试可能继续暴露架构行为差异；本轮不作“整个 Rust 已编译通过”
或“#205 已验收”声明。

根因与防线：机械合并同时保留调用端和 FyAgent 拆分后的模块结构，漏掉定义、字段、
初始化及集成测试入口。以后按生产入口、字段/参数、测试入口三层一起核对；不得只
扫描 `src`、只证明标签 ancestry，或通过增加空桩和禁用测试掩盖冲突。

## Original merge validation (historical, not a fresh fix1 run)

`pnpm typecheck` and `pnpm lint` passed. `pnpm test:unit` reported 207 files
passed and 6 failed (213 total); 2066 tests passed, 27 failed, and 3 skipped
(2096 total). The failing files are the known host-integration and governance
set: `miseTaskContract`, `windowsMsvcCross`, `developmentEnvironment`,
`systemCheck`, `taskDocs`, and `repositoryGovernanceScan`. The router-shell
load timeout did not fail on this run. Rust was not compiled on this machine.
`Cargo.lock` was not regenerated, so new upstream crates are not locked.

The long-term engineering contract is
[CC Switch Upstream Synchronization](../../.trellis/spec/backend/upstream-sync.md).
