# 规划复审记录

> 复审对象：`prd.md`、`design.md`、`implement.md`、调研记录与 Trellis 上下文
> 复审状态：可提交用户评审；任务仍为 `planning`，未批准进入实现。

## Round 1 — 产品模型与前端操作体验

### 检查问题

- 用户能否在不理解 OAuth、token、credential store 的情况下完成操作？
- “保存了账号”“软件连接了账号”“当前请求走哪个 Provider”是否被混为同一状态？
- 登录过程中关闭对话框、切换页面、端口冲突、授权取消、重启应用时会发生什么？
- Codex 切到第三方 API 后，用户是否明确知道官方登录仍保留？
- OpenCode Desktop 是否错误依赖系统 PATH 中的 CLI？

### 发现

1. 现有 V2 只有 Agent 详情中的 Auth 区块，没有能够承载多账号、跨软件连接和影响预览的一级页面。
2. 旧 Auth Center 已有多账号、默认账号、额度和 Device Code UI，但把 Codex/xAI 主要描述为 Proxy 上游，且轮询/error owner 在 renderer。
3. 单一“已登录”徽标无法准确表达账号、consumer connection 与当前模型来源。
4. 删除账号或切换连接如果没有 backend 影响预览，用户无法判断会不会断开其他软件。

### 调整

- 建立 V2 `/auth` 一级“账号与认证”页面，Agent 页只保留摘要和深链。
- 页面分为“账号”和“软件连接”；connection 卡片同时展示账号连接、当前模型来源、官方 session 是否保留。
- 将 browser/device 登录设计为 backend-owned session；dialog 关闭不自动取消，route hidden 可恢复。
- destructive mutation 采用 backend preview + revision；pending restart 不显示已生效。
- OpenCode Desktop 以官方 Provider Auth/credential store 为准，CLI 仅是独立可选 surface。

### 结论

通过。前端信息架构与状态机已作为 Phase 1 先于生产 backend 的硬门禁；不允许“后端先做完再补 UI”。

## Round 2 — 后端架构与复用

### 检查问题

- 是否在现有 Codex/xAI manager、SecretRef、Agent Auth session、Provider binding、Proxy owner 旁边又造一套？
- 前端、Proxy、Agent 与 Provider Form 是否会继续各自读取不同状态？
- 模块边界是否会形成巨型 OAuth switch 或 commands 业务 owner？

### 发现

1. FyAgent 已有 OpenAI Device Code、多账号、refresh lock、xAI discovery/device/refresh，以及统一旧 Auth commands。
2. `services::secret` 已有 macOS Keychain、Windows Credential Manager、opaque ref/version 与 zeroizing material，缺的是首个 production consumer/recovery合同。
3. Provider `authBinding` 与 Proxy concrete manager 依赖需要迁移，而不是删除后重建。
4. V2 现有 FeaturePort/strict parser/session 模式可以复用，但 V1 组件不能直接导入。

### 调整

- 设计唯一 `ManagedAuthService` facade；现有 manager协议逻辑迁入 provider adapter，旧 commands 作为 compatibility adapter。
- Proxy 只依赖窄 `ManagedAuthTokenResolver`；Agent/Provider/V2 共享同一 service 事实。
- 账号 metadata 进入现有 SQLite schema owner；secret 进入现有 SecretRef；不创建旁路 DB/JSON/keyring abstraction。
- provider-specific adapter保持显式，不用巨型通用 OAuth trait/switch。
- 新增 `current-fyagent-integration-seams.md`，列出真实文件 owner与迁移接缝；源码不塞进 JSONL 注入。

### 结论

通过。任务明确要求“迁移现有 owner”，不是“并排新增 owner”；Trellis implement/check context 已按 Spec/Research 重新整理且无警告。

## Round 3 — Token 安全、并发与恢复

### 检查问题

- 同一个 refresh token 是否可能被 FyAgent、Codex、OpenCode、Grok 同时刷新？
- OS vault与SQLite无法物理同事务时如何恢复？
- native app 外部写回新 token 后，旧异步结果会不会覆盖它？
- token/code/state/verifier 是否可能进入 renderer、日志、DB export 或测试工件？

### 发现

1. “统一账号”若被误实现为“复制一份 refresh token给所有 consumer”，会造成旋转凭据复用与随机失效。
2. 现有 Codex/xAI JSON store虽有部分原子写，但仍是明文 token owner；扩大用途前必须迁往 OS vault。
3. 单纯进程内 mutex 不足以处理 native app、多个窗口或晚到网络结果；需要 owner + generation + version revalidation。
4. browser callback 携带的是 code/state，不能把 callback URL 当 token；完整 URL也不能写日志。

