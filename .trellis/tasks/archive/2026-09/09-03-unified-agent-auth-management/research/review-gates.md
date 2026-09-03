# 实施与验收复审门禁

> 本清单用于 implement/check agent。任何“通过”都需要对应代码、测试或正式 HIL 证据；仅有设计说明不算完成。

## 1. 产品与前端体验

### Information architecture

- [ ] `/auth` 是唯一主要账号管理入口；旧Settings只跳转/兼容，不维护独立状态。
- [ ] Agent页只显示摘要和deep link，不能同时保留第二套增删/轮询流程。
- [ ] 页面明确分离：账号身份、软件连接、当前请求Provider。
- [ ] Codex第三方模式明确显示官方账号仍保留；切回不要求有效账号重新登录。
- [ ] OpenCode Desktop无系统CLI时仍可观察/管理Desktop Provider。

### Login wizard

- [ ] OpenAI browser、OpenAI Device Code、xAI Device Code有统一但不抹平协议差异的stage。
- [ ] browser callback页面不显示/存储token/code/state/verifier。
- [ ] Device Code只显示user code与allowlisted官方URL。
- [ ] dialog关闭、route隐藏、重新打开、取消、app restart语义明确。
- [ ] 端口冲突自动转注册fallback/Device Code，不要求用户杀未知进程。
- [ ] success只在SecretRef + metadata + optional projection readback之后发布。

### Actions and recovery

- [ ] remove/switch/reconnect先显示backend生成的影响预览与revision。
- [ ] pending restart不显示已生效。
- [ ] quota/profile失败不等于账号退出。
- [ ] unknown/unsupported/migration blocked都有下一步，不伪装成功。
- [ ] 窄窗口、keyboard、focus、screen reader、reduced motion通过browser tests。

## 2. Frontend architecture

- [ ] `src/v2/**`只importV2或批准的neutral core；页面无Tauri import。
- [ ] strict parser拒绝extra keys、unknown enum、invalid ID/time/URL和secret-like fields。
- [ ] account/connection/session queries有单一query-key owner。
- [ ] backend session不是page-owned timer；hidden route时不丢失。
- [ ] shared component只在两个真实consumer出现后抽取。
- [ ] V1 compatibility不被V2反向依赖。
- [ ] 所有用户文案进入当前语言合同；不新增散落fallback文案。

## 3. Backend owner and reuse

- [ ] 只有一个`ManagedAuthService`运行时owner。
- [ ] OpenAI Device Code从当前Codex manager迁移/复用，不存在第二套实现。
- [ ] xAI discovery/device/refresh从当前xAI manager迁移/复用，不存在Grok副本。
- [ ] Proxy只依赖`ManagedAuthTokenResolver`，不依赖具体manager类型。
- [ ] Agent Auth、Provider Form、V2账号页和旧compatibility调用同一service事实。
- [ ] DB使用现有schema/migration/backup owner，不建立旁路JSON/SQLite index。
- [ ] OS secret使用现有`services::secret`，不引入另一个keyring层。
- [ ] 新依赖有exact pin、license/advisory/维护/跨平台/体积评审；否则使用现有crate。
- [ ] cockpit-tools主仓库没有被复制/vendor/二进制嵌入。

## 4. Credential model

- [ ] `ManagedIdentity`稳定键不以email单独唯一化。
- [ ] `CredentialSession`明确purpose、consumer、refresh owner、generation、SecretRef/version。
- [ ] 同一identity可有多个session；UI聚合不合并refresh token。
- [ ] 不存在`shared` refresh owner或“高级共享refresh token”开关。
- [ ] Proxy/Codex/OpenCode/Grok等refresh-capable consumer默认独立session，或有证明安全的token broker。
- [ ] owner转移时旧owner停止refresh，并完成native generation reconcile。

## 5. OAuth protocol

### OpenAI

- [ ] client ID、scope、authorize/token endpoints、hosted wrapper和redirect ports来自实施时最新第一方证据。
- [ ] PKCE S256、32-byte state、exact redirect URI、one-shot callback、deadline与cancel。
- [ ] listener仅`127.0.0.1`；GET/path/Host/request size受限。
- [ ] state/session/generation不匹配拒绝；late callback无副作用。
- [ ] 1455→1457→Device Code顺序有测试；不cancel未知进程。
- [ ] Device Code server interval、deny/expire/cancel有测试。

