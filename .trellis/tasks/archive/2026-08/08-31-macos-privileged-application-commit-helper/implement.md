# Implementation Plan — macOS privileged known-application commit helper

## Execution contract

- 一个任务、一个共享 ABI（`research/implementation-seam.md`）。允许按 Swift helper / Rust port / 签名 verifier 并行分包，禁止各写一套协议。
- 前置 lifecycle 任务已归档；只读当前树中的 helper-facing 合同。不要实现后续 `08-31-macos-agent-directory-install-policy` 的产品面。
- 每个分包先 focused test，再进入集成。
- `/Applications` production action 在 formal signed/notarized HIL 通过前始终 disabled。
- 发现设计与最新代码不一致时，先回到 PRD/design 更新；不在代码中通过临时 fallback 绕过。

## Phase 0 — Rebase evidence and freeze the seam

### Work

- [ ] 记录当前 commit、工作树和现有 macOS lifecycle task最终状态。
- [ ] 重新读取当前 `agent_install`、Codex DMG transaction、application identity、inventory target、job/progress、release signing与Tauri bundle代码。
- [ ] 列出当前用户态流程已负责与 helper必须负责的步骤，确认不存在两个 transaction owner。
- [ ] 确认首期产品/target-slot列表及其统一策略来源。
- [ ] 确认系统目标仍为 disabled，没有其他分支先行加入 sudo/AppleScript/helper。
- [ ] 更新本任务 research中的过期文件路径、版本、Apple API和依赖信息。

### Exit gate

- helper seam 是 `MacSystemCommitPort`，renderer wire不变；
- 已有任务的代码没有被覆盖；
- 产品策略与事务抽取方案有一份可评审 ADR。

## Phase 1 — Dependency and build spike

### Work

- [ ] 刷新 Blessed、SecureXPC、Authorized及transitive package的官方repo、tags、commits、license、advisories和Swift/Xcode兼容性。
- [ ] 根据 `design.md` pin exact versions/revisions并提交`Package.resolved`。
- [ ] 建立最小 Swift package：protocol、client bridge、helper executable、tests。
- [ ] 证明 helper/client可分别构建 arm64/x86_64并产出universal artifacts。
- [ ] 证明 Rust/Tauri可以调用主进程内C ABI bridge；不创建外部client CLI。
- [ ] 编译一个无业务操作的SecureXPC ping/status route。
- [ ] 在签名fixture中验证双向code-signing requirement；ad-hoc仅用于unit fixture，不作为SMJobBless证据。
- [ ] 记录SecureXPC tag/revision选择，特别是0.8.0与post-0.8.0 hardened-runtime fixes的取舍。

### Validation

```text
swift package resolve / package pin check
swift test
universal architecture inspection
Rust FFI contract test
license/provenance/lockfile checks
```

### Exit gate

- 依赖全部exact pin且来源可审计；
- ping route只接受正确签名peer；
- no-path/no-command协议骨架通过negative tests。

## Phase 2 — Closed product policy generation

### Work

- [ ] 从届时backend权威产品策略抽取/生成`KnownSystemApplicationPolicy`。
- [ ] 生成Rust与Swift projection；禁止手工双表。
- [ ] 锁定首期product enum、target-slot enum、fixed basename、Bundle ID、version source/equivalence和allowed actions。
- [ ] Codex/ChatGPT basename兼容只采用application-identity contract中的allowlist。
- [ ] 添加deterministic generation、snapshot和drift tests。
- [ ] 未知产品、未知slot、显示名、path字符串均无法进入helper request。

### Exit gate

- 一个source生成两端策略；
- Swift helper不接受产品identity字符串；
- Windows产品表/行为无变化。

## Phase 3 — Authorization and helper registrar

### Work

- [ ] 通过Authorized定义commit与helper-removal custom rights，使用系统authenticate-admin rule和本地化说明。
- [ ] 实现fresh operation Authorization request、external form transfer、helper-side immediate recheck与destroy-rights。
- [ ] 接入Blessed `authorizeAndBless`/`bless`，映射cancel、requirement、version、disabled-job和unknown failures。
- [ ] 实现bundled/installed helper metadata、signature、version、protocol和health检查。
- [ ] 实现missing/older/equal/newer/tampered/incompatible状态机；拒绝helper降级。
- [ ] 生成并验证`SMPrivilegedExecutables`、`SMAuthorizedClients`、embedded info/launchd plist和Mach service label。
- [ ] 不使用sample的source-hash auto-increment；helper version绑定正式app version。

### Tests

- [ ] Authorization canceled/denied/expired/malformed/external-form reuse。
- [ ] Correct/wrong Team、identifier、version、ad-hoc、tampered peer。
- [ ] Bundled older/equal/newer helper与installed requirements交集。
- [ ] BlessError映射与路径/requirement脱敏。
- [ ] PID变化/复用不影响identity decision。

