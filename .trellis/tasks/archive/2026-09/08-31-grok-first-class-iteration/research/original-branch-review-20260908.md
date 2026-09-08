# 原 `feat/grok-first-class-iteration` 分支静态审查

审查基线：固定提交 `b8b15dbaf141f7c7fbd7816914fda59a07a2208a`，对比其父提交 `b8b15dbaf141f7c7fbd7816914fda59a07a2208a^`。只看该提交的 `git show`/差异，不以当前工作区内容为依据；未运行测试、未登录、未读取凭据。

## 结论

原分支已经把主要产品链路接上了：认证中心已有 xAI OAuth 账号后，V2 模型页能按目标创建 `xai_oauth` Provider；Claude Code / Claude Desktop 直接激活，Codex 建草稿并进入既有 Change Plan；token 留在 `xai_oauth_auth.json`，Provider 只保存 managed-account binding；代理层随后按请求取 token。它不是只改文案的原型。

但完成度是“源码链路 + 部分受控写入”，不是完整验收：

- Claude/Claude Desktop 绑定命令直接 `ProviderService::add(..., true)` 或 `switch`，绕过统一 Change Plan 的预览、漂移检查和 typed job；UI 的确认框只是绑定确认，不是目标 live 配置的完整计划。
- Codex 已有特殊 gate：可用的 `xai_oauth` managed account + 合法 Responses TOML 才允许生成 `NoNewCredentialMaterial` 计划；账户不可用则 fail closed。这是本分支新增的实际修复。
- WorkBuddy 只用 SuperGrok token 拉模型名单，保存仍要求自己的 API key/配置，明确不会把 OAuth token 写进 `models.json`；因此原分支没有完成“同一订阅直接运行 WorkBuddy”。
- 旧 `XaiOAuthManager` 仍是实际 token authority/回退路径；新增 bind 只检查磁盘账户存在且 `requires_reauth=false`，不在绑定阶段验证 token refresh 或 entitlement。真实请求失败时才会暴露问题。

## 已完成的实际链路

### Claude Code / Claude Desktop

`src-tauri/src/commands/provider.rs:463-616` 新增 `BindXaiManagedRequest` 与 `bind_xai_managed_provider`：只允许 `claude`、`claude-desktop`、`codex`；取认证中心默认或指定账号；生成 `provider_type=xai_oauth`、`AuthBindingSource::ManagedAccount`、`auth_provider=xai_oauth` 的 Provider。Claude 配置写入 xAI base/model 环境字段，Desktop 额外写入 Proxy mode 与模型 route。

对 Claude/ Desktop，`activate = true`，分支在 `:589-600` 直接保存并激活；这证明能落到已有 Provider/代理执行面，但也形成下面的 Change Plan 缺口。

### Codex

同一命令在 `:588-603` 对 Codex 使用 `add_draft`，不直接切当前 Provider；前端 `src/v2/pages/models/Page.tsx:1347-1406` 收到结果后把 Provider 设为 preferred target，`ChangePlanWorkspace` 自动生成现有切换计划（`Page.tsx:1655-1656`）。

`src-tauri/src/services/change_plan/service.rs` 的差异在 `:1585-1680`：先识别可用 xAI managed account，再校验 Provider 的 `settings_config.config` 是合法 TOML，才返回 `NoNewCredentialMaterial`；否则返回 `SecretDependencyUnavailable`。这是原分支对 v0.4.4 既有 Codex gate 的最小放行形状。

### 代理与凭据

`src-tauri/src/proxy/providers/xai_oauth_auth.rs:220-235` 新增 token-free `stored_account_is_usable`，只检查账号存在且未标记 `requires_reauth`。实际 refresh/访问令牌仍由原 `XaiOAuthManager` 管理；Provider 不携带 refresh token。`src/v2/shared/platform/tauri/feature-ports/models.ts:542-547` 注册绑定 IPC，`src-tauri/src/lib.rs:1876` 注册命令。

