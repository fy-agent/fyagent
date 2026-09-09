# Grok 订阅跨 Agent 完整范围审查

日期：2026-09-08。审查目录为当前隔离 worktree，比较基线为 `origin/main`，另包含本次新增的生产/测试模块。原 checkout 不在写入范围。第二轮复核已完成，最终结论见文末；第一轮发现保留用于说明修复依据，其“待修”状态已被第二轮结论取代。

## 审查范围与证据

- 已读取当前任务 PRD、design、implement、check.jsonl 及列出的 Native/Renderer 契约。
- Native 主链：`commands/provider.rs` → `services/provider/managed_xai.rs` → 当前 Provider 锁及补偿事务 → `services/proxy.rs`。Codex 保存草稿后复用 `services/change_plan/service.rs` 的计划/应用准入和写入器。
- 授权主链：显式 overview identity → `ManagedAuthService::xai_proxy_account` → 绑定不含秘密的 legacy credential identity → `resolve_access_material`。已有 `purpose=proxy_upstream`、`consumer=fyagent_proxy`、`refresh_owner=fyagent` 边界保留。
- Renderer 主链：当前 Managed Auth overview → `XaiSubscriptionSection` 账号/模型选择 → strict port → 确认 → native 应用/保存 → Provider summary 与 Managed Auth 双回读。Codex 续接现有 Auth source workspace。
- 检查了新 `subscription_tests.rs` 的合成 vault、真实 Provider/Change Plan writer、真实 Proxy listener/router/转换器及假上游 HTTP 断言。这些证明隔离 native 链路，不证明真实订阅上游、额度扣除、安装 CLI 的完整行为或 Windows 运行。

## 第一轮 Findings（历史记录，已由第二轮结论取代）

### 1. Desktop 草稿错误回读会误封锁 Claude Code

- 文件：`src/pages/models/XaiSubscriptionSection.tsx`。
- 证据：Claude 面板可发 `app=claude-desktop`，但确认回调以 `props.app=claude` 获取 summary，再要求返回的 Desktop Provider ID 出现在 Claude summary 中。真实分区数据必然不匹配；原测试复用了不按 app 分区的 summary fixture，掩盖了这个问题。
- 主控决定：本期只暴露 Claude Code/Codex CLI，移除 Desktop 按钮和专属文案分支；native/DTO 保留明确的 Desktop draft 能力，不新增 Desktop summary port。
- 修复 owner：原 Renderer 实施者，待最终 diff/回归复核。

### 2. 卸载后的异常回读绕过父目标封锁

- 文件：`src/pages/models/XaiSubscriptionSection.tsx`。
- 证据：`confirmBind` 在 summary/selected-current 一致性检查之前执行 `if (!mounted.current) return`。切目标可卸载该 child；native 完成但回读不匹配时会跳过 `onUnconfirmed`，回到原目标仍可继续写。
- 修复要求：权威匹配检查和父目标封锁先执行；mounted 仅阻止本地 state 更新。增加 pending → unmount → mismatch 的回归。
- 修复 owner：原 Renderer 实施者，待最终 diff/回归复核。

### 3. 跨目标激活与失败清理可争用同一个 listener

- 文件：`src-tauri/src/services/proxy.rs`、`services/provider/mod.rs`。
- 证据：目标级写锁不同，两个目标都可能记录 `was_running=false`。A 启动服务、B 复用服务但尚未写 enabled 时，A 失败恢复的 `another_active=false` 可停止服务；B 后续仍可能完成配置并报告成功。只包住 `start()` 的锁不能保护整个激活/恢复事务。
- 主控接受修复：在既有 Proxy owner 以明确锁顺序序列化 managed 激活事务，覆盖 runtime snapshot、prepare、Provider 提交/补偿；补双目标并发回归。
- 补充：runtime 补偿必须回读 app/global 状态，不能只依据 UPDATE 返回成功报告已恢复。
- 修复 owner：原 Native 实施者，待最终 diff/回归复核。