### Exit gate

- helper可被正式签名fixture安装/更新并建立authenticated XPC；
- mutation route仍未开放，系统目标仍disabled。

## Phase 4 — Source capability handoff

### Work

- [ ] 在existing用户态transaction中增加prepared source capability seam，不改下载/DMG/source owner。
- [ ] 用安全flags打开source app directory FD并绑定file identity/source revision。
- [ ] C ABI bridge duplicate/own FD，SecureXPC使用`FileDescriptorForXPC`或等价typed wrapper。
- [ ] helper用fd-relative方式读取固定bundle metadata/executable/version路径。
- [ ] mutation前重新验证source FD、file kinds、symlinks、containment、policy和revision。
- [ ] 评审Apple `copyfile`/clone API是否满足directory-FD/no-follow递归复制；记录ADR。
- [ ] 若系统API不能满足，实施最小fd-relative copier并覆盖metadata/xattr/ACL/resource-fork fixtures；不形成通用API。

### Tests

- [ ] Client path在FD打开后被替换、删除、symlink-swapped。
- [ ] FD指向file/socket/fifo/other directory/wrong product。
- [ ] Nested symlink、hard link、escape、special files、excess depth/count/size。
- [ ] Source revision在preflight与mutation间变化。
- [ ] App bundle metadata包含binary plist与product-specific version source。

### Exit gate

- root helper读取的是opened object而不是后来路径；
- 不存在path-only commit route。

## Phase 5 — Root transaction and recovery

### Work

- [ ] 实现单route `commitKnownApplication`，内部完成root-only staging/backup/commit/verify/rollback。
- [ ] target仅由closed target slot解析为`/Applications`固定直接子项。
- [ ] 生成same-volume stage/backup，拒绝pre-existing、symlink和identity drift。
- [ ] 实现root-private versioned receipt，mutation前fsync，阶段更新后fsync。
- [ ] fresh/update分别处理；update保留exact selected slot。
- [ ] commit前检查application-running/target revision/authorization/source revision。
- [ ] commit后重新验证source/stage/installed identity/version等价。
- [ ] verification失败时移除exact replacement并restore/reverify backup。
- [ ] helper启动/请求前执行bounded recovery；unknown/multiple/drift进入`recovery_required`。
- [ ] terminal后bounded cleanup、parent fsync和structured result。

### Fault-injection matrix

- [ ] copy开始/中途/完成失败。
- [ ] stage验证失败。
- [ ] target移动backup前/后helper被kill。
- [ ] replacement rename前/后helper被kill。
- [ ] target验证前/后helper被kill。
- [ ] rollback remove/rename/reverify失败。
- [ ] stage/backup/target被第三方替换。
- [ ] receipt损坏、未知version、多receipt。
- [ ] disk full、read-only、permission、fsync failure。

### Exit gate

- 每个phase的crash recovery可证明；
- 旧app不丢失或明确`recovery_required`；
- helper只操作fixed target与自己generated paths。

## Phase 6 — Rust lifecycle integration

### Work

- [ ] 实现crate-private `MacSystemCommitPort`和Swift bridge adapter。
- [ ] 用户scope继续走existing unprivileged transaction。
- [ ] 生产路径上system scope仍返回 `authorization_required`；测试可注入fake port。
- [ ] 现有opaque inventory/target/revision在进入helper前fresh revalidation。
- [ ] 复用现有job stage；只扩展helper-specific reason codes。不增加renderer root API，不为install-policy预做目录/Claude UX。
- [ ] post-helper success后fresh inventory验证exact path/scope/version/no duplicate。
- [ ] readback失败映射rollback/recovery语义；不能乐观成功。
- [ ] `production_enabled()`保持false；unsigned/debug不得启用系统目标。

### Tests

- [ ] renderer请求仍拒绝path/URL/command/helper字段。
- [ ] stale inventory/revision authorize zero helper mutation。
- [ ] helper success + inventory failure不变green。
- [ ] user scope与system scope不互相fallback。
- [ ] Codex专用job ownership不被Agent slot吞并。
- [ ] Windows compile/contracts保持原样。

### Exit gate

- portable fake-helper integration通过；
- production system target仍等待signed HIL flag。

## Phase 7 — Helper lifecycle and explicit removal

### Work

- [ ] backend投影helper missing/update-required/ready/incompatible/tampered/recovery-required。
- [ ] 实现closed `removeHelper` route与独立admin right；不新建Settings产品页。
- [ ] removal拒绝active/recovery transaction，且只删除fixed helper/plist/receipt artifacts。
- [ ] client确认removal后的连接/文件/register状态。
- [ ] 用户文案区分helper授权取消、app commit失败、rollback和recovery，不泄露内部路径。
- [ ] 后续install-policy任务负责把系统目标变成可点的产品action；本任务只保证reason/port可复用。

### Exit gate

