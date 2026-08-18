# Issue #35 详细设计独立静态评审

## 结论

`DETAILED_DESIGN_REVIEW=REQUEST_CHANGES`

当前设计不能进入 `DESIGN_FREEZE`。开放计数：`P0=0`、`P1=10`、`P2=5`、`P3=0`。只有下列每个 P1/P2 都完成设计修订、静态回读并降为 0 后，才能改为 `DETAILED_DESIGN_REVIEW=PASS`。

## 评审信封

- 评审日期：2026-08-14 Asia/Shanghai。
- 专用 worktree：`/Users/serendipity/.codex/worktrees/issue-35-secret-backend`。
- 精确代码基线：`afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab`。
- 设计快照 SHA-256：`prd.md=66407226…facae0`、`technical-design-overview.md=bb1ccf1f…d7fd8`、`detailed-design-overview.md=12dbc9a7…a464d`、`execution-plan.md=18143853…f7d7b`、`research/secret-surface-inventory.md=e3b23c5a…fef60`。
- 证据级别：`source_report + code_audit + static_design_review`；不是 build/test/runtime/native/UAT 证据。
- 严格遵守设计阶段边界：本评审运行了零测试、零构建、零 dependency resolution、零 browser/renderer/server/native runtime、零截图；没有修改生产代码、依赖、任务设计或其他 reviewer 文件。

## Findings

### DD-001 — P1 — Codex secret 调用面与 file-owner map 严重漏项

- 证据：`research/secret-surface-inventory.md` §“Codex first-slice surfaces”与 `detailed-design-overview.md` §1 未列出现有代理真实流量链。当前 `src-tauri/src/proxy/providers/codex.rs::extract_key/extract_auth` 从 `Provider.settings_config` 复制 key 到 `AuthInfo(String)`；调用点位于 `src-tauri/src/proxy/providers/adapter.rs`、`src-tauri/src/proxy/providers/mod.rs`、`src-tauri/src/proxy/forwarder.rs`。`src-tauri/src/services/proxy.rs` 还会在 takeover/live-backup 流程复制 Codex key。
- 其他精确漏项：`src-tauri/src/commands/misc.rs::open_provider_terminal`、`commands/model_fetch.rs::fetch_models_for_config`、`commands/balance.rs::get_balance`、`commands/failover.rs::get_available_providers_for_failover`、`deeplink/provider.rs::import_provider_from_deeplink`、`commands/sync_support.rs`，以及相关测试文件均不在任何生产 owner 的精确路径清单中。
- 影响：若按现设计 scrub `settings_config`，代理转发等现有路径会失去认证；若保留现状，则 Codex key 继续被复制为普通 `String`、进入 backup/terminal/IPC，直接违反 PRD §2/§4。并行 worker 也没有合法文件所有者来闭环这些路径。
- 必须关闭：先补全一份从所有 Codex 入口到真实 sink 的静态 call graph；明确 proxy request/terminal/model-fetch/balance/deep-link/import/restore 的 consumer 与 controlled-resolve 方案；把每个生产及测试文件放入唯一 owner；为 proxy 高频路径冻结是否支持硬件、确认/lease 生命周期和 no-fallback 行为。不能以“main thread 处理 shared integration”代替精确文件 ownership。

### DD-002 — P1 — “Exact public contract v1”目前既不完整也无法表示合法状态

- 证据：`technical-design-overview.md` §2 的 `SecretSummary` 强制要求 `secretRef/backend/capabilities/createdAt`，但 `detailed-design-overview.md` §5 的最高优先级状态是“legacy value/conflict without binding -> migrationRequired”；此时不存在 ref、record、backend 或 createdAt。
- 证据：PRD §5/§6.3 和 repository API允许一个 ref 被多个 owner 共享并整体轮换，但 `SecretSummary` 只有单个 `owner`，而 `rotate_secret`、`set_secret_locked`、`delete_secret`、readiness 都按 ref 返回单个 `SecretSummary`，返回语义不确定。
- 证据：`SecretDeleteImpact`、`SecretMigrationReport`、`SecretAuditPage`、summary filter/cursor、所有 command request DTO 均未定义；命令没有冻结 `Result<Ok, SecretCommandError>` wire envelope。Tauri 要求成功值与错误都实现 `Serialize`，当前设计无法保证错误只含 stable code/action。
- 证据：PRD §3.1 说 Agent owner 本轮可创建/查询/轮换，技术设计 §2 却说 MVP 只接受 `provider/codex`，Agent namespace/ownerId 验证和返回行为互相冲突。
- 影响：Rust/TS 无法从文档实现同一套可编译、可穷举、无伪造 ref 的合同；migrationRequired、多 owner、command error 会被实现者自行猜测。
- 必须关闭：给出所有 Rust/TS request/response/error 的完整镜像定义与 serde 属性；为无 binding 的 legacy 状态设计独立 owner-level summary/union；按 ref 的 mutation 返回确定的 aggregate/owner list；冻结 Agent owner 本轮边界；逐命令写出参数、成功值、错误 envelope 和 invariants。