### 4. 同模型多账号生成同名 Codex 来源

- 文件：`src-tauri/src/services/provider/managed_xai.rs`。
- 证据：Provider ID 按账号+模型区分，但 name 仅为 `Grok · model`；两个账号使用同模型时 Auth 来源列表无法凭名字辨认。保存后的继续操作正是按该 name 提示选择。
- 主控决定：新来源名包含安全、短的公开账号标签与稳定短标识；不输出秘密或完整 legacy ID。既有绑定只要 ID/目标/meta/settings 匹配，应保留既有 name，让账号显示名变化或用户改名不破坏幂等。定义/绑定不匹配仍拒绝覆盖。
- 修复 owner：原 Native 实施者，待多账号同模型和重绑定回归。

### 5. 订阅上游入口仍需来源核证及真实 URL 断言

- 文件：`src-tauri/src/proxy/providers/{mod,claude,codex}.rs` 等当前 xAI adapter owner。
- 证据：当前 OAuth inference 仍使用 `https://api.x.ai/v1`；已有研究提示 Grok CLI 订阅可能使用独立 `cli-chat-proxy.grok.com`。由主控安排 Luna 核对官方 CLI 来源，未经核证不得把普通 API 入口等同订阅入口。
- 当前假上游 seam 会替换最终 URL，现有成功测试本身不能识别错误的真实计费入口。修复后应在替换前断言实际生产上游 URL/协议所需的来源合同。
- 状态：产品主链的重要待定项；本报告不判定额度复用已完成。

## 第一轮已核对的保护边界

- 新绑定请求明确账号和模型；缺失授权/错误 purpose/错误 refresh owner 失败关闭，不使用默认账号代替。
- 公开绑定 DTO、计划与目标配置未包含上游 access/refresh token；新 vault 绑定不存在时不会退回 legacy JSON 内存账号。
- Codex 继续 plan/digest 的既有应用路径，不新增任意执行器；预览后撤销及实际请求撤销有新增回归。
- 同步目标使用既有 Provider 锁和文件恢复 owner；不直接修改原 checkout 或真实用户 Agent 文件。
- WorkBuddy 仍仅能借账号读取模型名单；Desktop native 仍是草稿。这两项不能宣称订阅 inference 已集成。

## 第一轮 Verification（历史记录）

- Lint / TypeCheck：等待本轮修复后的 reviewer 定点检查及主控最终 gate。
- Native：实施者报告旧定点过滤通过；新补回归、订阅入口与并发修复仍待最终测试。报告不把实施者自述替代 final gate。
- Renderer/browser：主控和 Renderer 实施者负责当前批次；最终数量及退出码待回读。
- 真实订阅账号、CLI smoke、Windows：本审查未执行，不宣称通过。

以上是修复前审查记录；当前状态以下面的第二轮结论为准。

## 第二轮结论

代码复核通过：两轮发现的六项明确问题均已修复并核对最终实现，没有剩余已确认的本轮代码阻断项。Reviewer 独立执行的 lint、TypeScript 检查与 31 项 Renderer/Port 定点测试通过。主控仍须完成全量门禁、完整浏览器/性能检查和 PR 交付，本报告不代替这些结果。

### Findings（fixed）

