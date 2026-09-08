# PRD：简化 Codex 官方账号与第三方 API 切换

## 1. 目标

移除 Codex Managed Auth 对 matching-host HIL 的生产硬门控，并按 Codex 的
真实文件模型实现最小切换：

1. 官方账号 A→B：只交换完整 `auth.json`；
2. 第三方 API→当前已保存官方账号：只在需要时把 Provider route 切回
   `openai`；
3. 第三方 API→另一个官方账号：交换 `auth.json`，再复用现有 Provider
   owner 切回 `openai`；
4. 官方→第三方 API：继续走现有 Provider 页面和 config-only 路径，
   `auth.json` 保持不变；
5. “已连接”只由 live 文件身份和 Provider 读回证明，不由 FyAgent 内部存在
   credential 推断。

本任务当前只完成调研和 Trellis 规划，状态保持 `planning`。收到明确实施指令并
执行 `task.py start` 前，不修改产品代码。

## 2. 已确认事实

### 2.1 Codex 的两个正交状态

Codex 官方登录和请求路由不是同一件事：

- 官方登录凭据：file store 下位于 `$CODEX_HOME/auth.json`；
- 请求 Provider：由 `config.toml` 顶层 `model_provider` 决定，缺失时默认
  `openai`。

因此“换官方账号”和“换 Provider”必须分开判断，不能每次都同时重写两个文件。

### 2.2 有效 file store

固定的 OpenAI Codex 上游源码表明：

| 配置观察 | 有效 store | 本任务行为 |
| --- | --- | --- |
| 缺失 / unset | `file`（官方默认） | 允许投影；不补写配置键 |
| 显式 `file` | `file` | 允许投影 |
| 显式 `keyring` | keyring | fail closed；不写 `auth.json` |
| 显式 `auto` | 运行时不确定 | fail closed；不猜测 backend |
| 显式 `ephemeral` | 不持久 | fail closed |
| 非法/未来未知值 | 未知 | fail closed |

FyAgent 不静默把用户明确选择的 keyring/auto/ephemeral 改成 file。

### 2.3 当前 FyAgent 已有能力

- OpenAI OAuth、identity claim、SecretRef、generation/refresh-owner CAS；
- Codex config/auth 路径、TOML patch、原子写和局部回滚；
- 第三方 Provider config-only 投影，正常情况下保留官方 `auth.json`；
- `ProviderService` 的 Codex mutation guard、current backfill、proxy policy、
  DB/device current、live config 与 MCP；
- OpenCode consumer 的 bounded read、revision、writer lock、`0600`、readback
  与 rollback 模式；
- Managed Auth direct action 的 expected revision 与闭集 action；
- Codex Desktop 的可信 restart 协调。

本任务应复用这些 owner，不新增第三方运行时依赖、第二套 Provider writer、第二套
secret store 或新的 Change Plan public operation。

## 3. 当前实现的问题

1. `CODEX_FILE_PROJECTION_PRODUCTION_ENABLED=false` 把官方支持、可自动验证的
   file-mode 能力整体关闭；
2. gate 后仍没有真实 Codex auth writer，只会写 connection metadata；
3. credential 存在可能被显示为 connected，即使 Codex 没有使用该账号；
4. `switch_to_official` 已有 wire/UI 表达，但 Codex 后端不广告、不执行；
5. 缺失 `cli_auth_credentials_store` 被错误当成不可投影，而官方默认是 file；
6. 缺失 `model_provider` 被错误当成未知，而官方默认是 openai；
7. 当前 Managed OAuth bundle 为适配 Windows 单条 secret 2560-byte 上限，可能
   省略 ID/access token，不能直接假装成完整 `auth.json`；
8. Codex 会自行刷新并回写活动 `auth.json`，切走前若不读取 live 文件，会丢失
   最新 refresh lineage；
9. 历史 live 状态仍可能是 API-key-only `auth.json`，覆盖前必须由现有
   Provider owner证明第三方 key 已回填。

原实现复杂在错误的位置：用 HIL blanket gate 和模糊状态阻止所有用户；真正需要
保护的只是完整凭据、最小文件差异、并发、读回与可恢复写入。

## 4. 最小差异矩阵

实现必须先观察 live account 与 Provider，再只写发生变化的部分：

| 当前状态 | 目标 | 必要写入 |
| --- | --- | --- |
| 官方账号 A + official route | A | no-op；只刷新 overview |
| 官方账号 A + official route | B | 仅原子替换 `auth.json` |
| 官方账号 A + third-party route | A | 仅复用 ProviderService 切 official |
| 官方账号 A + third-party route | B | 替换 `auth.json`，再切 official |
| 缺失官方 auth + third-party route | B | 写 B 的 `auth.json`，再切 official |
| 官方 route | 第三方 Provider X | 现有 Provider 流程；`auth.json` 字节不变 |
| 第三方 X | 第三方 Y | 现有 Provider 流程；`auth.json` 字节不变 |

