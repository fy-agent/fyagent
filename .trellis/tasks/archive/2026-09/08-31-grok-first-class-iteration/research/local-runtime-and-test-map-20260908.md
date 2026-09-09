# 本地代理生命周期与测试复用地图

调查基线：固定主线 `2f264d2f89326601a33f610c72a9f0143306d066`（源码只读，未运行测试、未启动服务、未读取真实凭据）。以下行号均以该提交的 `git show <sha>:<path>` 为准。

## 结论

`xai_oauth` provider 的保存/切换不会单独启动本地代理，也不会自动接管 Claude/Codex 的 Live 配置。代理启动与 Live 接管是两个动作：`set_takeover_for_app(app, true)` 才会在未运行时启动代理并改写目标客户端配置；启用后切换 provider 走热切换，当前客户端仍指向本地地址。应用退出会恢复 Live 配置并停止监听，但保留“下次启动自动恢复”的 enabled 状态；用户显式关闭则恢复并清除状态。

## Provider xai_oauth 到运行时

- `Provider::is_xai_oauth` 只按 `meta.provider_type == "xai_oauth"` 识别（`src-tauri/src/provider.rs:90-111`）。Claude 与 Codex adapter 对该类型都返回固定 xAI API origin，并写入 `xai_oauth_placeholder` + `AuthStrategy::XaiOAuth`（`src-tauri/src/proxy/providers/claude.rs:490-497,703-713,774-779`；`src-tauri/src/proxy/providers/codex.rs:672-680,732-739`）。可编辑的 base URL/placeholder 不是实际 secret。
- 每个请求的真实 access token 在 `forwarder.rs:1740-1787` 按 `managed_account_id_for("xai_oauth")` 从 ManagedAuth 取得；失败是账号级 `AuthError`，`forwarder.rs:2692-2698` 将其标为不可通过别的 provider 掩盖的 non-retryable。实现者不能把 provider 切换当作“登录完成”或把 token 写进 Claude/Codex live 文件。
- `commands/provider.rs` 的一般 provider switch 最终调用 `ProviderService::switch`（约 `5483-5655`），正常模式写入客户端 Live 配置；若发现 backup/placeholder 接管状态则转到 `hot_switch_provider_inner`，不会恢复外部 endpoint。`commands/proxy.rs:277-306` 的 `switch_proxy_provider` 仅切换已运行代理的路由目标，它本身没有 `start()`。

## 启动、接管、重启与退出

- `services/proxy.rs:578-633` 的 `ProxyService::start` 开启全局 proxy flag、读取配置、绑定监听器并保存 `ProxyServer`；已经运行时直接返回当前信息。`proxy/server.rs:94-223` 用 `TcpListener` + Tokio task 接受连接，监听器状态由 `shutdown_tx`/`server_handle` 持有。
- `services/proxy.rs:791-911` 的 `set_takeover_for_app(app, true)` 是应用级入口：未运行先 `start()`（796-800），然后备份 Live、同步 token 到 DB、写入 loopback proxy URL/占位符并设置该 app 的 `enabled=true`。因此 xai_oauth 切换只有在调用这条接管路径后才会“让 Claude/Codex 使用本地代理”。
- `services/proxy.rs:914-975` 的关闭接管会恢复 Live、删除 backup、清除 enabled；最后一个 app 关闭时调用 `stop()`。`disable_takeover_for_app_sync`（977-1015）供无 Tokio runtime 的同步 Profile 路径使用：恢复文件并清状态，但刻意不停止正在运行的服务。
- `commands/proxy.rs:40-43` 的 `stop_proxy_with_restore` 调用 `stop_with_restore`。后者（`services/proxy.rs:1350-1394`）停止服务、恢复所有 Live、清 `live_takeover_active`/所有 app enabled、删除 backups、清健康状态；这是用户明确停止，下一次不会自动接管。
- 正常退出由 `cleanup_before_exit`（`lib.rs:2813-2852`）处理：有 backup/placeholder 时调用 `stop_with_restore_keep_state`，恢复 Live 但保留 enabled；无接管残留时只停止监听。窗口 `CloseRequested`（`lib.rs:987-1018`）若 `minimize_to_tray_on_close` 为 true 仅隐藏窗口，进程和代理继续运行；否则调用 `app_handle.exit(0)`，进入退出清理。
- 启动恢复在 `lib.rs:2879-2933`：读取 Claude/Codex/Gemini/GrokBuild 各 app 的 `proxy_config.enabled`，逐个调用 `set_takeover_for_app(true)`；失败时调用 false 清理状态。故“窗口关闭”可能只是隐藏，“真正退出/重启”会先恢复 Live，下一次启动又按 enabled 自动重新接管。`ProxyServer::stop`（`proxy/server.rs:225-255`）发送 oneshot 并最多等待 5 秒；超时只报告 `StopTimeout`，实现者需在验收中检查端口是否确实释放。

## 现有测试与可复用的合成 fixture

### 本地文件与 DB 隔离

