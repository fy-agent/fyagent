# 调研记录：统一 Agent 官方登录与账号管理

> 调研日期：2026-09-03
> FyAgent 基线：`ecb64d70945b0521661aefea3078b88710c93fc6`（`dev/laiyongjie`）
> 结论用途：为本任务冻结产品边界、复用边界、协议来源和许可证边界；实现开始前仍需刷新上游版本与真实平台证据。

## 1. 结论摘要

1. **Codex 本地回调 OAuth 可行，且属于第一方已公开实现。** OpenAI Codex 官方源码使用 Authorization Code + PKCE、短生命周期 localhost callback server、`state` 校验、授权码换取 `access_token` / `refresh_token` / `id_token`，默认端口为 `1455`，并提供已注册的 `1457` 回退端口。
2. **Grok Build 官方登录也可以由 FyAgent 管理。** xAI 第一方 Grok Build 已公开 OIDC discovery、Device Authorization Grant、`~/.grok/auth.json`、refresh-token 自动轮换、文件锁、外部认证命令和凭据热加载合同。
3. **OpenCode Desktop 有自己的 Provider Auth 与统一 credential store。** OpenCode 官方 `Auth` owner 将 OAuth/API 凭据写入 `Global.Path.data/auth.json`，OAuth shape 为 `type/access/refresh/expires/accountId?`，写入权限为 `0600`；Desktop 的 Provider Connect 调用官方 Provider Auth API，而不是要求用户额外安装系统 PATH CLI。
4. **`cockpit-tools` 证明了跨 Agent 账号管理的产品可行性，但不能默认复制代码。** 它实现了 Codex localhost PKCE、Device Code、Grok OIDC、多账号、原生凭据投影、OpenCode provider entry 同步和复杂 refresh-token 协调；主仓库许可证是 CC BY-NC-SA 4.0，未经书面商业授权不得作为 FyAgent 代码来源。
5. **FyAgent 已经有大部分底座。** 当前已有 Codex/xAI 多账号 OAuth manager、统一旧版 Auth Center、Provider `authBinding`、V2 Agent Auth session/port、OS Keychain/Credential Manager `SecretRef` 服务。正确做法是重构现有 owner，而不是再造第二套 OAuth、第二个账号中心或第二个 token store。
6. **一个旋转 refresh-token lineage 只能有一个刷新 owner。** 同一 refresh token 不能同时交给 FyAgent Proxy、Codex、OpenCode 或其他进程各自刷新。统一 UI 可以把多个 Credential Session 归并为同一个账号身份，但默认必须为每个 refresh-capable consumer 建立独立 session，或显式转移唯一 refresh lease。

## 1.1 可复现仓库入口

- OpenAI Codex: https://github.com/openai/codex
- xAI Grok Build: https://github.com/xai-org/grok-build
- OpenCode: https://github.com/anomalyco/opencode
- cockpit-tools（只读参考）: https://github.com/jlcodes99/cockpit-tools

上述仓库必须与下表 exact commit 一起使用；只看默认分支当前内容不能复现本次结论。

## 2. 证据与来源矩阵

| 来源 | 审查版本 | 许可证 | 已确认能力 | 本任务使用方式 |
| --- | --- | --- | --- | --- |
| OpenAI `openai/codex` | `36984da4424cb91b6bc88c6af8d73207930ac729`（2026-09-03） | Apache-2.0；含 NOTICE | localhost PKCE、1455/1457、state、token exchange、Device Code、file/keyring/auto store、app-server account API | **协议与安全实现的主要权威来源**；按 FyAgent 边界重构/复用，不复制无关 UI/runtime |
| xAI `xai-org/grok-build` | `72a61251fcffb464bcc687aeb5a998e5a98ec0c9`（2026-09-01） | Apache-2.0；含 third-party NOTICE | OIDC discovery、Device Code、refresh、`auth.json`、文件锁、外部 auth command、热加载 | **Grok 协议与原生 store 的主要权威来源** |
| OpenCode `anomalyco/opencode` | `b578b7261fc9ec4917fe272df5cc4bd8a056cd5d`（2026-09-03） | MIT | Desktop Provider Connect、Provider Auth API、统一 `auth.json` schema、`0600` 写入 | **OpenCode credential schema 与 Desktop 交互边界权威来源** |
| `jlcodes99/cockpit-tools` | `1e2af3df5f4ecb047571974c278a86af62396e52`（2026-09-03） | CC BY-NC-SA 4.0；商业使用需书面授权 | Codex loopback/device、多账号/切号、Grok OIDC、OpenCode 投影、原生文件/Keychain 写入、refresh-token generation/lock/reconcile | **产品设计、失败案例和测试矩阵参考**；禁止直接复制其主仓库代码 |
| FyAgent 当前树 | `ecb64d70945b0521661aefea3078b88710c93fc6` | 项目自身 | Codex/xAI manager、旧 Auth Center、V2 Auth port/session、SecretRef、Provider authBinding、Codex provider switching | **首选复用 owner**；通过抽取、迁移和严格合同扩展 |