不得为了“统一流程”在无变化时刷新 token、重写 TOML、重写 auth、更新 DB current
或要求重启。

## 5. 用户故事

### US-1：连接已保存官方账号

用户选择一个已保存 OpenAI 账号后，FyAgent 把完整官方 ChatGPT auth 投影到
Codex，并以 live identity 读回作为结果。

### US-2：官方账号 A/B 切换

用户从 A 切到 B 时，FyAgent 只交换 auth 文件；不触碰 Provider 定义、模型、
MCP 或第三方 API 配置。

### US-3：第三方 API 切回官方

如果 live auth 已经是所选官方账号，只切 Provider；如果账号也不同，再先交换
auth。第三方 Provider 定义和 API key 保留，之后仍可切回。

### US-4：官方切第三方后保留会话

现有 Provider config-only 路径不得删除或覆盖官方 auth；切回官方时通常无需重新
浏览器登录。

### US-5：状态真实

账号仅保存但未投影时，UI 明确显示“已保存、Codex 尚未使用”；不能显示 connected。

### US-6：不支持的 store 不被静默降级

显式 keyring/auto/ephemeral 用户得到准确说明和手工配置指引；FyAgent 不擅自改
安全策略。

## 6. 功能需求

### FR-01：移除错误 HIL 门控

- 删除 blanket production gate 及其“matching-host HIL 才能启用”语义；
- HIL 保留为平台/版本 smoke evidence，不决定 action availability；
- capability 由 effective store、完整凭据、identity、revision、policy、写后读回
  和恢复能力决定。

### FR-02：修正 effective defaults

- unset `cli_auth_credentials_store` → effective file；
- missing `model_provider` → effective openai；
- 使用固定上游 commit fixture/compatibility test 保护这两个默认值；
- 显式非 file store 零 auth 写入。

### FR-03：完整第一方 auth 文档

- 使用 research 固定的 `AuthDotJson`/`TokenData` 兼容子集；
- `auth_mode=chatgpt`；
- 目标文档包含身份一致的 ID token、access token、refresh token、account ID 和
  `last_refresh`；
- 不混入 API key、PAT、Bedrock 或其它账号的 agent identity；
- 未知上游字段读取可容忍，跨账号写入不盲目透传；
- 缺少完整材料时返回 `requires_reauth`，不写残缺文件。

### FR-04：最小凭据物化

- 新 OAuth grant 完整时直接使用，不重复 refresh；
- 历史 bundle 完整且身份匹配时直接使用；
- 只有 bundle 缺少必要 token 或 token 已不可用时，才复用现有 OpenAI refresh；
- refresh 返回缺失字段时，只能保留仍有效且身份匹配的旧字段；仍不完整则 reauth；
- 不新增 OAuth client，不把 token 送到 renderer/log/event；
- 不提高全局 secret 上限，不新增明文账号快照；若实现证明当前 SecretRef 无法满足
  无重复登录切换，必须先回到 planning 评审“基于现有 SecretService 的分片组件
  secret”，不能私建 vault。

### FR-05：活动账号对账

- 覆盖 live auth 前 bounded-read 当前完整文件；
- 当前是受管官方账号时，把 Codex 轮换后的 token 通过 generation/owner CAS 回写；
- 只有唯一身份匹配时才回写；未知官方账号不自动 admission、不静默覆盖；
- 一个 refresh lineage 任意时刻只有 Codex 或 FyAgent 一个 owner；
- 对账失败时零覆盖。

### FR-06：窄 auth 文件交换器

- 放在 Managed Auth Codex consumer / `codex_config` 现有 owner 中；
- 复用现有 atomic write，不建立第二套 tempfile/rename 框架；
- 共享进程内 writer lock；
- 写前 expected revision；
- bounded read、exact preimage、Unix `0600`、写后 identity/readback；
- 失败只在 revision 仍等于本次写入时回滚；检测外部变化立即停止覆盖；
- 纯账号 A→B 不触碰 `config.toml`、Provider DB/device current 或 MCP。

### FR-07：Provider 变化只复用现有 owner

- 本任务不新增 Change Plan public operation/resource/job UI；
- Managed Auth command 可同时取得 ManagedAuthState 与 AppState；
- 所有组合动作在现有 Codex Provider mutation guard 下执行；
- route 需要变化时调用 `ProviderService` 的 crate-private lock-held seam；
- 不复制 current backfill、proxy takeover policy、DB/device current、live writer、
  stale-key cleanup 或 MCP；
- 当前 live auth 是 legacy API-key-only 时，必须先由现有 Provider owner证明 key 已
  回填，否则不覆盖；
- auth 先就绪，再切 official route，避免 official route 指向缺失/错误账号；
- Provider switch 失败时按 auth revision 尝试恢复 preimage；外部已改则返回
  external change。

