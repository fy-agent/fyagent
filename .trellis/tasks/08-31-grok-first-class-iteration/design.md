# 技术方案：复用当前授权和内置转发

## 基线与 review

新分支 `codex/grok-auth-reuse-completion` 从原分支 `b8b15dba` 建立，隔离目录 `~/.codex/worktrees/grok-auth-reuse-20260908/fyagent`。整合 `origin/main` 的 0.4.4 提交 `2f264d2f89326601a33f610c72a9f0143306d066`。保留主线历史和原分支；原提交标题不符合当前 CI，提交前只在新分支创建规范标题副本并验证同树/同父提交，原远端分支不改写。旧分支落后主线 236 个提交，前端目录迁移和 Managed Auth 更新造成冲突。

原分支已实现绑定命令、Codex 准入和页面入口，但读取旧 JSON 账号、复活旧认证 UI、固定模型与 Provider ID 等逻辑不能直接带入新版。原始 review 见 `research/original-branch-review-20260908.md`，旧计划留档见 `research/original-plan-20260831.md`。本设计覆盖旧父任务“父任务不写代码/只能旧认证中心”的限制。

## 最小行为缺口与责任

已有账号 → 安全选择账号/模型 → 保存目标 Provider → 启动/采用本机转发 → 目标文件回读 → 实际请求解析授权。缺口位于现有 Provider、Change Plan 和 Proxy 的连接处，不靠 renderer 写秘密或再建服务填补。

```text
现有 Managed Auth 页面 / vault
  -> 不含秘密的账号选择
  -> 当前模型/Agent 页面
  -> 受限绑定 IPC
  -> Provider 服务 + 现有 Codex Change Plan
  -> 本机 ProxyService / 转发器
  -> Managed Auth resolve_access_material（唯一刷新者）
  -> Grok 订阅上游
```

## Native 合同

2026-09-08 独立复核补充：原有 xAI OAuth adapter 把推理送往 `api.x.ai/v1`，不能据此认定使用订阅额度。已核对官方 Grok CLI 固定提交 `72a61251fcffb464bcc687aeb5a998e5a98ec0c9`，依据见 `research/xai-subscription-upstream-20260908.md`。本次必须改用 `https://cli-chat-proxy.grok.com/v1/chat/completions`，后端设置 `X-XAI-Token-Auth: xai-grok-cli` 及 `x-grok-model-override: <最终所选模型>`，复用已有 Chat 协议转换。模型路由实际由请求头决定，不能透传客户端任意覆盖。现有 OAuth scope 已含 `grok-cli:access`，不重做登录。测试 seam 替换网络地址前断言真实 vendor host/path，并覆盖两种下游协议的流式工具调用。

普通 API key 来源仍使用 `api.x.ai`；订阅选项不访问该站的 API-key 模型目录。Native 经过账号资格检查后返回官方已文档化的 `grok-build` 路由建议，页面明确它不是账号 entitlement 名单，用户仍需选择或手动输入。这是实验性兼容接入；真实额度消费仍需要真实账号和上游回执。

- 保留 `bind_xai_managed_provider` 这一闭合用途；命令层只做 DTO/state/error 映射，Provider 构建、合法性、保存/激活交给服务 owner。
- 绑定依据当前 Managed Auth 元数据/凭据能力，不以旧 `xai_oauth_auth.json` 是否存在判断。仅接受可用于 `proxy_upstream` 且由 FyAgent 管理的 xAI 授权；不取消 purpose/refresh-owner 隔离。
- 请求明确选择 `accountId` 和 `modelId`，目标限定受支持 app。新增命令尚未发布，可收紧为空值失败；renderer 与 Rust DTO 同步。后端重新验证账号可用性，不能信任页面已检查。
- 保留原结果 `providerId/providerName/app/alreadyBound/activated`；如需警告/恢复信息，使用现有 mutation/result 合同并通知前端实施者同步。`activated` 只能来自完成的目标写入/回读，不能由数据库存在推断。
- 不覆盖无关 Provider：账号/目标范围的稳定身份或受验证的已有绑定；身份/绑定不符拒绝，保存失败使用现有事务和补偿。不在新 command 内复制 SQL/文件/网络逻辑。
- 多账号使用相同模型时，保存来源名称必须可辨认账号；使用 vault 公开身份的短标签及必要的稳定短标识，不显示秘密或完整凭据 ID。同一绑定的 ID、目标、元数据和设置匹配时保留已保存名称，不能因单独改名破坏幂等。
- Claude 复用既有 Provider 配置应用/Proxy 接管，Codex 保持 draft + 既有 Change Plan，不扩建第四种执行器。确保 Codex capability 放行只针对已验证托管 xAI 形状，撤销/过期在 apply 边界重新检查。
- 应用后必须是正确本机入口；订阅上游由现有认证/转发 owner 决定，不以普通 xAI API key 入口的成功替代。
- 只管理目标 Agent 的配置，保留官方原生登录和无关配置。复用备份/序列化写锁/恢复。不得复制上游 token 到目标 auth.json、日志、计划或 renderer。
- per-app 写锁外还需保护共享监听器的激活事务：统一 Proxy owner 持有固定顺序的 guard，覆盖运行态快照、启动、提交与失败补偿，避免一个目标失败时停止另一个目标正在采用的监听器。补偿必须回读 app/global 状态。