### 调整

- 分离 `ManagedIdentity` 与 `CredentialSession`；同一 identity 可有多个 purpose-specific session。
- 每个 rotating lineage只有一个 refresh owner；默认为每个 refresh-capable consumer建立独立 session，不提供强制共享开关。
- 一起轮换的 access/refresh/id token作为一个 SecretRef bundle replace；SQLite只存 opaque ref/version/generation/status。
- 使用 provisioning/migration journal、generation CAS、exact preimage/revision与 readback 处理跨存储事务和 external writer。
- wire/DOM/log/export建立 forbidden-field tests；raw OAuth response/error不越过 backend安全边界。

### 结论

通过，但属于实现期最高风险部分。SecretRef正式签名构建、refresh owner转移和故障注入未通过前，consumer capability必须保持 disabled。

## Round 4 — 第一方协议、开源复用与许可证

### 检查问题

- 登录协议是否由第一方事实支持？
- 能否直接复用现有/开源模块，而不是抄一个管理器？
- `cockpit-tools` 是否允许直接复制到 FyAgent？
- 哪些行为仍只是源码推断，必须真机验证？

### 发现

1. OpenAI Codex 第一方已经公开 loopback PKCE、Device Code、凭据 store 和 app-server account API。
2. xAI Grok Build 第一方已经公开 OIDC Device Code、registry、refresh、lock、external auth command与热加载合同。
3. OpenCode 第一方提供 Desktop Provider Auth 与统一 credential schema。
4. `cockpit-tools` 证明产品可行性并提供大量故障处理参考，但其主仓库 CC BY-NC-SA 4.0 不适合作为未经授权的直接代码来源。
5. native store、reload/restart、signed entitlement、Windows user context仍需要正式 HIL，不能由源码阅读宣称可用。

### 调整

- 调研记录固定 exact source commit、license、关键文件与可复用范围。
- 协议实现优先从 FyAgent当前代码与 OpenAI/xAI/OpenCode第一方 Apache/MIT source收敛。
- `cockpit-tools` 只作产品/状态机/故障参考；代码复用需要独立书面授权与NOTICE审查。
- Phase 0 要求刷新上游事实和dependency/license decision；每个 consumer有独立HIL capability gate。

### 结论

通过。研究足以形成任务，但不替代实施当日的上游刷新与正式 HIL。

## Round 5 — 范围、交付切片与可审查性

### 检查问题

- 该任务是否大到无法在一个 PR 中安全评审？
- 能否在前端体验冻结后按独立 capability 分阶段提交？
- 某个 consumer 阻断时，是否会拖累已完成的安全底座或迫使猜测性 fallback？

### 发现

该工作横跨 V2 IA、OAuth、OS vault、DB migration、Proxy、三个 consumer和双平台 HIL，不适合作为一个巨型实现 PR。直接一次性推进会增加 secret migration、回滚和 UI 语义同时失控的风险。

### 调整

本目录作为统一产品/架构父任务。批准实施后，在 Phase 0 将以下切片创建为 Trellis child tasks 或等价独立 PR：

1. V2 Auth wire + Mock UX；
2. ManagedAuth metadata + SecretRef + legacy migration；
3. OpenAI browser/device + Codex consumer；
4. xAI + Grok consumer；
5. OpenCode Desktop consumer；
6. Proxy/V1 convergence + failure hardening + HIL/spec closeout。

切片共享同一 wire/domain contract，不得各自创建 manager/store。每个切片都有独立 capability flag、测试与回滚；未通过 HIL 的 consumer不阻断其他已验证能力，但保持不可执行。

### 结论

通过。任务适合作为 P0 统一父任务；**不应直接以单个巨型 PR 执行**。

## Remaining uncertainties

以下均已被显式放入 Phase 0/HIL，而不是当作已知事实：

- 当前 stable Codex Desktop/CLI 各 store mode 的真实刷新与缓存行为；
- Grok `auth_provider_command` 在目标正式版本和双平台上的完整合同；
- OpenCode Desktop credential store变更后的热加载/重启要求；
- macOS signed entitlement、Windows正式安装用户上下文下的OS vault；
- 中国大陆网络环境中的官方认证域名可达性与失败体验。

## Planning verdict

- Product/UX：通过，前端先行。
- Architecture/reuse：通过，单一 owner + 现有模块迁移。
- Security/concurrency：设计通过，实施/HIL为硬门禁。
- Open-source/license：通过，第一方/宽松许可优先，受限项目不复制。
- Scope：通过，作为父任务，实施前拆分子任务/PR。
- Implementation authorization：**未获得；保持 `planning`。**
