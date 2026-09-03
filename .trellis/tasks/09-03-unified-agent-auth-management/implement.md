# 实施计划：统一 Agent 官方登录与账号管理

> 当前任务保持 `planning`。本文件是实施顺序与退出门禁，不代表已经批准启动。
> 原则：**先验证前端体验和合同，再接入后端；每个阶段可独立审查、回滚；没有HIL证据不开放production capability。**

## Execution contract

- 只在用户审核本任务并明确批准后执行 `task.py start`。
- 实施前从最新 `dev/laiyongjie` rebase并重新运行调研刷新；任务文档与事实不一致时先更新文档。
- 一个 `ManagedAuthService`、一个 metadata store、一个 SecretRef owner；禁止并行保留新的第二套OAuth/store。
- 前端页面只能通过 V2 FeaturePort；禁止为赶进度直接import V1 hooks/Tauri API。
- token、code、state、verifier、device code、SecretRef、绝对路径和raw OAuth error不得进入renderer、普通日志、测试snapshot或task evidence。
- 变更量过大时按下列Phase拆成可独立PR/子任务，但共享同一PRD、wire contract与migration owner，不各自发明协议。

## Phase 0 — Refresh evidence and freeze decisions

### Work

- [ ] 记录最新FyAgent commit、工作树、DB schema、现有Auth/Proxy/Agent所有者与测试基线。
- [ ] 刷新OpenAI Codex、xAI Grok Build、OpenCode、cockpit-tools exact commit、license、NOTICE和关键认证文件。
- [ ] 在`research/dependency-and-license-decision.md`完成依赖复用评审；默认使用现有crate，任何新增依赖需明确减少的安全敏感代码。
- [ ] 在macOS/Windows正式安装版本采集Codex credential-store、Grok auth_provider_command/registry、OpenCode data dir/reload行为。
- [ ] 冻结首期consumer capability matrix；未验证项标为disabled。
- [ ] 确认旧Auth Center、Provider forms、Proxy和V2 Agent Auth的迁移调用图。
- [ ] 将本任务拆分建议（若需要）写入task meta，但不改变统一owner。

### Exit gate

- 第一方协议、license和平台行为均有exact evidence；
- 浏览器PKCE端口/scope/client identity与当时OpenAI代码一致；
- Grok helper vs native owner、OpenCode official vs FyAgent projection有书面选择；
- 不存在依赖cockpit-tools代码的实施项。

## Phase 1 — Frontend contract and experience prototype

### Work

- [ ] 扩V2 navigation/primary page type，加入`/auth`和lazy/prefetch/persistent surface。
- [ ] 定义`managed-auth.ts`严格DTO、closed enum、exact-key parser、forbidden-field检查和`ManagedAuthPort`。
- [ ] 建立Mock/Browser FeaturePort数据集：无账号、单账号、多账号、需重登、部分Provider失败、迁移阻断、pending restart、第三方模式。
- [ ] 实现AuthPage shell、账号/软件连接tabs、master-detail、空/加载/错误状态。
- [ ] 实现账号卡片、详情、consumer connection card和account/connection/request-source三层文案。
- [ ] 实现ManagedAuthLoginDialog全部stage；browser与Device Code只使用safe display字段。
- [ ] 实现impact preview、remove/reconnect/switch/restart dialogs。
- [ ] AgentAuthStatusPanel改为摘要+deep link；保留现有Claude/国产handoff行为。
- [ ] 加入keyboard/focus/narrow viewport/reduced motion/persistent route browser tests。
- [ ] 审查前端文案，去除“projection/authority/credential”等内部词。