### 2.1 主要上游文件

- OpenAI Codex：
  - `codex-rs/login/src/server.rs`
  - `codex-rs/login/src/device_code_auth.rs`
  - `codex-rs/login/src/auth/manager.rs`
  - `codex-rs/app-server/README.md`
- xAI Grok Build：
  - `crates/codegen/xai-grok-shell/src/auth/device_code.rs`
  - `crates/codegen/xai-grok-shell/src/auth/storage.rs`
  - `crates/codegen/xai-grok-shell/src/auth/manager/**`
  - `crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md`
- OpenCode：
  - `packages/opencode/src/auth/index.ts`
  - `packages/opencode/src/provider/auth.ts`
  - `packages/app/src/utils/server-compat.ts`
  - `packages/app/src/components/settings-v2/providers.tsx`
- cockpit-tools（只读参考）：
  - `src-tauri/src/modules/codex_oauth.rs`
  - `src-tauri/src/modules/codex_account_runtime_switch.rs`
  - `src-tauri/src/modules/codex_account_projection.rs`
  - `src-tauri/src/modules/grok_oauth.rs`
  - `src-tauri/src/modules/grok_account.rs`
  - `src-tauri/src/modules/opencode_auth.rs`

## 2.2 基线快进复核

任务编写期间 `dev/laiyongjie` 从旧基线快进到 `ecb64d70945b0521661aefea3078b88710c93fc6`。新增提交完成了 Grok 官方 npm 安装与 OpenCode Windows Desktop source 规划/实现，并归档了 `09-03-remove-grok-install-opencode-windows`。复核结果：

- 本任务涉及的 OpenAI/xAI manager、Agent Auth action/session、V2 Auth wire、旧 Auth Center、SecretRef、DB schema 与 Provider binding 文件在该区间没有变更；
- 生命周期合同现在更明确：Grok 安装归 Tooling owner，认证不能夹带 registry/version/npm 控制；OpenCode 归 Desktop source，但 Windows 安装仍受签名身份 HIL 门禁；
- “CLI 安装成功不代表官方登录或网络推理可用”已成为现有 lifecycle spec，本任务继续保持安装与认证分离。

相关已归档证据：`.trellis/tasks/archive/2026-09/09-03-remove-grok-install-opencode-windows/research/current-state-and-upstream-evidence.md`。

## 3. 当前 FyAgent 复用审计

| 当前 owner | 已有能力 | 本任务决策 |
| --- | --- | --- |
| `proxy/providers/codex_oauth_auth.rs` | OpenAI Device Code、多账号、credential ID、access cache、refresh lock、Provider binding migration | 抽取为通用 OpenAI managed-auth adapter；补 browser loopback PKCE 与 SecretRef，不保留第二个 Codex manager |
| `proxy/providers/xai_oauth_auth.rs` | xAI discovery/device flow、多账号、refresh、reauth 状态、并发锁 | 抽取为通用 xAI managed-auth adapter；Proxy 与 Grok consumer 共用服务接口，不复制协议 |
| `commands/auth.rs` + `src/lib/api/auth.ts` | 旧版统一账号 DTO 与增删/默认/登录命令 | 作为迁移输入；V2 新 wire 必须严格解析、闭集 reason、无 raw string error |
| `AuthCenterPanel` / `useManagedAuth` | 账号列表、默认账号、Device Code UI、额度 | 不把旧组件导入 V2；提炼交互经验，轮询/超时移回 backend session owner |
| V2 `agent-auth.ts` / `AgentAuthStatusPanel` / `useAgentAuthSession` | 严格 Agent Auth observation/session、后台会话、隐藏页生命周期 | 演进为 Agent 摘要与中央账号页的入口；不在页面新建直连 Tauri 调用 |
| `services/secret/**` | macOS Keychain、Windows Credential Manager、opaque SecretRef、zeroizing material、create/replace/probe/delete | 作为首个生产 consumer 激活；扩 SecretPurpose 与恢复合同，禁止明文 JSON fallback |
| Provider `meta.authBinding` | Provider 到 managed credential 的绑定 | 保持兼容；迁移为 consumer binding 的一个投影，不让 Provider JSON 成为账号主存储 |
| Codex provider switch/config owner | 官方/第三方 config 投影、`preserveCodexOfficialAuthOnSwitch` | 把“官方登录不被第三方切换破坏”升级为硬不变量；逐步废弃可选开关 |

## 4. 协议事实

### 4.1 OAuth callback 中的内容

浏览器本地回调通常携带：

```text
GET /auth/callback?code=<one-time-code>&state=<opaque-state>
```

回调 URL **不是 token 存储接口**。本地服务必须：

1. 只监听 loopback；
2. 校验 path/method/Host/size；
3. 常量时间或等价严格校验 `state`；
4. 使用对应 `code_verifier` 与完全相同的 `redirect_uri` 换 token；
5. 不将 code、state、verifier、token 或完整 callback URL 写日志；
6. 返回最小成功/失败页面并关闭会话。

