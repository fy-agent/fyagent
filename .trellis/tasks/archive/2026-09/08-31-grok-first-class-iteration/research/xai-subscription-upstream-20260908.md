# xAI 官方订阅上游核验（2026-09-08）

## 结论

官方 Grok Build CLI 的会话登录（SuperGrok/CLI 订阅路径）与 xAI API key 是两条不同的上游路径。官方 CLI 文档给出的会话令牌请求目标是：

```text
POST https://cli-chat-proxy.grok.com/v1/chat/completions
Authorization: Bearer <~/.grok/auth.json 中的 token>
X-XAI-Token-Auth: xai-grok-cli
x-grok-model-override: grok-build
```

官方文档没有把该会话令牌路径描述为 `api.x.ai/v1/responses`，也没有承诺订阅 Bearer 可用于普通 API 端点。当前 FyAgent 固定把 `xai_oauth` 请求送往 `https://api.x.ai/v1`，因此若目标是复用 CLI/订阅额度，现有上游地址和协议至少存在明确阻断；不能仅凭 Bearer 格式相同推断可通用。

## 一手证据（官方仓库）

核验对象为官方仓库 [xai-org/grok-build](https://github.com/xai-org/grok-build)，`main` 当前提交（查询日）：`72a61251fcffb464bcc687aeb5a998e5a98ec0c9`，提交时间 `2026-09-01T22:20:33Z`。本机已安装的官方 CLI 元数据为 `grok 1.0.13 (5e9a58528b76)`；未把本机二进制版本冒充为仓库 main 版本，也未读取 auth.json 内容。

| 要点                    | 官方来源和精确证据                                                                                                                                                                                        | 能确认什么                                                                                                                                                                                                        | 边界                                                                                                |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 订阅/CLI 会话上游和协议 | [xai-grok-shell README](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-shell/README.md#using-authjson-for-api-access)，`Using auth.json for API Access` 小节（约 L510-L538）     | `POST https://cli-chat-proxy.grok.com/v1/chat/completions`；请求体为 `model: grok-build`、`messages`、可流式；官方列出 CLI 身份头和模型覆盖头                                                                     | 该小节没有 `/v1/responses` 示例或兼容承诺；模型/能力由代理路由决定                                  |
| 必要 headers            | 同上，表格（约 L523-L529）                                                                                                                                                                                | `Authorization: Bearer` 携带 auth.json token；`X-XAI-Token-Auth: xai-grok-cli` 标识 Grok CLI；`x-grok-model-override: grok-build` 选择路由（默认 grok-build 时可省略）                                            | 这是官方 CLI 请求契约，不是对第三方客户端的授权许可                                                 |
| token 有效期/刷新       | 同上（约 L530-L538）；[authentication guide](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md#browser-login)（约 L202-L241、L389-L423） | 浏览器登录到 grok.com/auth.x.ai 后写入 `~/.grok/auth.json`；后台自动刷新；README 说明 token 约 7 天过期并提示 `grok login`                                                                                        | 没有公开“把 token 发给 Claude/Codex”或跨客户端 grant；第三方必须自行承担会话失效、刷新和条款风险    |
| OAuth scopes            | [auth/config.rs](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-shell/src/auth/config.rs#L4-L26)                                                                                 | 默认 OAuth2 scope 含 `openid profile email offline_access grok-cli:access api:access conversations:read conversations:write workspaces:read workspaces:write`；注释明确 `grok-cli:access` 用于 API proxy requests | scope 不等于 API key 权限，也不能证明 token 可打普通 API host                                       |
| 外部 OIDC 可替换 proxy  | [authentication guide](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md#external-oidc-provider)（约 L266-L325）                         | `GROK_CLI_CHAT_PROXY_BASE_URL` 可将 CLI 指向自定义 proxy；默认 OIDC scope 含 `api:access`；token 由 CLI 以 Bearer 发给 xAI API                                                                                    | 这是 CLI 的配置扩展点，不是官方公开的 FyAgent 集成 API；自定义 proxy 的安全、额度和维护由部署者负责 |

官方认证指南还明确 credential precedence（约 L410-L423）：per-model API key/env key 优先，其次 active session token，最后 `XAI_API_KEY`。这支持“API key 与 session token 是不同凭据来源”的判断。

## 与 API key / API 余额的区别

官方 xAI API 文档的 [REST API reference](https://docs.x.ai/developers/rest-api-reference/inference)（更新 2026-09-02）把 Inference API 的 base URL 定为 `https://api.x.ai`，认证方式为 `Authorization: Bearer <xAI API key>`；Responses 和 Chat Completions 均属于该 API，路径分别是 `/v1/responses` 与 `/v1/chat/completions`。其 [Models reference](https://docs.x.ai/developers/rest-api-reference/inference/models) 说明 `/v1/models` 返回“对当前 API key 可用”的模型和价格信息；[Billing](https://docs.x.ai/console/billing) 说明 API consumption 从团队 prepaid credits 或月结额度扣除。

因此：

- `api.x.ai/v1` + `XAI_API_KEY` 是 API 团队余额/计费路径，可用 Responses、Chat Completions 和 API models catalog。
- `cli-chat-proxy.grok.com/v1/chat/completions` + auth.json session token 是官方 CLI 的代理路径，具备 `grok-cli:access`/CLI identity 约束；公开材料没有说明它消耗哪一项 API credit，也没有把 SuperGrok 订阅余额映射为 API key 余额。
- 即使两个请求都使用 `Authorization: Bearer`，issuer、scope、host、附加 headers 与额度归属仍不同。FyAgent 不应把 `/v1/models` 的 API key catalog 当作订阅代理的 catalog；`grok-build` 是官方代理示例中的虚拟/路由模型名。

## 对固定主线与 Native 实施的影响

固定主线 `2f264d2f89326601a33f610c72a9f0143306d066` 中，`src-tauri/src/proxy/providers/mod.rs:48` 定义 API base；`claude.rs:709-713`、`codex.rs:677-680` 对 `xai_oauth` 返回 `https://api.x.ai/v1`。这条路径与官方 CLI 订阅请求契约不一致。

最小可行适配建议：

1. 为 `xai_oauth` 单独保存 upstream origin（默认 `https://cli-chat-proxy.grok.com`），不要复用 API key 的 `api.x.ai` origin。
2. 订阅路径先实现官方已证明的 `POST /v1/chat/completions`，固定附加 `X-XAI-Token-Auth: xai-grok-cli` 和必要的 `x-grok-model-override`；下游 Claude Messages、Codex Responses/Chat 的转换要在 FyAgent 内完成。
3. 保留 API key provider 的 `api.x.ai/v1`；其 `/responses` 与 `/chat/completions` 由官方 API 文档证明。不要把订阅 token 直接送到 API `/responses`，也不要把官方 CLI proxy 当成稳定公开 SDK。
4. 登录实现需复用/实现 OAuth PKCE、auth.json 等价的安全存储和 refresh；只输出一次性授权 URL/本地 callback 状态，不回显 token。官方公开文档提供 CLI 登录行为，但没有面向 FyAgent 的 token exchange API，故这是集成工作而非现成官方接口。

## 仍未证实的事项（阻断点）

官方公开材料截至查询日没有给出：

- `cli-chat-proxy.grok.com` 的公开 OpenAPI、Responses 端点、非 CLI client identity 白名单或第三方 OAuth 客户端注册流程；
- SuperGrok 订阅额度与 proxy 请求的精确计费/配额模型，或 `/v1/models` 的订阅专属 catalog；
- 可供 FyAgent 直接调用的官方 generate-auth-url/exchange-code/refresh API。

所以当前证据足以确认“订阅 OAuth 不能按 API key 的 `api.x.ai/v1` 路径实现”，但不足以声称官方支持任意第三方客户端复用订阅额度。若 Native 方案要继续，必须以官方 CLI proxy 契约为实验性兼容目标，并把 upstream 变更、账号条款和回归验证列为产品阻断条件。