### Validation

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
```

### Exit gate

- 用户可在Mock backend完整走完添加、重新登录、连接、切换、移除和错误恢复；
- UX评审通过后才冻结wire v1；
- DOM/console/mock wire负向扫描无secret-like字段；
- V2页面未importV1/Tauri。

## Phase 2 — Managed-auth metadata and SecretRef production activation

### Work

- [ ] 扩`SecretPurpose`与typed secret bundle codec；不增加generic raw-bytes API。
- [ ] 注册`services::secret` production backend，完成macOS signed-app entitlement与Windows Credential Manager证据。
- [ ] 新增managed-auth metadata tables、DAO、schema migration、FK/index/backup/export策略。
- [ ] 实现ManagedIdentity/CredentialSession/Connection domain types与严格ID/enum。
- [ ] 实现SecretRef create/replace/probe/delete recovery journal与generation一致性。
- [ ] 建立`ManagedAuthService` facade和Tauri state，但暂不切Proxy流量。
- [ ] 加入secret zeroization、Debug redaction、DTO forbidden-field和DB no-token tests。

### Validation

```bash
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml secret_service_contract -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml managed_auth -- --nocapture
mise run check:contracts
```

### Exit gate

- OS vault是唯一新token write target；
- DB/backup/export不含token；
- crash/partial write有deterministic recovery；
- SecretRef不可用时fail-closed且旧文件未破坏。

## Phase 3 — Legacy store migration and compatibility facade

### Work

- [ ] 实现`codex_oauth_auth.json`与`xai_oauth_auth.json`严格解析、source hash和versioned migration journal。
- [ ] 逐账号创建SecretRef并写identity/session/default mapping。
- [ ] 迁移Provider`authBinding`到新credential ID，保持legacy ID映射与悬空检测。
- [ ] 全部成功后rename bounded backup；失败不删除源、不写新登录。
- [ ] 将旧`commands/auth.rs`通过ManagedAuthService compatibility adapter提供，保持现有UI/Proxy暂时可用。
- [ ] 实现migration status/repair/retry DTO与V2 UI。
- [ ] 加入中断点、重复启动、部分SecretRef、DB rollback、旧binding和未来version测试。

### Exit gate

- 迁移幂等；
- old manager不再直接写明文store；
- 所有现有账号/Provider binding可解析或显示明确blocked原因；
- rollback到旧版本的文件保留策略已验证。

## Phase 4 — OpenAI managed login (browser first, device fallback)

### Work

- [ ] 从现有Codex manager抽取OpenAI Device Code/token refresh/identity逻辑到provider adapter。
- [ ] 实现127.0.0.1 loopback PKCE server、1455/1457、one-shot callback、bounded request、state/generation校验和cancel。
- [ ] backend直接打开系统浏览器；授权URL只在短生命周期内存对象中存在。
- [ ] 端口不可用/用户选择时转Device Code；session snapshot不暴露device_auth_id。
- [ ] 实现backend-owned login session store、active session恢复、route hidden行为和restart cancellation。
- [ ] token exchange后写SecretRef、identity merge、purpose-scopedCredentialSession。
- [ ] 实现refresh coordinator、singleflight、rotated token、generation CAS、reauth classification。
- [ ] 将V2 LoginDialog接入真实ManagedAuthPort。

### Tests

- [ ] fake issuer完整PKCE参数和verifier。
- [ ] wrong/missing/duplicate/late callback、state mismatch、wrong path/method/host、oversized request。
- [ ] 1455占用→1457；两者占用→Device Code；不cancel未知进程。
- [ ] browser open failure、cancel race、timeout、app restart。
- [ ] Device Code pending/deny/expire与poll interval。
- [ ] same identity不同purpose生成独立session。

### Exit gate

- OpenAI账号可在V2完成browser与device登录；
- SecretRef/readback后才显示成功；
- 没有第二个OpenAI store/refresh loop。

## Phase 5 — Codex native connection and official/third-party invariant

### Work

- [ ] 实现Codex consumer observation：account/read优先或受支持native store adapter。
- [ ] 完成file/keyring/auto/ephemeral capability matrix与HIL gate。
- [ ] 实现专用Codex CredentialSession连接、refresh owner转移/回收、native generation同步。
- [ ] 复用现有Codex process/instance owner做停止、重启和readback。
- [ ] 把第三方Provider写入改为config-only；所有路径禁止修改官方credential store。
- [ ] 删除V2/V1中的preserve toggle，兼容读取旧setting但强制preserve；安排字段移除migration。
- [ ] 实现官方→第三方→官方事务与impact preview。
- [ ] Proxy要求独立`proxy_upstream` session；不能复用Codex-native session。
- [ ] Agent Codex Auth observation改为ManagedAuthService事实，不再只返回跳转占位。

### Tests/HIL

- [ ] 每个Codex store mode的支持/拒绝。
- [ ] official→DeepSeek/Kimi/other→official，登录与token generation不变或正确同步。
- [ ] 当前第三方配置在官方session失效/重登失败时不被破坏。
- [ ] Codex运行中刷新、外部账号替换、切号、restart失败、rollback。
- [ ] 全仓扫描第三方writer不能delete/overwrite官方auth。

### Exit gate

- 有效官方账号切回无需重新登录；
- 第三方切换不能破坏官方session；
- native状态与V2摘要一致。

## Phase 6 — xAI managed login and Grok Build connection

### Work

- [ ] 将现有XaiOAuthManager抽取到ManagedAuth provider adapter和SecretRef。
- [ ] backend-owned xAI Device Code session；补cancel/slow_down/restart状态。
- [ ] 实现Grok consumer observation与current account binding。
- [ ] 优先完成auth_provider_command helper spike、closed protocol、packaging、identity与HIL。
- [ ] helper通过则FyAgent保持refresh owner；实现token response、timeout、stderr redaction和hot reload。
- [ ] helper不通过则按design实现独立native session、registry merge、auth.json.lock、Grok-owned refresh与reconcile。
- [ ] 删除Grok `HandoffOnly`生产语义；未支持环境返回closed unsupported/manual state。
- [ ] 接入V2账号/connection/Agent摘要与quota独立状态。

### Tests/HIL

- [ ] malicious discovery endpoint、non-x.ai host、HTTP、oversized JSON。
- [ ] Device Code pending/slow_down/deny/expire/transport retry。
- [ ] refresh rotation、external registry change、unknown scope preservation、lock contention。
- [ ] Grok CLI absent/multiple home/version unsupported/helper failure/native fallback。
- [ ] macOS/Windows真实登录与自动续期。

### Exit gate

- Grok不再只能handoff；
- refresh owner唯一且可观察；
- official registry未知字段不丢失。

## Phase 7 — OpenCode Desktop provider management

### Work

- [ ] 以OpenCode official `Global.Path.data`和当前target user上下文解析credential store；不依赖系统PATH CLI。
- [ ] 实现provider connection observer：OAuth/API/wellknown metadata，DTO不返回secret。
- [ ] 实现“由OpenCode管理”官方Provider Connect handoff/公开API路径；无稳定deep link时只launch并给出步骤，不扫描sidecar。
- [ ] 实现FyAgent-managed独立session投影：schema validation、read-modify-write、unknown provider保留、0600、atomic rename、revision/CAS。
- [ ] refresh owner转给OpenCode，external writeback/reconcile；禁止复制Codex/Proxy lineage。
- [ ] 通过HIL决定hot reload或受控restart；pending restart状态接入UI。
- [ ] 移除OpenCode Desktop主Auth对`opencode auth *`CLI的依赖；CLI能力只作为独立可选surface，不代表Desktop状态。
- [ ] 接入provider disconnect、target selection、impact preview和readback。

### Tests/HIL

- [ ] no CLI + Desktop installed。
- [ ] empty/configured/mixed provider store、invalid entry、unknown keys/provider。
- [ ] concurrent/external file change、permission、disk full、restart cancel/failure。
- [ ] OpenAI/xAI dedicated session与owner reconciliation。
- [ ] macOS/Windows current stable Desktop真实连接与断开。

### Exit gate

- OpenCode Desktop在无CLI环境可完整管理Provider状态；
- connection success由store/readback证明；
- 不访问私有sidecar密码。

## Phase 8 — Proxy, Provider forms and legacy Auth Center convergence

### Work

- [ ] `proxy/forwarder`改用ManagedAuthTokenResolver；只有FyAgent-owned proxy session可refresh。
- [ ] Provider `authBinding`使用新credential/connection ID并保留migration compatibility。
- [ ] V1 Provider forms通过compatibility adapter复用ManagedAccountPicker/同一backend DTO，不再读具体manager。
- [ ] 迁移GitHub Copilot账号UI到V2页面；backend协议保持现有owner，必要secret迁移单独审查。
- [ ] Settings Auth Center变为V2入口/兼容shell；删除旧page-ownedpolling和重复mutation。
- [ ] 删除`src/lib/api/auth.ts`旧DTO/命令中已无consumer的路径，或保留versionedcompatibility直到下个版本。
- [ ] 检查所有账号入口、空状态、deep link与返回路径只有一个主要owner。

### Exit gate

- V2/Provider/Proxy/Agent显示同一账号事实；
- 没有两套默认账号或两套refresh loop；
- legacy UI行为有迁移/删除测试。

## Phase 9 — Failure recovery, security and migration hardening

### Work

- [ ] fault injection覆盖SecretRef、DB、native store、restart、network与external writer各提交点。
- [ ] 实现repair overview：missing secret、orphan secret ref、dangling connection、native mismatch、migration journal。
- [ ] account remove影响预览与fresh revision；多连接补偿。
- [ ] 日志/telemetry sanitizer加入OAuth query/token-like字段；测试所有Error/Debug。
- [ ] DB export/import明确opaque SecretRef不可迁移，跨设备显示需重新登录。
- [ ] 完成macOS signed-app entitlement验证、Windows Credential Manager installer/user-context HIL。
- [ ] 完成license/NOTICE/source manifest与依赖advisory检查。

### Exit gate

- crash/restart不产生旧token覆盖或静默假成功；
- secrets不出现在持久普通文件、DB导出、renderer、日志和测试工件；
- 修复状态可操作且不自动破坏原生登录。

## Phase 10 — Full validation and native HIL

### Automated gates

按仓库锁定命令实际存在情况运行并记录：

```bash
pnpm install --frozen-lockfile
pnpm typecheck
pnpm typecheck:v2
pnpm lint:v2
pnpm test:unit
pnpm test:v2
pnpm test:v2:browser
pnpm test:i18n
pnpm build:renderer
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo check --workspace --all-targets --keep-going --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --workspace --all-targets --keep-going --locked --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --workspace --features fyagent/test-hooks --locked --manifest-path src-tauri/Cargo.toml --no-fail-fast
mise run check:contracts
```

- [ ] native Windows/macOS CI jobs由现有change classifier触发；不建平行workflow绕过失败。
- [ ] architecture tests：V2无V1/Tauri import、Proxy无concrete OAuth manager、secret fields不在DTO。
- [ ] full repository scan：token-like fixture仅在明确test secret中，日志/Task无真实credential。

### Native HIL matrix

- [ ] macOS signed build + Windows formal install build。
- [ ] OpenAI browser/device、port conflict、cancel、timeout、network failure。
- [ ] Codex all observed store modes、running refresh、third-party round trip。
- [ ] xAI/Grok login/refresh/helper或registry/external change。
- [ ] OpenCode Desktop no-CLI/provider connect/restart/concurrent change。
- [ ] OS vault locked/denied/unavailable。
- [ ] migration from current production JSON stores。
- [ ] multiple accounts/consumers/targets and delete impact。
- [ ] app crash at each transaction phase。
- [ ] China mainland network failure produces bounded actionable copy; no unauthorized bypass。

### Hard gate

任何consumer没有正式平台HIL，其production action保持disabled；任务不得以“代码已完成”声明该能力可用。

## Phase 11 — Review, spec convergence and archive

### Review 1 — Product/UX

- [ ] 账号、连接、请求来源不会混淆。
- [ ] 主要流程无需理解OAuth/token/store。
- [ ] 每个失败状态有安全且可执行的下一步。
- [ ] keyboard/responsive/hidden route/return体验通过。

### Review 2 — Architecture/reuse

- [ ] 一个ManagedAuthService、一个metadata store、一个SecretRef owner。
- [ ] 现有Codex/xAI/Agent/Proxy owner被迁移，不是复制。
- [ ] shared UI/DTO只在真实复用处抽取。
- [ ] OSS使用有exact source/license/NOTICE，cockpit-tools无代码复制。

### Review 3 — Adversarial security

- [ ] 恶意renderer、callback、外部file writer、并发refresh、old generation、locked vault不能取得/覆盖secret。
- [ ] 一个refresh lineage只有一个owner。
- [ ] third-party switch全路径不能破坏official auth。
- [ ] no raw URL/token/path/log leakage。

### Review 4 — Operations/migration

- [ ] 升级、降级、旧backup、SecretRef丢失、native mismatch、rollback和repair可维护。
- [ ] DB/OS vault/native store版本兼容和future-version fail-closed。
- [ ] 真实HIL证据与capability flag一致。

### Closeout

- [ ] 根据最终代码更新`design.md`/research与相关Trellis specs。
- [ ] 运行task docs/context/prearchive检查和完整diff review。
- [ ] 不提交token、账号、callback URL、HIL用户路径、系统vault导出或临时auth文件。
- [ ] 只有Definition of Done全部满足才归档；缺少某consumer HIL时可拆出明确blocked follow-up并保持该capability disabled。

## Rollback plan

1. 关闭对应consumer capability，不删除ManagedIdentity/SecretRef。
2. 保留旧JSON migration backup，只读用于人工恢复；新版本不继续写明文。
3. Proxy可回滚到最后一个已验证的FyAgent-ownedsession resolver，不借用native-ownedtoken。
4. Codex第三方Provider保持config-only，不恢复会破坏官方auth的旧行为。
5. V2 Auth Page可暂时只读展示；旧Settings compatibility入口保持，但不能重新成为独立token owner。
6. native projection不确定时恢复exact preimage或标记recovery_required，不用猜测文件内容修复。
