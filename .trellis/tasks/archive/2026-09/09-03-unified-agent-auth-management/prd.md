# PRD：统一 Agent 官方登录与账号管理体验

> 状态：Implementation
> 优先级：P0
> 产品顺序：**前端操作体验 > 正确性与安全 > 后端复用与扩展性 > 功能覆盖速度**

## 1. Goal

在 FyAgent 中提供一个统一、清晰、可验证的“账号与认证”体验，使用户能够通过厂商官方登录流程管理 Codex/OpenAI、Grok/xAI 与 OpenCode Desktop 的登录状态，并在官方订阅与第三方 API Provider 之间切换时保持账号关系可理解、凭据不丢失、状态不误报。

本任务不是给现有按钮换文案，而是建立一个长期可扩展的账号控制面：

- 用户在一个地方查看和管理官方账号；
- 用户能看懂“账号身份”“这个软件连接了哪个账号”“当前模型请求走官方还是第三方”三件不同的事情；
- 登录过程由 FyAgent 组织，但始终使用官方授权页面与协议；
- token 永不展示给 renderer 或用户；
- Codex、Grok Build、OpenCode Desktop 与 FyAgent Proxy 复用统一账号能力，但不共享会产生并发刷新风险的同一 refresh-token lineage；
- 现有能力迁移到一个 owner，不留下旧 Auth Center、Agent Auth 和 Proxy Auth 三套相互矛盾的状态。

## 2. Background

当前产品存在以下用户问题：

1. V2 Agent 页面只显示一个较小的“认证状态”区块；Codex 被引导到旧版设置中的 Auth Center，Grok 只能 handoff，OpenCode 仍以 CLI `auth` 为主要观察/执行方式。
2. 旧 Auth Center 能管理多账号和额度，但文案把 Codex/xAI 账号主要描述为 Claude/Proxy 的上游订阅，不能表达 Codex/Grok/OpenCode 原生登录连接。
3. 用户无法直接分辨：
   - 已经在 FyAgent 保存了账号；
   - 账号已经连接到某个 Agent；
   - 该 Agent 当前正在使用官方订阅还是第三方 API。
4. Codex 第三方 Provider 切换存在可选的“保留官方登录”设置；在本产品目标下，保留官方登录应是正确性不变量，不应由用户承担风险选择。
5. 现有 OAuth manager 将 refresh token 存在应用 JSON 文件中；项目已经有 OS-native `SecretRef`，但尚未成为生产账号凭据 owner。

## 3. Target users and key jobs

### 3.1 单账号用户

- 使用 ChatGPT Plus/Pro 登录 Codex；
- 临时切到 DeepSeek/Kimi 等第三方 API；
- 之后一键切回 OpenAI Official，不重新登录。

### 3.2 多账号用户

- 保存多个 OpenAI 或 xAI 账号；
- 清楚知道哪个是默认账号、哪个连接到 Codex/Grok/OpenCode/Proxy；
- 重新授权、移除或切换账号时不会误伤其他软件连接。

### 3.3 OpenCode Desktop 用户

- 不额外安装系统 PATH CLI；
- 在 FyAgent 看到 OpenCode 已连接的 Provider；
- 通过官方 Provider Auth 或受控凭据投影完成连接；
- 明确知道修改是否需要重启 OpenCode。

### 3.4 受限网络或无回调环境用户

- browser callback 不可用时能切换到 Device Code；
- 网络失败、端口占用、授权取消、超时都能得到可操作提示；
- 不接触 token 或手工编辑敏感配置。

## 4. Product decisions

1. 新增 V2 一级页面 `/auth`，用户显示名称为“账号与认证”；它是唯一主要账号管理入口。
2. Agent 目录与详情保留紧凑状态摘要，但完整增删、重新登录、连接和切换均进入中央页面。
3. 旧 Settings Auth Center 迁移为兼容入口/跳转，不继续独立演进；现有 Copilot 能力迁入统一页面但不在本任务重写协议。
4. OpenAI 登录默认使用官方 browser loopback PKCE；失败或用户主动选择时使用官方 Device Code。
5. xAI/Grok 使用官方 OIDC discovery + Device Code。
6. OpenCode Desktop 不依赖系统 PATH CLI。优先使用其官方 Provider Auth；需要凭据投影时只写官方 credential store，并遵守唯一 refresh owner。
7. 第三方 Provider 切换永远不得删除、覆盖或失效化 Codex 官方账号 session。
8. 账号身份可统一展示；refresh-token session 默认按 consumer 隔离。
9. 任何“成功”必须来自 backend 完成凭据保存/投影并重新观察后的结果，不能由浏览器返回或进程退出推断。