| 项目                                   | 最终修复与复核证据                                                                                                                                                                                                                                                                                                                                                                                                      |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Desktop 错误回读                       | `XaiSubscriptionSection` 删除 Desktop UI 入口和仅此入口使用的文案；pending request 只允许 Claude/Codex。Native/DTO 仍保留明确 draft 兼容，不新增 Desktop summary port。                                                                                                                                                                                                                                                 |
| 卸载后异常回读                         | summary/current 不一致时先调用父级 `onUnconfirmed`，再以 mounted 限制组件 state。Reviewer 执行 pending bind → unmount → mismatch 回归通过。                                                                                                                                                                                                                                                                             |
| 共享 listener 竞态及补偿读回           | `ProviderService` 在 app 锁后、所有 snapshots 前取得 managed activation guard，覆盖 prepare/commit/rollback；manual per-app takeover 也使用 app → activation → start 顺序。恢复后读回 target/global 配置及 listener，篡改不再冒充 restored。已复核实入口双目标门控测试（Change Plan 与 manual 两种入口）、端口冲突与 target/global trigger 篡改测试。                                                                   |
| 同模型多账号同名                       | 新 Provider 名包含过滤后的公开账号短标签、稳定 identity digest 短标识和模型；不使用秘密或完整 legacy ID。重绑定沿用已有 name，其他定义/绑定仍完整比较。已核对多账号同模型可区分、改名幂等、实际配置变更 conflict 回归。                                                                                                                                                                                                 |
| 订阅上游/协议错误                      | OAuth 固定到 `https://cli-chat-proxy.grok.com/v1/chat/completions`；API key 保留原 `api.x.ai` 路径。Claude 强制 Chat，Codex Responses→Chat 与 `ProxyChat` profile 一致。最后 native 阶段设置 CLI 标识头和最终模型头，覆盖客户端/自定义副本。测试替换 I/O 前断言实际 vendor HTTPS/host/path；双协议测试检查非流式结果、流式工具参数及结束事件，撤销后零新增上游请求。                                                    |
| 冲突官方 metadata 误分类（第二轮新增） | `resolve_codex_catalog_tool_profile` 原先先判断固定官方 ID/category；与 xAI metadata 冲突时会返回 NativeResponses。同一个 official helper 还控制 URL 与 native-auth passthrough。现 helper 明确排除 xAI，catalog xAI pin 优先。新增回归要求冲突行仍 subscription origin、XaiOAuth strategy、Chat/非 Anthropic、ProxyChat；去掉 xAI metadata 后正常官方保持 NativeResponses。Reviewer 已检查该 helper 的所有当前调用点。 |

来源与范围同步已核对：

- 订阅入口来源为 `research/xai-subscription-upstream-20260908.md` 的官方 Grok Build 固定提交 `72a61251fcffb464bcc687aeb5a998e5a98ec0c9` README L510–538，由 Luna 核验、主控抽查。报告中原有“当前 FyAgent 错误 API 入口”段落描述的是修复前基线。
- `get_xai_oauth_models` 不再向 API-key catalog 发送会话 token；验证所选 vault 凭据后提供官方示例 `grok-build`。UI 明确是模型选项、非账号 entitlement 名单，并标记实验性。
- WorkBuddy 订阅 picker 已移除；其原有 API 服务/密钥、模型发现及保存流程保留。Desktop 只保留 native draft，不宣称本轮 inference 支持。
- 新增保护边界已同步到 backend Managed Auth、Proxy Runtime、Change Plan 与 frontend Models spec；没有新增云服务、Docker、OAuth store 或任意执行器。
- 主控修订了浏览器测试 locator：按精确“Claude Code 模型配置”/“Codex 模型配置”父 region 选择订阅区域，并等待 Codex 按钮出现；避免过渡期命中退出中的旧目标。原两次 bind payload 和一次 apply 断言保留，没有新增 sleep/force 或放宽业务断言。

以上生产修复由原 Native/Renderer 实施者和主控在各自责任文件完成；Reviewer 协调发现并核对最终 diff，未并发覆盖他人代码。

### Findings（not fixed / 证据边界）

- 没有遗留已确认的本轮代码错误。
- 真实 Grok 账号的上游请求、实际额度归属/配额消费没有在本审查中验证。官方 CLI 文档和合成 HTTP 成功不能证明本机账号 entitlement；当前为明确标注的实验性兼容。
- 安装 CLI smoke、Windows native 未由本审查执行；不能从协议 fixture 或 macOS 编译推导通过。
- 全量 Rust/Renderer/contracts、完整浏览器和性能门禁、远端 CI/PR 状态由主控收尾；本报告不把尚未完成的层级写为 PASS。