### FR-08：现有第三方路径保持不变

- 官方→第三方和第三方→第三方继续使用现有 Provider 页面/Change Plan；
- 切换前后 `auth.json` 字节相等；
- 第三方 API key 保持 Provider-owned config/storage；
- Auth 页面不复制 API key、endpoint、model 或 Provider CRUD 表单。

### FR-09：状态与动作

- connected 只来自 live ChatGPT identity 与 connection credential 匹配；
- ready SecretRef 但 live 未使用 → saved-not-projected/disconnected；
- live official account 匹配、route third-party → official session preserved；
- 目标文件已写但运行进程可能缓存旧值 → pending restart；
- invalid/unknown auth、unknown account、revision drift、owner conflict、reauth 分别使用
  闭集 reason；
- `switch_to_official` 只使用当前 connection 明确绑定账号，不隐式任选默认账号；
- mutation 后前端刷新 authoritative overview，不 optimistic patch 成功。

### FR-10：重启复用

- Managed Auth 只返回 pending restart/capability；
- 复用现有 Codex Desktop 可信 restart coordinator；
- 不按进程名 kill、不猜安装路径；
- 重启后重新读回 identity/provider，再清除 pending。

## 7. 验收标准

### 最小写入

- [ ] A→A 是 no-op：auth/config/Provider/DB/device revision 均不变。
- [ ] A→B 只有 auth revision 变化，config/Provider/DB/device/MCP 不变。
- [ ] third-party+A→official+A 只有 Provider route/current 变化，auth 字节不变。
- [ ] third-party+A→official+B 同时只改变目标 auth 与必要 Provider 状态。
- [ ] official→third-party 和 third-party→third-party 的 auth 字节不变。

### 安全与真实状态

- [ ] unset 与 explicit file 可用；explicit auto/keyring/ephemeral/unknown 零 auth 写入。
- [ ] missing model_provider 按 official/openai 观察。
- [ ] 只有完整、身份匹配的 auth 文档能落盘。
- [ ] 当前 native refresh 在覆盖前被对账；未知账号不覆盖。
- [ ] legacy third-party API key 未证明回填时不覆盖。
- [ ] stale revision、并发切换、permission/readback/rollback failure 有确定结果。
- [ ] 新 auth 文件 Unix `0600`，token/SecretRef/path/raw JSON 不进 DTO、日志、DOM。
- [ ] 仅 vault credential 不会显示 connected。

### 复用边界

- [ ] 无新增 Rust/npm 依赖。
- [ ] 无新 Change Plan operation/resource/contract 变更。
- [ ] 无第二套 Provider CRUD/writer、OAuth client、secret backend、restart manager。
- [ ] Provider route 变化复用现有 mutation guard/backfill/policy/current/live/MCP。
- [ ] auth 文件原语优先复用 `codex_config` 与 OpenCode consumer 已有模式。

### 验证

- [ ] 单元/集成测试覆盖最小差异矩阵、token rotation、identity mismatch、CAS 冲突、
  legacy API key、外部改写、0600、rollback 和 secret redaction。
- [ ] V2 测试覆盖 saved-not-projected、switch-to-official、pending restart、reauth、
  unsupported store、external/recovery。
- [ ] HIL 可记录 macOS/Windows CLI/Desktop smoke，但缺少 HIL 不再关闭 file-mode 能力。
- [ ] 归档前按 Trellis 更新 Spec 并删除过时 blanket gate 文案。

## 8. 非目标

- 直接读写 Codex 私有 keyring/secrets backend；
- 探测 `auto` 当前实际命中的 backend；
- 静默把显式非 file store 改成 file；
- per-process 多 Codex 身份隔离或同时运行不同账号；
- 自动按额度轮换账号；
- 新建第三方 Provider/API key 管理；
- 新建 Change Plan operation；
- 新建 app-owned 明文或加密 vault；
- 修改 Grok Build/OpenCode 的独立门控策略。

## 9. 停止条件

实施中出现以下任一事实，必须回到 planning：

- 上游 AuthDotJson/default/refresh/store 合约发生实质变化；
- 当前 SecretRef 无法在不重复登录的前提下保留必要材料；
- ProviderService 无法提供不复制逻辑的 lock-held/backfill seam；
- legacy API key 无法在覆盖前证明可恢复；
- 需要新依赖、第二套 writer/store、renderer secret/path input；
- live authority 无法把中间态分类为目标、基线或可安全重试状态。

## 10. 规划结论

原方案确实把能力设计得过重：HIL 被当作运行时总开关，账号与 Provider 被绑定成
统一大事务。修正后，核心就是“观察差异后交换完整 `auth.json`”；只有 route 真正
变化时才调用现有 Provider owner。必要的原子写、身份读回、token rotation 与
revision rollback 保留，因为它们是安全交换凭据文件的最小边界，而不是额外产品
控制面。