## 5. Experience requirements

### UX-01：统一入口与返回路径

- 侧边栏“AI 软件配置”组中提供“账号与认证”。
- Agent 卡片和 Agent 详情显示短状态及“管理账号/管理连接”入口。
- 从 Agent 进入时，`/auth` 自动定位对应软件/Provider；返回后恢复原 Agent 与 section。
- 不出现“设置里一套、Agent 页一套、Provider 表单里又一套”的竞争入口。

### UX-02：账号与软件连接分层

中央页面至少提供两个分段：

1. **账号**：按 OpenAI、xAI、GitHub Copilot 等身份提供方组织；显示账号、订阅/状态、默认标识、重新登录、移除。
2. **软件连接**：显示 Codex、Grok Build、OpenCode 与 FyAgent Local Proxy 当前绑定的账号、连接健康状态和当前请求模式。

界面必须明确区分：

```text
账号：person@example.com
软件连接：Codex 使用该账号
当前请求：DeepSeek API（官方 ChatGPT 登录仍保留）
```

不得把这三类状态压成一个“已登录”徽标。

### UX-03：账号总览

每个账号卡片至少显示：

- 可识别登录名/email；
- Provider 品牌；
- 正常、需重新登录、检查中、不可确认等状态；
- 默认账号标识；
- 已连接软件数量；
- 可用时显示 plan/quota 摘要，但额度失败不把账号误判为退出登录；
- 最近成功认证时间（非 token 到期倒计时）。

内部 credential ID、workspace routing ID、SecretRef、generation、原始 error 和 token 不显示。

### UX-04：登录向导

“添加账号”打开统一向导：

1. 选择账号类型和用途/要连接的软件；
2. 显示将打开的官方域名与登录方式；
3. 发起 browser callback 或 Device Code；
4. 展示单一进行中状态、取消动作和安全提示；
5. 后端完成 token exchange、保存和可选连接；
6. 显示账号与连接结果，并提供“完成”或“查看连接”。

要求：

- browser flow 不展示 code/token；Device Code 只展示厂商 user code；
- 打开浏览器失败时保留可点击官方链接；
- loopback 端口不可用时明确自动回退到 Device Code；
- 提供“改用设备码”和“重新开始”，不要求用户理解 PKCE；
- 取消会停止 backend session，晚到 callback 不得保存账号；
- 页面隐藏/路由切换不终止 backend session；返回后恢复进行中状态；
- 应用重启后安全地标记旧 session 已结束，不恢复过期 verifier/code。

### UX-05：软件连接

每个 consumer 卡片显示：

- 目标软件与安装目标（存在多个安装时）；
- 当前连接的账号/Provider；
- “已连接”“需要重新登录”“等待重启”“无法确认”等状态；
- 当前使用官方订阅、第三方 API 或未配置；
- 可执行动作：连接、切换账号、断开、刷新状态、打开软件；
- 操作是否会重启软件、是否会覆盖该软件当前账号，必须在确认前说明。

### UX-06：Codex 官方与第三方无损切换

- 切到第三方 API 时，界面明确显示“当前使用第三方 Provider；OpenAI 官方账号仍保留”。
- 切回 OpenAI Official 时，用户选择已保存的官方账号即可；有效 session 不重新登录。
- 若官方 session 已失效，切换前进入重新登录向导，不先破坏当前第三方配置。
- 删除一个官方账号前列出受影响连接；用户确认后才解除绑定和删除 secret。
- “保留官方登录”不再是默认关闭的危险选项。

### UX-07：OpenCode Desktop

- 状态来自 Desktop 同源 credential store/官方 Provider Auth，不把系统 CLI 缺失当成 Desktop 未安装或未登录。
- 显示已连接 Provider 列表，而非一个伪造的全局登录状态。
- 用户可进入 OpenCode 官方连接流程，或选择 FyAgent 管理的兼容账号 session；两种方式必须标明 credential owner。
- 写入 credential store 后进行 readback；当前 OpenCode 版本需要重启时，先请求确认并恢复到同一页面。

### UX-08：错误、未知与恢复