### 4.2 OpenAI

- 首选 browser loopback PKCE；端口只使用第一方注册值，不使用任意动态端口猜测 allowlist。
- `1455` 不可用时使用第一方实现确认的 `1457`；两者不可用时回退 Device Code。
- Device Code 继续作为 headless、端口冲突或浏览器回调不可达时的正式能力，不是隐藏 debug fallback。
- OAuth 参数、scope、hosted-login wrapper 和 client identity 必须在实施 Phase 0 重新对齐当时的 OpenAI 第一方代码。

### 4.3 xAI/Grok

- 端点通过 `https://auth.x.ai/.well-known/openid-configuration` 解析，并校验 HTTPS + x.ai host。
- Device Code 按服务端 interval 轮询，正确处理 `authorization_pending`、`slow_down`、`access_denied`、`expired_token`。
- 原生 Grok store 是 registry，不得整体覆盖未知 scope；写入必须有 owner-only 权限、锁和原子替换。
- 第一方 `auth_provider_command` 可减少 refresh token 在多个进程复制；实施时优先验证该路径是否满足当前 FyAgent/Grok 版本矩阵。

### 4.4 OpenCode Desktop

- Desktop 自己通过 Provider Auth API 完成 OAuth/API Key 连接；内部 sidecar 的随机端口/密码不是公共控制面，FyAgent 不扫描或劫持它。
- 官方 credential store 由 provider ID 映射到 `oauth`、`api` 或 `wellknown` entry；修改时必须 read-modify-write、保留未知 provider、权限 `0600`。
- 直接投影 OAuth refresh token 时，OpenCode 会成为潜在刷新者，因此不得与 FyAgent/Codex 共享同一个 refresh lineage。

## 5. Refresh-token 所有权结论

### 5.1 必须区分两个对象

```text
ManagedIdentity
  issuer + subject/account/workspace + display metadata

CredentialSession
  one OAuth grant + one rotating refresh-token lineage + one purpose/consumer
```

同一个 `ManagedIdentity` 可以有多个 `CredentialSession`。UI 将它们聚合为一个账号，但 backend 不合并其 refresh token。

### 5.2 默认策略

| Consumer | 默认 session / refresh owner |
| --- | --- |
| FyAgent Local Proxy | 独立 session；FyAgent owner |
| Codex native | 独立 session；激活后 Codex native owner，FyAgent只在受控切换时同步最新 generation；若未来通过官方 API 保持 FyAgent owner，必须有独立证据 |
| OpenCode provider | 独立 session；OpenCode owner，或由 OpenCode 官方 Provider Auth 自己创建 |
| Grok Build | 优先独立 session + FyAgent token helper（FyAgent owner）；不支持时使用独立 native session + Grok owner |

禁止默认把一个 refresh token 同时写入多个 consumer。用户可看到“同一账号已连接多个软件”，但这不等于共享同一 credential lineage。

## 6. 许可证与供应链结论

- `cockpit-tools`：只做行为、架构、故障和 UX 参考；任何代码级复用必须先取得书面许可并完成 NOTICE/来源记录，否则禁止。
- OpenAI Codex / xAI Grok Build：按 Apache-2.0 要求保留 NOTICE/归因，复制或派生片段必须记录 exact source commit 与修改说明。
- OpenCode：MIT；若复用源码片段，保留版权与许可文本。
- 优先复用 FyAgent 已有 `reqwest`、`url`、`base64`、`uuid`、`sha2`、`subtle`、`zeroize`、Tokio 与原生 Keychain/Credential Manager adapter。
- 不为了“看起来通用”新增 OAuth 框架。只有实施前 dependency review 证明新的 crate 能显著减少安全敏感代码、支持 OpenAI 特殊 Device Flow/xAI discovery、跨平台、许可证和审计均通过时才引入，并锁定版本。

## 7. 尚需真实 HIL 证明的项目

这些项目不得在任务里假定为已验证：

1. 当前正式 Codex Desktop/CLI 在 macOS 与 Windows 上各 credential-store 模式的实际读取、刷新、热更新和进程缓存行为。
2. Codex native refresh 后 file/keyring 中的字段变化、refresh rotation 与 app-server reload/restart 要求。
3. 当前 Grok Build 正式版对 `auth_provider_command` 的完整桌面/CLI行为、外部 token 热更新和锁竞争。
4. OpenCode Desktop 在 macOS/Windows 当前 stable build 的实际数据目录、刷新写回、运行中 auth file 变化后的 reload/restart。
5. 中国大陆网络环境下 auth.openai.com、chatgpt.com、auth.x.ai 的可达性与失败文案；不得因此引入未授权镜像或代理。

未通过 HIL 的 consumer adapter保持 `unsupported` / `manual_action_required`，不能用 mock/源码阅读宣称生产可用。