- `src-tauri/tests/support.rs:6-24` 的 `ensure_test_home` 创建进程隔离临时 HOME，并设置 `FYAGENT_TEST_HOME`、`HOME`（Windows 另设 `USERPROFILE`）；`reset_test_fs`（26-53）清理 `.claude`、`.codex`、`.fyagent` 等目录；`test_mutex`（64-101）串行化全局 HOME/设置写入。
- `create_test_state` / `create_test_state_with_config`（`support.rs:103-118`）分别用 `Database::init()` 或迁移合成 `MultiAppConfig`。纯 DAO/路由单测可直接用 `Database::memory()`（`src-tauri/src/database/mod.rs:192-199`），避免磁盘 DB；涉及 live 文件路径应使用 support 的临时 HOME。
- provider switch/接管回归样例在 `src-tauri/tests/provider_service.rs:530-709`：合成 Codex OAuth JSON（`oauth-access`/`oauth-id`）、第三方 provider TOML、动态端口 `listen_port=0`，断言 switch、backup、loopback URL、`PROXY_MANAGED` 与 stop/restore。它证明了生命周期投影，但不是 HTTP 上游端到端测试。

### xAI OAuth 登录合成 issuer

`src-tauri/src/services/managed_auth/providers/xai.rs` 的测试已经提供最有价值的无秘密 OAuth fixture：

- `xai.rs:813-862` 的 `spawn_issuer` 在 `127.0.0.1:0` 启动 Axum issuer，合成 device-code、pending/granted token、`/oauth/token`，返回 JWT 形状 claims；`xai.rs:865-877` 用 `Database::memory()` + `MemorySecretBackend` + `tempdir` 构造 ManagedAuthService。
- `xai.rs:897-940` 的 device-code 测试通过 `XaiLoginHooks` 注入本地 endpoints，启动 login session、轮询到 Completed，并断言 snapshot 不泄露 device auth id、refresh token、access token 等。可复用此模式测试 refresh rotation、过期、slow_down、取消，不应调用真实 xAI。
- `xai.rs:42-91` 的 `start_xai_login` 只接受 `ManagedAuthProvider::Xai` + `DeviceCode`，对 `ConnectConsumer` 只允许 Grokbuild/FyagentProxy/Opencode；`xai.rs:254-323` 将 token 变成 credential identity，写入 secret backend（`RefreshOwner::Fyagent`），完成登录后仍不自动修改 consumer 文件（305-306）。Native 实施者应显式串联“login completed → 用户确认连接 → provider/consumer projection”。

### 本地 HTTP 双协议测试建议与当前缺口

固定主线的 proxy server 路由已经同时提供：Claude `POST /v1/messages`（`src-tauri/src/proxy/server.rs:296-298`）、OpenAI Chat/Responses（308-329）以及独立 GrokBuild Responses（330-355）。因此真实本地 HTTP 验收可按以下顺序构造：

1. 用 `ensure_test_home`/`reset_test_fs`、`Database::memory` 和合成 `Provider { meta.provider_type: "xai_oauth" }` 建 state；用 `listen_port=0` 启动 `ProxyService`，记录实际 loopback 端口。
2. 在不读取真实 secret 的前提下，为 forwarder 提供合成 ManagedAuth credential（access token 仅是测试字符串），或把上游请求目标注入本地 Axum `TcpListener` fixture；断言仅发 `Authorization`/provider headers 的合成值，测试日志禁止输出完整 token。
3. 用同一端口分别发送 Claude Messages JSON（`messages`、tools、stream）和 Codex Responses JSON（`input`、tools、stream），断言路由进入同一 xai_oauth provider、请求形状/错误映射/usage 与 SSE 完整性。再调用 `set_takeover_for_app(false)`，读取 Claude/Codex live 文件确认恢复，并确认端口停止。

当前固定主线没有发现一个已经把“启动 ProxyServer → 发本地 Claude + Codex HTTP → 伪造 xAI 上游 → 读回响应”的完整 integration test；现有 `provider_service` 主要验证文件投影，`forwarder` 内 xAI 测试主要验证错误分类，`proxy/server` 路由本身没有该端到端夹具。Native 实施者需要补这条测试，不能把 provider switch 单测或 ManagedAuth login 单测当作双协议可用性证明。

## 实施注意事项

1. UI 的 xai_oauth provider switch 应先确认代理已运行且目标 app 已 takeover；若产品动作要“一键登录后可用”，必须显式编排 login session、credential admission、provider selection、`set_takeover_for_app(true)`，不能只调用 `switch_proxy_provider`。
2. “窗口关闭”“真正退出”“手动 stop”三个行为必须分别验收：隐藏到托盘不释放服务；退出会恢复 Live 但保留 enabled；手动 `stop_with_restore` 恢复并清 enabled；重启随后按 enabled 再接管。
3. 两个下游协议应共用同一合成 account，但分别检查 Claude `/v1/messages` 与 Codex `/v1/responses`；xai_oauth 的 adapter 固定 xAI origin，无法靠 editable base URL 把真实请求改到本地 fixture，测试需要在 HTTP client/issuer/forwarder 层提供受控注入点。
4. 秘密 owner 是 ManagedAuth secret backend / credential row；consumer live 文件只放 loopback 地址与占位符。任何 stop/restore 失败都必须保留 backup 并报告恢复不确定，不能静默清理。