界面使用闭集、可行动状态：

- 官方页面未完成/用户取消；
- 回调端口被占用并已回退；
- 登录超时；
- token exchange/refresh 被拒绝；
- OS 凭据库锁定/拒绝/不可用；
- 原生 credential store 模式暂不支持；
- 软件正在运行，需要重启；
- 原生文件被外部修改；
- 状态无法确认。

未知状态不能显示成功；错误不得直接呈现 raw backend message、URL query、路径或 token。

### UX-09：响应式与可访问性

- 键盘可以完成页签、账号选择、向导、确认和取消；焦点在 dialog 打开/关闭后正确恢复。
- 状态不仅依赖颜色；所有 icon-only 控件有可访问名称。
- 窄窗口下账号列表和详情切换为单列，主要动作始终可见。
- 长 email/provider name 有安全截断并可访问完整文本；不产生横向溢出。
- reduced-motion 下不依赖动画表达认证进度。
- 加载、空、部分失败、无账号、无连接均有单一清晰状态和下一步动作。

## 6. Functional requirements

### FR-01：统一 managed-auth domain

- OpenAI、xAI 与现有 Copilot 通过一个 backend facade/port 暴露账号摘要、登录 session、默认账号、删除和重新授权。
- Proxy、Agent Auth、Provider Form 与 V2 Auth 页面不直接持有具体 manager lock。
- 不允许新增第二个 OpenAI/xAI token store。

### FR-02：安全凭据

- refresh/access/id token 存入 OS-native `SecretRef`；SQLite 和 renderer 只看到 opaque refs/metadata，renderer DTO 不含 SecretRef 本体。
- 一组会一起轮换的 token 作为一个 versioned secret bundle 保存。
- 不允许明文 JSON fallback；迁移失败时保留旧数据与功能，显示恢复状态。

### FR-03：账号身份与 credential session

- 同一账号身份可关联多个用途明确的 Credential Session。
- session 包含唯一 refresh owner、generation、过期/reauth 状态与 consumer purpose。
- 同一旋转 refresh lineage 不得被多个 owner 同时刷新。

### FR-04：OpenAI 登录

- browser loopback PKCE 为默认；只使用第一方注册的 loopback 端口/路径。
- Device Code 为正式 fallback。
- 完成后解析并校验稳定身份 claim；身份不明确不保存。

### FR-05：xAI/Grok 登录

- 使用 OIDC discovery 和 Device Code；校验 endpoint host/scheme。
- 保存可刷新 session；支持 token rotation 和 reauth 状态。
- 能以官方支持的方式连接 Grok Build，并重新观察结果。

### FR-06：OpenCode Desktop

- 读取/写入官方 auth schema，保留未知 Provider 与字段，owner-only 权限和原子写入。
- 多安装时使用稳定 inventory/target capability；不扫描内部随机 sidecar 密码。
- 连接/断开后进行 readback，必要时受控重启。

### FR-07：Codex Provider switching

- 官方 credential 与第三方 API credential 分开拥有。
- 第三方切换不修改官方 Credential Session。
- Codex `file/keyring/auto/ephemeral` 模式按真实能力矩阵处理；不支持时 fail-closed，不静默切换 store。

### FR-08：迁移与兼容

- 将现有 `codex_oauth_auth.json`、`xai_oauth_auth.json` 迁移到 SecretRef + metadata store，保留可恢复备份与幂等状态。
- 保持现有 Provider `authBinding` 可映射到新的 Credential Session。
- 旧 Auth Center 只作为兼容跳转，迁移完成后删除重复状态与轮询逻辑。
- Copilot backend 行为不回归。

### FR-09：后台 session 与并发

- 登录、refresh、connection switch 和 native projection 由 backend session/job 拥有。
- 同 provider/identity/consumer 的冲突操作被拒绝或串行化。
- 外部应用刷新或改写 credential store 时，通过 generation/readback 合并，旧结果不得覆盖新结果。

## 7. Reuse and engineering constraints