- helper生命周期可恢复；
- 没有`sudo launchctl`作为产品流程。

## Phase 8 — Build, signing, release and verifier

### Work

- [ ] 集成Swift package build到现有host-native/release任务；不在mise公共task中加入不合规shell行为。
- [ ] 产出universal client/helper并嵌入最终FyAgent.app固定位置。
- [ ] 扩展现有macOS signing script执行inside-out nested signing，再签main app。
- [ ] 扩展signed-app verifier检查helper/client architectures、identifier、Team、runtime、timestamp、embedded plists、requirements、version、Mach service和bundle paths。
- [ ] 更新Tauri/macOS Info.plist merge与bundle structure tests。
- [ ] 保持现有single-DMG notarization/staple流程；最终mounted DMG再次验证nested code。
- [ ] preflight记录unsigned/structure-only证据，formal记录Developer ID/notary证据；二者不得混淆。
- [ ] 更新dependencies/license/NOTICE/build metadata与change classifier。

### Verification

```text
Swift package tests and exact-resolution check
Rust fmt/check/clippy/test
V2 typecheck/unit/browser tests for affected states
release workflow mutation/fixture tests
universal lipo checks
codesign nested requirement and deep/strict verification
notary/stapler verification in formal build
```

### Exit gate

- formal artifact结构可复现；
- 不存在unsigned nested helper或旁路notarization。

## Phase 9 — Signed HIL

### Environment gate

- [ ] Developer ID Application identity/team与正式release一致。
- [ ] formal helper/client/main app inside-out signed。
- [ ] final DMG notarized/stapled。
- [ ] disposable test account/host与应用pre-state已记录。
- [ ] macOS 12 host与current supported host；Apple Silicon；x86_64 slice verification，Intel host可用时真机执行。

### Matrix

- [ ] helper missing -> admin prompt -> install -> authenticated health。
- [ ] helper equal/older/newer/tampered/disabled。
- [ ] fresh five-product system install（按可安全获取的fixture/产品逐项）。
- [ ] existing system update exact slot。
- [ ] auth cancel/wrong credential/denied。
- [ ] app running、source drift、target drift、多candidate。
- [ ] helper kill at receipt phases并恢复。
- [ ] verification failure -> rollback restored。
- [ ] rollback uncertain -> recovery-required + no retry。
- [ ] app downgrade无法连接new helper。
- [ ] helper update需要/完成/取消。
- [ ] explicit helper removal与reinstall。
- [ ] post-operation inventory exact path/scope/version/no duplicate。
- [ ] no `~/Applications` fallback。

### Evidence

记录sanitized：build/run ID、OS/build、arch、app/helper version、operation/outcome/reason、pre/post inventory、signature/notary verifier结果。不得记录用户名、Authorization bytes、密码、完整用户路径或第三方凭据。

### Enablement gate

只有上述formal HIL通过，backend capability才允许system destination eligible。缺少macOS 12或signed环境时保留disabled，并在任务结论中列为阻断项。

## Phase 10 — Review, spec convergence and closeout

### Review 1 — Architecture/reuse

- [ ] 无second downloader/DMG/product registry/job/RPC/auth wrapper。
- [ ] OSS只承担其成熟边界；business protocol是FyAgent closed operation。
- [ ] current lifecycle task代码未被覆盖。

### Review 2 — Adversarial security

- [ ] 假设renderer、同用户进程、用户可写source path甚至FyAgent进程部分被利用，root能力仍被product/slot/FD/auth/signature限制。
- [ ] fuzz/negative scan覆盖wire、FD、receipt、filesystem和peer identity。
- [ ] 无root shell/process/network/generic path。

### Review 3 — Release/operations

- [ ] update/downgrade/removal/residual helper/recovery/diagnostics可维护。
- [ ] exact pins、licenses、signing/notary/architectures/verifier完整。
- [ ] SMJobBless deprecation与SMAppService future migration记录清楚。

### Closeout

- [ ] 解决所有findings并重跑focused/full gates。
- [ ] 更新backend external-agent、Codex installer、application identity、release workflow及必要frontend reason/state specs。
- [ ] 负向扫描删除obsolete system-authorization-only placeholder或重复owner；保留真实capability unavailable语义。
- [ ] 不提交HIL临时包、证书、profiles、用户日志或绝对用户路径。
- [ ] 按Trellis流程提交、归档；未通过formal HIL时不得归档为fully enabled，但可以归档为“代码与可移植测试完成、系统action仍禁用”。

## Rollback plan

任一阶段发现helper协议、签名或recovery不能安全满足：

1. 保持`MacSystemApplications` non-actionable；
2. 删除/关闭未发布的system capability wiring；
3. 保留普通用户scope与手动Finder安装；
4. 不启用sudo/AppleScript/generic helper fallback；
5. 记录blocked gate和证据，等待新的Apple API/最低系统策略。