### WorkBuddy

`src/v2/pages/models/Page.tsx:308-345` 的 `fetchXaiManagedModels` 确实使用已登录账号读取模型名单并默认填入 xAI base URL，但提示明确说“保存仍走 WorkBuddy 自己的预览；不会把刷新令牌写进 models.json”。随后 `:463-505` 的保存请求仍由 `buildSaveRequest` 生成 API-key/无 key 配置，不能据此称为订阅额度运行链路。

## 具体缺陷与风险

| 问题                                            | 固定提交证据                                                                            | 影响                                                                                                                                                                                                                                                                          |
| ----------------------------------------------- | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude/ Desktop 直接 live 激活                  | `src-tauri/src/commands/provider.rs:589-600`                                            | 绕过统一 Change Plan；没有目标文件 revision/digest 预览，也没有 typed apply job 的统一回读语义。已有 Provider 写入器可能保证自身原子性，但不等于本迭代合同的先看→确认→写入→回读。                                                                                             |
| “已绑定”与“已可用”混淆                          | `Page.tsx:1372-1406`                                                                    | 前端只 refetch Provider summary/currentId；若当前已切换但代理首次 refresh/上游 entitlement 尚未验证，仍可显示已绑定/待确认，而不是请求级可用。                                                                                                                                |
| 绑定阶段只做弱 admission                        | `xai_oauth_auth.rs:220-235, 783-787`                                                    | 过期、撤销、tier 不允许的 token 只要未标 `requires_reauth` 就能生成 Provider；应由后续 refresh/目标最小请求把状态分层，不把文件存在当登录/额度成功。                                                                                                                          |
| WorkBuddy OAuth 断链                            | `Page.tsx:308-345, 463-505`                                                             | 可用 SuperGrok 拉名单，但不能让 WorkBuddy 运行时使用同一订阅；目标要求若包含 WorkBuddy，这一项仍未完成。                                                                                                                                                                      |
| Claude 配置依赖外部路由开关                     | `provider.rs:503-516`                                                                   | 生成的是 `ANTHROPIC_BASE_URL=https://api.x.ai/v1`，没有在 bind 命令中启动/确认 Claude 本地 route takeover；若用户没有打开本地路由，Claude 可能直连 xAI 或失败。目标链路依赖既有代理状态，不能只看 Provider 已保存。                                                           |
| 主线仍需保留的 OAuth manager/forwarder 兼容路径 | `xai_oauth_auth.rs:380-430`（父分支已有）与新 bind helper                               | 原分支新增 API 只是 binding façade，没有替换 `XaiOAuthManager` 的 refresh/forwarder 兼容职责；合入 v0.4.4 时若只移植新命令而漏掉主线仍调用的 manager/forwarder 回退，会破坏已有 OAuth。Provider preset、JSON 文件存在性判断和旧 UI 不应取代当前 Managed Auth/SecretRef 入口。 |
| 旧认证中心以动态 legacy host 挂回 V2            | `src/index.html:25-29`、`src/legacy-auth-boot.ts:1-4`、`src/legacy-auth-host.tsx:52-69` | 这是原分支的历史兼容挂载，不是 v0.4.4 当前入口合同。当前入口是 `src/pages/auth` 的 Managed Auth 与 SecretRef vault；retired Provider forms/Settings auth tab 及 legacy mutations 应由当前入口替代。仅保留主线明确需要的 legacy manager/forwarder 只读或转发兼容职责。         |

本分支没有发现把 access/refresh token 序列化进 Provider JSON、Change Plan 或新 UI 的证据；新增测试字符串断言也明确检查 payload 不含 `refresh`、`ANTHROPIC_AUTH_TOKEN`、`OPENAI_API_KEY`（`provider.rs` 测试约 `:1710-1755`）。因此凭据泄露风险主要在错误的后续移植/日志，而不是该提交的 Provider payload。

## 0.4.4 必须保留或重新接入的最小修改