## Renderer 合同

- 延续主线 `src/` 单入口与当前 `src/pages/auth` Managed Auth 页面。删除原分支新增的 legacy 动态挂载，不恢复已退役 Settings/Provider UI。
- 原投放能力接到现有模型页/Agent 页面；复用管理账号 overview、query invalidation、导航和 Change Plan workspace。已有账号可选择，失效/不可用状态不显示可绑定。
- 选择模型后绑定；使用后端文档路由建议或明确输入，不把建议当作订阅 entitlement 证据。
- Codex 绑定后的结果是草稿，继续预览/确认；Claude 应用后显示配置/运行依赖，不声称真实额度已验证。当前仅展示 Claude Code/Codex 投放入口；Claude Desktop 的后台草稿能力保留但不展示未接通的应用入口。
- WorkBuddy 保留主线 API 服务/密钥、模型发现和保存流程；不展示订阅入口，CLI 路由建议不应用于它的 API 模型配置。不引导把 refresh token 粘贴为 API key，不降低原生 Grok 官方登录/ChatGPT/其他 Agent 的主线行为。

## 生命周期与测试隔离

优先复用现有单例 ProxyService、恢复和托盘/退出行为。固定主线实际行为：普通 Provider switch 不会启动转发，set_takeover_for_app(true) 才启动并接管；真正退出恢复 Live 配置但保留 enabled，重启依 enabled 再接管；手动 stop_with_restore 恢复并清 enabled。必须补齐绑定与接管连接，不改变已有退出恢复语义。只为本链路实际缺口做局部修改，不重建守护进程或整套生命周期。绑定需在安全本机监听前提下才能应用；监听失败不遗留成功状态。先以源码和隔离 native 测试确认当前行为，再决定是否需要补丁。

测试优先使用临时 DB、FYAGENT_TEST_HOME、合成 token 和本机假上游，证明绑定→目标配置→代理协议→账号解析链；真实账号仅在明确可获得且能保持唯一授权持有者/恢复配置时使用。不读取或输出 token，不复制 refresh lineage 做并行测试。真实 UI/上游与 Windows 证据分别记录，不能伪称完成。

## 修改边界与分工

- Native：`commands/provider.rs`、必要的 Provider/Change Plan/Proxy/Managed Auth owner、权限注册和对应 Rust tests；删除旧 JSON 准入，不改登录协议或 schema，除非出现必须的明确缺口。
- Renderer：原分支触及的 `src/pages/{agents,models}`、当前 auth 导航、shared feature/ports/workspace 与对应 renderer/browser tests；跟随当前主线结构，删除残留 `src/v2`/legacy boot。
- 主控：基线整合、任务/方案、集成环境、跨层检查、审查与 PR。两个实施者的文件责任分开，接口调整先通知对方。
- 不改根目录 dirty worktree、其他 worktree、发布版本、无关功能和现有本机 Agent 配置。已有计划文档复制到本隔离分支提交。

## 取舍与恢复

当前直接延用内置 Rust 引擎，CLIProxyAPI 仅作明确兼容阻碍时的研究储备。本次不引入新网关依赖。所有修改在新分支，原分支和主线均不改写；产品级回滚使用原有目标配置备份和来源切换能力，不把 Git 回滚当作用户文件恢复。