### xAI

- [ ] discovery只接受HTTPS+x.ai host，并限制响应大小/超时。
- [ ] `authorization_pending`/`slow_down`/`access_denied`/`expired_token`处理正确。
- [ ] stable `sub` 缺失或identity冲突不保存。
- [ ] refresh-token rotation写入新generation。

## 6. Secret storage and migration

- [ ] access/refresh/id token只存在OS vault或短生命周期zeroizing内存。
- [ ] SQLite、普通JSON、DB export、logs、renderer、tests/task evidence无真实token。
- [ ] 一起轮换的token作为一个versioned bundle原子replace。
- [ ] OS vault create成功/DB失败、DB row/secret缺失、partial migration均可deterministic recovery。
- [ ] `codex_oauth_auth.json`和`xai_oauth_auth.json`迁移幂等、有source hash/journal/bounded backup。
- [ ] 迁移失败不删除旧文件、不继续向明文store写新token。
- [ ] Provider binding remap不会悬空或静默回默认账号。
- [ ] macOS signed entitlement与Windows正式安装用户上下文有HIL。

## 7. Refresh concurrency

- [ ] 每个Credential Session只有一个进程内/跨操作refresh临界区。
- [ ] network refresh前后校验generation、secret version、refresh owner。
- [ ] rotated refresh token与metadata在同一恢复协议中提交。
- [ ] old generation/late result丢弃，不能覆盖新token。
- [ ] native应用刷新时FyAgent不使用旧refresh token竞争。
- [ ] `refresh_token_reused`等错误有closed reason和安全恢复，不打印响应体token。

## 8. Consumer adapters

### Codex

- [ ] file/keyring/auto/ephemeral按正式平台证据开放；不静默降级file。
- [ ] 第三方Provider所有writer/proxy/rollback路径都不删除/覆盖official credential。
- [ ] `preserveCodexOfficialAuthOnSwitch` UI与行为已迁为硬不变量。
- [ ] 切号前同步native最新generation，写后account+provider readback。
- [ ] official→third-party→official在macOS/Windows真实通过。

### Grok Build

- [ ] helper路径若启用，使用第一方`auth_provider_command`合同、trusted binary、opaque ID和脱敏stderr。
- [ ] helper未通过HIL时使用独立native session或保持disabled，不回旧“执行grok login即成功”语义。
- [ ] registry merge保留未知scope，使用兼容lock/atomic write/readback。
- [ ] external login/refresh不被旧generation覆盖。

### OpenCode Desktop

- [ ] Desktop能力不依赖PATH CLI。
- [ ] 官方Provider Connect与FyAgent store projection是两条清晰能力。
- [ ] `auth.json`严格schema、read-modify-write、0600、unknown Provider保留、CAS/readback。
- [ ] 不扫描/复用Desktop私有随机sidecar密码。
- [ ] 独立OpenCode session或OpenCode自有登录；不复制Codex/Proxy lineage。
- [ ] reload/restart行为由stable build HIL决定。

### Proxy

- [ ] 只允许`purpose=proxy_upstream`且`refresh_owner=fyagent`。
- [ ] resolver只向请求边界提供access material，不返回refresh token/SecretRef。
- [ ] native-owned误绑定返回closed conflict而不是自动抢refresh owner。

## 9. Transaction and rollback

- [ ] target/account/session/connection在确认前fresh observation并生成revision。
- [ ] commit前revalidate target与generation。
- [ ] 写native store使用exact preimage/hash/revision，external change时不覆盖。
- [ ] readback失败可回滚；preimage已变化则进入`recovery_required`。
- [ ] restart失败保留安全projection并标`pending_restart`，不删除账号。
- [ ] remove account先解绑/验证，再删SecretRef，最后删metadata。

## 10. Evidence and release

- [ ] 关键协议来源记录exact commit、日期、license、NOTICE与修改说明。
- [ ] 自动化命令全部使用仓库锁定入口，失败不通过跳过/改CI规避。
- [ ] macOS与Windows HIL记录版本、closed outcome、sanitized revision；不含账号/token/path。
- [ ] 未通过HIL的capability保持disabled/unsupported。
- [ ] owning specs更新并删除互相矛盾的旧authority描述。
- [ ] 最终diff审查证明任务范围只覆盖统一Auth控制面及必要迁移。