### Verification

Reviewer 独立执行，均在最终 Renderer 修改之后：

| 检查                                                                                                                                    | 结果 | 工具回读                                  |
| --------------------------------------------------------------------------------------------------------------------------------------- | ---- | ----------------------------------------- |
| `mise run lint`                                                                                                                         | PASS | session 23558，exit 0                     |
| `mise run typecheck`                                                                                                                    | PASS | session 43150，exit 0                     |
| `mise run test:unit -- tests/renderer/pages/models/XaiSubscriptionSection.test.tsx tests/renderer/platform/xaiSubscriptionPort.test.ts` | PASS | session 90391，2 files / 31 tests，exit 0 |
| `git diff --check`                                                                                                                      | PASS | Reviewer 最终只读检查 exit 0              |

Native 实施者最终报告 `subscription_` 24/24（双入口并发、trigger unknown、双 stream/非 stream）和 `xai_oauth` 18/18 通过；rustfmt 与 diff check 通过。Reviewer 已核对对应最终测试实际路径，为避免并发 Cargo 没有重复运行；最终全量 Rust 结果由主控 evidence 记录，不用实施者自述替代全量 gate。

### 门禁后的最终局部复核

主控在全量门禁中修正两处机械问题，Reviewer 已读取最终源码及相对 `origin/main` 的差异：

- `proxy/forwarder.rs`：测试 seam 的 `is_some()` 加 `expect()` 改为 `if let Some(_fixture)`；`cfg(test)` 内直接使用匹配值。生产构建仍是 `Option<()> = None` 并进入原 AppHandle 授权路径，fixture 仍不进入生产；没有 lint allow 或行为扩展。
- `commands/agent_catalog.rs`：本次新增一个已注册/授权命令后，测试冻结数从 367 更新为 368，并显式断言包含 `bind_xai_managed_provider`。`allowed == registered` 和远端来源限制保持，变更没有放宽权限集合；该文件相对主线仅有这两处断言变化。

主控报告这两项修复后的 `check:backend` 完整重跑 exit 0：合计 3518 passed / 6 ignored，Clippy、Rust fmt、Rust check 均通过；frontend 已 1614 passed / 1 原有 skip，另 7 项 desktop mock 通过。本段明确记录主控回执，Reviewer 按协调要求未重复执行测试。此前核心审查结论保持通过，完整产品/真实账号及远端 PR 证据仍以主控最终 evidence 为准。

最终源码抽查 SHA-256（后续生产修改需复核相关部分；除最后一项外路径相对 `src-tauri/src/`）：

```text
6d96eb33da11ab4b9bb7fd905e2d57b2537835859abb398f9f32383593f32bfd  services/provider/managed_xai.rs
b5b180bd43b4c03a83f49e974e240c74b387c3e422a5ecb24d67d291610d055b  services/proxy.rs
9922cb6f6f2a2f4627688b1f352f9b11bd50523e580a8374c0fd076ea031b84e  proxy/providers/codex.rs
cb90c2d7bd3d9f92dab744df7c7f637c5c7d8f4621f4360bd479b206a5902b3c  proxy/providers/claude.rs
04c62afbb6756839301eef3f240dfa31de22cc1eb61c0b7830ca180058382f33  proxy/forwarder.rs
80c503aa51e42ed5b3aba5f3063158fcd064e1476ea631ed41d3af3eb7982fa2  services/managed_auth/subscription_tests.rs
b7f3b1918d96288994a53dd1ec3316c00c027553c284eb29beba43e41b577623  commands/agent_catalog.rs
a1c7ee77ed415ed070615b0c64a10d5e6157eccb469ddab9215e83ffd2ea11f1  src/pages/models/XaiSubscriptionSection.tsx
```