1. 复用顺序：当前 FyAgent owner → 当前依赖/共享 UI → 第一方 Apache/MIT 实现 → 已审查 OSS adapter → 最小自研。
2. V2 页面不得导入旧版 `src/components/**`、`src/hooks/**`、`src/lib/**` 实现；通过 V2 FeaturePort 接入。
3. 页面不直接调用 Tauri；strict parser 在 feature-port adapter 边界完成。
4. 共用 login session shell、account card、connection row、status/reason mapping；Provider 特有协议保持显式 adapter，不做巨型通用 if/switch。
5. 禁止直接复制 CC BY-NC-SA 的 cockpit-tools 主仓库代码。
6. 任何新增依赖需 exact version、license、advisory、维护状态、跨平台和 bundle footprint 评审。
7. 不记录 token、authorization code、state、verifier、完整 callback URL、原生凭据路径或用户目录。

## 8. Out of scope

- Claude Code 官方账号体系的重新设计；现有 agent-owned observer 保持。
- QoderWork、TRAE Work、WorkBuddy 的 token 接管。
- 云端同步、团队共享或远程备份 OAuth 凭据。
- 自动绕过中国大陆网络限制、未授权 OAuth 镜像或中转服务。
- 浏览器密码、账号密码、2FA secret 管理。
- 任意第三方 OAuth Provider 插件市场。
- 不经用户确认自动重启/关闭桌面软件。

## 9. Acceptance criteria

### 前端验收

- [x] V2 存在唯一“账号与认证”主要入口，并能从 Codex/Grok/OpenCode Agent 卡片深链进入和返回。
- [x] 账号页清楚分离账号、软件连接、当前请求 Provider；Codex 第三方模式明确显示官方登录仍保留。
- [x] OpenAI browser PKCE、Device Code fallback、xAI Device Code 三条交互都有完整准备/等待/成功/取消/超时/失败状态（V2 + backend session 自动化；真机成功仍待 HIL）。
- [x] OpenCode Desktop 不因系统 PATH 无 CLI 而显示认证不可用。
- [x] 多账号可设默认、重新登录、连接、切换、移除；破坏性操作显示影响范围。
- [x] 任何 UI、DOM、事件、错误和测试快照都不包含 token、code、verifier、SecretRef 或原始路径。
- [ ] 键盘、focus、screen-reader label、窄窗口、reduced-motion、隐藏路由暂停/恢复均有自动化覆盖。

### 后端验收

- [x] 现有 Codex/xAI manager 已收敛到一个 managed-auth owner；Proxy/Agent/页面复用同一服务。Leftover `auth_*` / Copilot 登录/删除已 fail-closed；未密封 JSON 仍是只读兼容源。
- [ ] OAuth secret 已迁入 OS-native SecretRef；数据库/普通 JSON 无 refresh/access/id token 明文。未迁移源在密封前仍可读明文 JSON。
- [x] 一个 Credential Session 只有一个 refresh owner；并发、晚到 refresh、外部写回与 generation CAS 有测试。
- [x] Codex 第三方切换从所有写路径证明不会删除/覆盖官方 session；切回有效账号无需重新登录。
- [ ] Grok 与 OpenCode 原生 store 更新为 merge + atomic write + lock/readback，未知字段/Provider 不丢失。Grok helper/file 生产写入保持关闭；OpenCode 外部写热加载未 HIL 证明。
- [x] 不支持的 credential-store/平台组合返回闭集不可操作状态，不做明文或 CLI fallback。
- [x] 旧 JSON 迁移幂等、失败可恢复、Provider binding 不悬空。

### 工程与证据验收

- [ ] 第一方/OSS exact source、license、NOTICE、修改记录和 dependency lock 完整。
- [x] 前端 strict wire parser、backend DTO exact keys、forbidden-field scan、secret redaction tests 通过。
- [ ] macOS 与 Windows 真机完成 Codex/Grok/OpenCode 登录、refresh、切换、重启、外部修改和回滚矩阵。
- [x] HIL 未通过的能力保持 disabled/unsupported，不能以 mock、源码阅读或交叉编译代替。
- [x] 相关 Trellis backend/frontend specs 与当前实现一致：leftover `auth_*` / Copilot 登录/删除已标为 disabled compatibility；旧 Auth Center 不再作为第二 owner。

## 10. Definition of done

完成不是“能拿到 token”或“页面显示已登录”。完成必须同时满足：

1. 用户能顺畅理解并完成操作；
2. backend 能证明凭据、绑定和 native state 已提交并 readback；
3. token 生命周期只有一个 owner；
4. 旧入口与旧 store 已安全迁移或明确保留兼容状态；
5. 自动化与真实平台证据覆盖主要成功和失败路径；
6. 规范、文档、许可证和回滚方案完整。