### DD-003 — P1 — backend/service API 与读回验证、一次性 capability 流程自相矛盾

- 证据：`technical-design-overview.md` §3 的 `SecretBackend` 只有 `write/probe/prepare/confirm/delete`，没有 read/resolve；同文件 §8 capture step 3 和 `detailed-design-overview.md` §7 却要求 backend write 后“resolve/read-verify + constant-time compare”。该算法无法调用冻结的 trait。
- 证据：`ResolveContext` 仍携带未定义的 `HardwareConfirmationReceipt`，但 `confirm` 只接收公开 DTO `HardwareConfirmStep`；pending-step 的 native registry、TTL、原子 consume、防重放、取消/超时清理均未定义。
- 证据：`SecretService::resolve_for_apply` / `with_resolved_secret` 没有精确 Rust signature。`SecretMaterial::expose_to<T>` 的任意泛型返回值允许 closure 直接返回 `Vec<u8>/String`，不能支撑“material 不能从 closure 逃逸”的设计断言；现有 async usage 和遗漏的 proxy request 也没有可编译的 await/borrow/shortest-copy 方案。
- 影响：新建和迁移不能验证写入；#41 handoff 的 one-shot 语义不能由类型/状态机保证；实现可能把材料跨 await 或返回到普通对象中。
- 必须关闭：冻结可验证写入所需的 exact backend read/verify API；删除过时类型或补全 confirmation registry/receipt 状态机；给出 taking-by-value、不可 Clone、原子 consume 的 capability API，以及 async/sync writer 的允许输出类型；增加 replay/expiry/cancel 与 closure-escape 的 compile/static tests。硬件 write/delete 是否也需确认必须显式裁决。

### DD-004 — P1 — 双平台 native capture/store 方案尚未达到 API 可实现性门槛