1. 保留 `bind_xai_managed_provider` 的三目标白名单、Provider IDs、`xai_oauth` binding 和不携带 token 的 Provider 形状；它是把认证中心账号投向 Claude/Codex 的最小桥。
2. 保留 `xai_oauth_managed_account_is_ready` + `prove_xai_oauth_switch_shape` 的 Codex gate 放行逻辑，并继续让无账号/坏 TOML/撤销账号返回 `SecretDependencyUnavailable`。
3. 将原分支的 V2 → 认证中心 handoff（`legacy-auth-boot.ts`、`legacy-auth-host.tsx`、`auth-center-handoff.ts`）替换为 v0.4.4 `src/pages/auth` 的 Managed Auth 入口、SecretRef vault 与对应 feature port；不要把旧动态挂载或 retired Provider forms/Settings auth tab 当作当前合同，也不要以重做第二套 OAuth UI 替代现有入口。
4. Claude/ Desktop 应接入与 Codex 等价的目标计划或明确记录“绑定后立即激活”的不同合同；最小安全要求是写入前检查目标 revision、失败不谎报、写入后回读当前 Provider/代理开关。
5. 代理侧必须保留旧 `XaiOAuthManager` 的 refresh lock、CAS/rotation、失败 reauth 标记和 forwarder fallback；新增 bind 不得创建第二个 token store。
6. WorkBuddy 若暂不支持 OAuth 运行时，应保留当前“只能用 SuperGrok 拉名单、保存不写 token”的明确文案，不把它标成已完成；若产品范围只收 Claude/Codex，则应在 UI/合同中显式缩小目标。

## 最小验证点（供主控安排，不在本轮执行）

- 单元/静态：三目标白名单；Provider payload 无 refresh/access/API key；Codex gate 对可用/撤销/坏 TOML 三态；旧 OAuth manager/forwarder 回退仍可编译注册。
- 目标写入：Claude、Desktop、Codex 各自独立 preview → confirm → apply/readback；一家失败不能改变另外两家状态。
- 运行链：同一 managed account 的代理请求触发 access-token refresh/CAS，Claude Messages 与 Codex Responses 均留下 route/provider 证据；不要只以 Provider currentId 代替目标请求成功。
- 状态层：认证成功、token 可 refresh、模型可见、quota/entitlement 可见、目标请求成功分别记录；403/invalid grant 应进入 reauth/blocked，而不是绿色成功。
- 兼容：v0.4.4 Managed Auth/SecretRef vault 入口、主线仍承担转发兼容的 OAuth manager/forwarder、已有普通 API-key Provider、Grok native handoff 均保持可达；旧 Auth Center 动态挂载和 retired Provider forms 不作为必须保留路径。不得把 Grok native login 误写入 Codex/Claude。

以上是固定原分支的 source review，不是当前主分支、安装包、真实账号、双机 HIL 或生产验收结论。

## 主控复核补充（当前设计的裁决）

- 上文描述的旧代码路径不意味着已形成可调用闭环。固定主线的普通 Provider switch 不自动启动 Proxy，必须补 start/takeover；详见 local-runtime-and-test-map-20260908.md。
- Claude 未使用 Codex Change Plan 本身不构成缺陷：当前合同允许复用其现有 Provider/Proxy 保存事务，不应为追求形式一致再造执行器。实际需修复的是缺少可靠的写前校验、补偿、启动与回读。
- 原固定 Provider ID 不能无条件保留：同名记录可能属于其他账号/来源。当前设计要求账号/目标隔离和既有绑定验证，防止覆盖。
- 原 JSON 准入须改为当前 vault/SecretRef 能力判断；保留 manager/forwarder 仅指主线仍需的兼容职责，不恢复已禁用的授权变更接口。
- 本次必须完成 Claude Code/Codex。Desktop 仅在其已有应用路径确认结果，草稿不称为激活；WorkBuddy 模型名单不称为订阅运行成功。