- 证据：`detailed-design-overview.md` §3 Windows 同时使用 `CREDUI_FLAGS_GENERIC_CREDENTIALS/ALWAYS_SHOW_UI`（`CredUIPromptForCredentialsW` 族）和“unpacked buffers”（`CredUIPromptForWindowsCredentialsW` + `CredUnPackAuthenticationBuffer` 族），两套 API/flag 体系混在一起；列出的精确 flags 也缺少阻止 Credential UI 自行保存所需的 `CREDUI_FLAGS_DO_NOT_PERSIST`。Microsoft 文档明确说明不设置 PERSIST/DO_NOT_PERSIST 时 Save checkbox 会参与保存行为：[CredUIPromptForCredentialsW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-creduipromptforcredentialsw)。
- 证据：`windows-native-keyring-store 1.1.0` 默认 persistence 是 `Enterprise`；只有 entry modifiers 显式传 `target=FyAgent/secret/v1/<ref>` 与 `persistence=Local` 才满足设计表。所选 crate 还明确警告同一 entry 跨线程顺序不可靠：[crate docs](https://docs.rs/windows-native-keyring-store/1.1.0/windows_native_keyring_store/)。当前文档没有冻结 build/modifier 调用形状与 mutex 覆盖范围。
- 证据：当前 `src-tauri/Cargo.toml` 的 `objc2-app-kit` 仅启用 `NSColor`；本地锁定的 0.2.2 API 要实例化 `NSSecureTextField + NSAlert` 至少还需 `NSSecureTextField/NSTextField/NSControl/NSResponder/NSView/NSAlert/NSButton/NSApplication` 等 feature 组合。Windows direct CredUI 也需要相应 `windows` feature。Tauri `run_on_main_thread` 只调度 `FnOnce() -> ()`，设计未定义 oneshot 回传、shutdown 取消与 async runtime 不阻塞的调用形状。
- 证据：`research/os-keyring-options.md` §Error normalization 未覆盖 `keyring_core::Error` 的 `BadEncoding(Vec<u8>)`、`BadDataFormat(Vec<u8>, ...)`、`Ambiguous`、`NoDefaultStore`、`NotSupportedByStore`；前两者可携带原始 bytes，不能按普通 source/debug 丢弃。
- 影响：可能产生第二份由 Windows UI 保存的凭据、意外 Enterprise roaming、已知 feature 编译失败、async runtime 阻塞，或通过 error/debug 泄漏材料。
- 必须关闭：Windows 二选一冻结函数族、完整 flags、parent HWND、buffer 大小/UTF-16/zeroize/free 与取消映射；冻结 direct-store exact modifiers；列出 macOS/Windows Cargo features 和 Tauri main-thread/oneshot sequence；给出每个平台原始 error code/所有 keyring error variants 到 stable error 的穷举表，携带 bytes 的错误必须先 zeroize 且绝不进入 Debug/source。

### DD-005 — P2 — 候选依赖与仓库声明 MSRV 不一致

- 证据：当前 `src-tauri/Cargo.toml` 声明 `rust-version = "1.85.0"`，`research/os-keyring-options.md` 选择 `windows-native-keyring-store = 1.1.0`；该版本 crate manifest 声明 Rust 1.88：[crate manifest](https://docs.rs/crate/windows-native-keyring-store/1.1.0/source/Cargo.toml)。仓库锁定 toolchain 是 `rust-toolchain.toml` 的 1.97.1，所以当前 host 可构建不等于 1.85 MSRV 仍真实。
- 影响：依赖加入后，仓库公开 manifest 的最低 Rust 版本承诺失真；1.85–1.87 会在 dependency compile 前失败。
- 必须关闭：在设计中明确选择兼容版本或授权提升 `rust-version`，并列出由此涉及的 manifest/toolchain/docs/CI 校验文件；dependency resolution 后记录 exact lock version、license/advisory/MSRV 结果再进入实现，不得只以当前 1.97.1 host 通过代替。

### DD-006 — P1 — public projection 与验收 scope 尚未闭合，无法 fail closed

- 证据：`detailed-design-overview.md` §6 只规定删除 `/auth/OPENAI_API_KEY` 和 TOML token，但当前 `Provider.meta.usageScript` 仍可含 `apiKey/accessToken`，`commands/provider.rs::read_live_provider_settings` 返回原始 live JSON，UniversalProvider 公开 `apiKey`，failover command 返回完整 `Provider`，proxy `AuthInfo` 持有 key。
- 证据：当前 `codex_config.rs::remove_codex_experimental_bearer_token_if` 只移除 active provider table 和 top-level token；inactive `[model_providers.*]` 中的 token 仍可留在 public TOML。迁移的“两处值”也没有定义多个 provider table 中不同 token 的 conflict/scrub 规则。
- 证据：内部 `Provider` 继续 derive `Serialize`，因此“每个 command 自觉调用 projection”不是可强制边界；`research/secret-surface-inventory.md` §Required design adjudication 仍保留两种选择，PRD/技术/执行文件没有正式选定 `codex_feature_runtime` 或 repo-global 范围。
- 影响：任一漏掉 projection 的 command/event/export 即可重新把 canary 送到 renderer；scanner 可能把 Codex-only 结果误报为全仓通过。
- 必须关闭：正式裁决 feature-scope；定义无 secret-bearing 字段的 `ProviderPublicDto` 并优先用类型/编译边界阻止 internal `Provider` 直接作为 IPC 输出；列全所有 provider-returning/raw-live/universal/failover/export/diagnostic 路径；对 Codex 阻断共享 legacy value APIs；遍历并清除所有 TOML token 位置；给每个 public surface 独立 canary 测试。

### DD-007 — P1 — startup/import/restore 与 artifact cleanup 顺序未冻结

- 证据：`detailed-design-overview.md` §7 只说“DB construction 后、app.manage 前迁移”。当前 `src-tauri/src/lib.rs` 在 `AppState::new` 后、`app.manage` 前先执行 live Provider import、seed、多类 import，并在 manage 前启动 WebDAV/S3 workers。空库必须先 import Codex live value 才有东西可迁移；仅写“before app.manage”不能决定安全插入点。
- 证据：`commands/import_export.rs::import_config_from_file/restore_db_backup/sync_current_providers_live` 与 `commands/sync_support.rs::run_post_import_sync` 可在运行期导入 legacy plaintext 后立即构造 production `AppState` 并写 live；设计只覆盖 startup migration。
- 证据：`Database::backup_database_file`、SQL export、WebDAV/S3 sync 都可在 migration pending 时复制 raw `providers.settings_config`。所谓“现有 FyAgent exports/backups”没有权威目录、文件类型、结构版本、损坏策略、future-write gate 或 ownership 清单。
- 影响：第一次启动、恢复备份、同步下载或 locked/denied migration 后，可在 scrub 前回填 live 或生成新的明文 backup/sync 副本，最终 scan 也无法解释哪些 artifact 被检查。
- 必须关闭：给出 startup 的 exact sequence（live import → secret migration/reconcile → 其余 consumer/worker → manage）；给 import/restore/sync/download 单独的 migrate-before-live sequence；migration pending 时 fail closed 阻断或结构化 scrub export/backup/WebDAV/S3；枚举所有受管 artifact 及原子替换/VACUUM/checkpoint/失败状态，并把所需文件加入 owner map。

### DD-008 — P1 — 外部 keyring side effect 缺少 crash-safe write-ahead reconciliation

- 证据：`technical-design-overview.md` §8 仅在 DB insert 失败且 compensation delete 又失败后写 recovery marker。若进程在 backend write 成功后、DB insert/marker 前崩溃，OS store 会留下无 record、无 binding、无 marker 的 credential。
- 证据：rotation/migration 同样存在 write 后、transaction 前的 crash window；old record 标为 stale 后也没有冻结 startup retry/abandon 操作。Windows crate禁用 default `search` feature，不能依赖事后枚举找回随机 ref。
- 影响：产生不可定位、不可审计、不可自动清理的真实 secret 副本，与“一份受管引用、可解释生命周期”目标冲突。
- 必须关闭：在任何 backend write 前原子写入只含 ref/operation/phase 的 write-ahead recovery journal，并规定 fsync/commit/clear 顺序；startup 先 reconcile journal/stale records 再开放 consumer；列出每个 crash point 的确定恢复动作、幂等性和 fault-injection 测试。marker 位置、schema、权限、retention 与 owner 必须精确。

### DD-009 — P1 — schema、lifecycle transition 与 destructive CAS 存在静态正确性缺口

- 证据：`technical-design-overview.md` §6 的 `CHECK (secret_ref GLOB 'sec_[0-9a-f]*' AND length(secret_ref)=36)` 只约束 `sec_` 后第一个字符为 hex；SQLite glob 中后续 `*` 可匹配任意字符，因此长度 36 的非 hex ref 可入库。
- 证据：单列 `lifecycle_state` 同时承载 active/locked/stale/revoked，却没有合法 transition table。锁定 stale/revoked 后再 unlock 若简单回到 active 会复活 stale/revoked；rotation 文字中的 `stale-pending-delete` 又不在 schema enum 中。
- 证据：delete 只 CAS `expectedDependencyCount`；owner A 离开、owner B 加入而 count 不变时，用户确认的 impact set 已变化但 delete 仍会通过。repository 的 `set_lock(expected_updated_at)` 也没有对应 IPC 参数；timestamp CAS 是否单调未定义。
- 证据：现有 `Database::init` 先 `create_tables_on_conn` 再 migration；`detailed-design-overview.md` 只说“tests start at v16”，未冻结不会先创建 v17 表的 exact v16 fixture/upgrade path。
- 影响：数据库可接受合同外 ref、逻辑 unlock 可破坏安全状态、destructive confirmation 可作用于未确认 owner，migration test 可能只验证 fresh schema 而未验证 v16→v17。
- 必须关闭：改用 `substr(secret_ref,5) NOT GLOB '*[^0-9a-f]*'` 等能约束全部 32 字符的 CHECK；冻结完整 transition/precondition table；为 binding set 使用单调 revision或 exact-set digest/CAS 并贯穿 DTO；统一 lock/delete/rotate 参数；定义手工 v16 fixture、fresh DB、idempotent retry、future-version block 四条独立 schema tests。

### DD-010 — P1 — AppState/test injection 与线程执行模型不足以防真实 keyring 误触

- 证据：当前 `src-tauri/src/store.rs::AppState::new(db)` 被 production、provider/proxy/deeplink/import tests 和 `commands/sync_support.rs` 多处调用。设计只增加 `new_with_secret_service(db, service)`，没有冻结 `Arc<dyn ...>` 字段、capture/clock/id generator/failure injector，也允许传入绑定到另一 `Database` 的 service，形成 state 内两套 DB。
- 证据：native capture 需要 `AppHandle`/main thread，但 production constructor 仍只有 DB；平台 store API 同步阻塞，而 Tauri commands/usage/proxy 是 async，设计没有规定 `spawn_blocking`、service mutex 与 closure/await 的锁范围。
- 证据：普通 `mise run rust:test` 会跑大量 `AppState::new` tests；没有 cfg/env/ignored gate 保证它们绝不接触真实 Keychain/CredMan，也没有列出所有需要迁移到 fake backend 的调用点。
- 影响：测试可能读取/写入开发者真实凭据、CI native store，或因 DB/service 不一致产生假通过；UI 主线程/async runtime 也可能阻塞或死锁。
- 必须关闭：冻结 AppState exact fields 与 constructors，使 service 必须从同一 DB + injected backend/capture/clock/id source 构造；列出并改造所有 AppState call sites；ordinary unit/integration 默认只能用 in-memory backend，真实 store tests 必须显式 env/ignored native gate；画出 capture、store I/O、service mutex、DB transaction 与 async callback 的线程/锁顺序。

### DD-011 — P2 — error/availability/audit 输入的全量语义未冻结

- 证据：`technical-design-overview.md` §5 共有 22 个 stable error code，但 matrix 只覆盖 8 个 family；write/read/verify/delete/input/migration/owner conflict/dependency change 的 presence、availability、retryable、action、audit outcome 未定义。
- 证据：`ApplyReadinessRequest.operationId`、`SecretOwner.ownerId/namespace` 是任意 public string，而 operation id 会进入 audit/log；没有 UUID/长度/字符集/服务端生成规则，调用者可把 secret-shaped 值塞进一个“允许字段”。
- 证据：`ApplyReadiness.error/hardwareConfirmStep` 与嵌套 `summary.error/hardwareConfirmStep` 可分别出现，未规定必须一致或谁权威；`SECRET_OWNER_CONFLICT` 同时被用作 owner collision 和 capture dialog busy。
- 影响：同一 native condition 可产生不同 UI/action/audit；所谓 no-value schema 可通过 unrestricted string value 绕过；frontend reducer无法穷举可靠状态。
- 必须关闭：对每个 error code给出 operation × presence × availability × retryable × action × audit outcome 的完整矩阵；operation/event/step/owner identifiers使用严格 newtype（优先服务端 UUID、长度和字符集限制）；拆分 busy 与 owner collision；规定 readiness 与 summary 单一权威/一致性校验并加 property/table tests。

### DD-012 — P2 — V2 Tauri adapter 路径已知违反现有 architecture gate

- 证据：`research/source-audit.md` §V2 facts 和 `tests/v2/app/architecture.test.ts` 明确只允许在 `src/v2/shared/platform/tauri/**` 直接 import `@tauri-apps/*`；`detailed-design-overview.md` §1/§8 却把唯一 direct invoke 放在 `src/v2/shared/data/credentials/tauri.ts`，且 owner map 不含任何新的 platform adapter 文件。
- 证据：执行计划要求四档 viewport/browser interaction，但 V2 owner 未列出任何 `tests/v2-browser/**` 精确文件。
- 影响：照文档实现会触发既有 architecture test；若临时改 platform 文件或 browser spec，又会越过唯一 owner 规则。
- 必须关闭：把 direct invoke 实现放到并纳入 ownership 的 `src/v2/shared/platform/tauri/credentials.ts`（data port只依赖该 adapter），或修改架构规则并重新评审；列出所有 viewport/browser spec 与 preview fixture 的精确路径和 owner。

### DD-013 — P1 — native/failure evidence 路径尚不可执行，尤其 Windows closure

- 证据：`runtime-preflight.md` 明确 Windows host 未 provision；`execution-plan.md` 要求至少三类 Windows failure path，却没有说明 missing/locked-or-denied/backend-unavailable/rotation-or-delete failure 哪些必须是真实 OS、哪些允许 injected，以及如何稳定诱发和清理。
- 证据：现有 `.github/workflows/ci.yml` 有 Windows backend runner，可做非交互 CRUD，但不能替代 user-visible native secure-capture UAT；普通 Rust test job也需要 gate 避免所有测试触碰 Credential Manager。
- 证据：执行计划把 branch push 放在 final review 之后，却要求此前在另一 matching Windows host 对 source-freeze SHA 取证；没有已授权的 commit 传输/checkout/CI dispatch 顺序。不存在可供 Windows host 获取未 push commit 的路径。
- 影响：即使代码完成，mandatory Windows native_runtime/三类 failure_path 仍无法生成可追溯 evidence；或被迫以 mock/CI/noninteractive 证据冒充 UAT。
- 必须关闭：冻结 Windows x64/ARM64 目标、获取 exact freeze SHA 的交付方式与必要的 pre-evidence push/CI 顺序；为 native CRUD、interactive capture UAT、真实 OS failure、injected failure 分别写 exact command/env/gate/cleanup/evidence class；指定至少三类 acceptance failure 的真实生成方式和 artifact manifest/readback。

### DD-014 — P2 — source-freeze gate 只有类别，没有 exact 可复现命令

- 证据：`execution-plan.md` Phase 4 明写“exact names confirmed from task manifests before execution”，随后仅列 `environment/system preflight` 等自然语言。当前实际 canonical tasks 是 `.mise/tasks/{core,frontend,rust,contracts}.toml` 中的 `env:check`、`system:check`、`lint:v2`、`typecheck(:v2)`、`format:check`、`rust:*`、`test:v2(:browser)`、`tasks:validate` 等；secret scanner/integration/native tasks 尚不存在。
- 证据：没有 source-freeze base/freeze range 的 exact ownership audit、`git diff --check` range、host metadata capture、exit/count schema，以及任一 source fix 后如何失效/重跑 evidence 的机器可检验命令。
- 影响：不同 worker 会执行不同 gate；“fresh pass on exact SHA”无法回放或审计。
- 必须关闭：在 execution plan 中逐条写出可复制执行的 `mise run ...`/native commands、filters 与预期 test count；先把新 task注册到 `package.json`/`.mise/tasks/contracts.toml` 并验证文档；冻结 `BASE_SHA...FREEZE_SHA` ownership/diff commands、evidence JSON schema、失败即停止和 invalidation/rerun 规则。

### DD-015 — P2 — scanner 的 pass/fail 语义与现有 fixture 基线冲突

- 证据：`technical-design-overview.md` §10 声称 scanner“rejects checked-in literal secret fixtures”，但当前 `src-tauri/src/codex_config.rs`、`proxy/providers/codex.rs`、`commands/provider.rs` 等已有大量 `sk-*`/token-shaped test literals；这些文件大部分也不在 scanner owner 的可改范围。
- 证据：`research/secret-surface-inventory.md` 后来定义 `repository_static_inventory` 只能 report、`codex_feature_runtime` 才可 pass，但该分级尚未同步到 PRD、详细测试设计和 Phase 4 的 exit criteria。
- 影响：scanner 要么从第一天永久红，要么通过宽泛 allowlist 隐藏新泄漏；最终报告也可能把 feature-scope canary 误标 repo-global。
- 必须关闭：冻结四个 scanner level 各自的输入、enumerated paths、allowlist ownership、exit code 与 evidence label；现有 literal baseline必须采用精确文件/AST理由而非全局 pattern waiver，并新增“同文件新 literal 必须失败”测试；PRD/执行/最终报告只能声明实际通过的 scope。

## 开放项计数

| Severity | Open |
| --- | ---: |
| P0 | 0 |
| P1 | 10 |
| P2 | 5 |
| P3 | 0 |

## Re-review gate

主线程统一修订后，详细 reviewer 必须重新读取同一 exact working tree 的 PRD、技术设计、详细设计、execution plan、全部 research、依赖 manifests 及上述现有调用链，并逐项记录 disposition。只要任一 P0/P1/P2 仍开放，结论保持：

`DETAILED_DESIGN_REVIEW=REQUEST_CHANGES`
