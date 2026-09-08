# 2026-09-08 实施与验证证据

## 交付范围

本次完成 Grok 订阅到 Claude Code / Codex CLI 的配置与本机转发实现，并保留实验性标识。已有 Managed Auth 登录、vault/SecretRef 和刷新 owner 是唯一凭据来源。Claude 直接确认应用；Codex 保存来源后通过既有 Change Plan 预览、确认和回读。目标配置、renderer 与计划中没有上游 access/refresh token。

原代码的 legacy JSON 准入、旧认证页面挂载、固定 Provider ID/模型、普通 API 推理入口已替换。官方 CLI 订阅入口和最终模型路由头由 native 控制；普通 API key 来源保持原路径。详细来源固定在 `xai-subscription-upstream-20260908.md`，不是凭假上游成功猜测真实地址。

逐目标写入复用现有事务、备份和恢复；共享监听器的激活与补偿串行化，补偿回读 app/global/listener 状态。未确认恢复的结果会封锁继续写入。Codex 运行中由自身刷新或切换的原生 auth 文件保留当前字节，不把旧快照覆盖回去。订阅来源关闭该目标的自动 failover，避免静默使用另一账号或 API 计费来源。

## 分支与工作区保护

- 原 checkout `~/fyagent` 的分支、HEAD、已跟踪 diff 哈希和相同选项下的 status 与本次启动基线相同；本轮写入仅发生在隔离 worktree。
- 续作分支：`codex/grok-auth-reuse-completion`。原分支 `feat/grok-first-class-iteration` / 原 PR #172 保持 `b8b15dbaf141f7c7fbd7816914fda59a07a2208a`。
- 整合主线 0.4.4：`2f264d2f89326601a33f610c72a9f0143306d066`。13 个主线冲突按当前 `src/` 入口与 Managed Auth 合同解决；未恢复退役 renderer。
- 原提交标题不符合当前 Conventional Commit CI。本续作仅规范化私有新分支上的来源标题；来源副本 `55cea0b506139892720f1ce3f82a73622a6c4d7f` 与原提交具有相同 tree `a26537cac0c97389fafc34d27c8a26c338300fef`、相同父提交 `790922210d21144d63982d8bd6e873bf4c59de1a` 和相同作者元数据，保留 Original-Commit 来源。原远端分支不改写。
- 代码与任务文档先提交，再归档当前父任务和记录本轮 journal。三个旧子计划保留原提交的 in_progress 状态，更新归档链接，不把四目标/双机 HIL 标为已完成。
- 不合并 PR、不发布版本、不覆盖正式安装，不更改真实用户的 Agent 配置或锁文件。

## 环境与执行方式

- `mise run bootstrap` PASS：frozen pnpm 依赖、locked uv 环境、工具版本/归属和 80 项任务元数据校验；没有依赖升级。
- `mise run system:check` PASS：macOS Apple Silicon 原生编译与运行前置条件可用。
- 所有命令外层使用本机 `rtk`。需要 Python 的检查使用 `mise exec -- uv run --locked --no-sync mise run ...`，不依赖系统 Python。
- 可选 Windows MSVC cross 环境未安装；这不是 Windows 原生证据。
- WebKit 测试缺失的锁定版本浏览器从 Playwright 官方 CDN 补齐；未改 Playwright 版本、测试阈值或全局环境。

## 最终检查

| 检查         | 命令与结果                                                                                                                                                                      | 证明范围                                                                                                     |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| 前端完整门禁 | `check:prearchive --exclude-active-task .trellis/tasks/08-31-grok-first-class-iteration` 中 `check:frontend` PASS：181 文件、1614 passed / 1 skipped；另 7 项 desktop mock PASS | TypeScript、lint、格式、renderer/domain、结构合同和 fake IPC；视觉 manifest 只作 preflight                   |
| 后端完整门禁 | `mise run check:backend` PASS，exit 0；19 个测试目标合计 3518 passed / 6 ignored，fmt/check/Clippy PASS                                                                         | 当前 macOS 原生 Rust 和隔离集成链路                                                                          |
| 归档前合同   | `check:contracts:prearchive --exclude-active-task .trellis/tasks/08-31-grok-first-class-iteration` PASS，exit 0                                                                 | 任务、文档、平台结构、锁文件、版本、release contracts；无 release 发布                                       |
| 浏览器       | `mise run test:browser` PASS：生产启动 2 项、renderer 主矩阵 534 项；新增订阅 WebKit 专项 2/2 PASS                                                                              | 主矩阵通过开发服务器测试 renderer，生产启动单独验证；Chromium 四尺寸和 WebKit，均使用 fake IPC               |
| 性能         | `mise run test:performance` 首轮 34 passed / 1 failed；保持阈值不变单独复跑 `state-performance.spec.ts`，2/2 PASS，exit 0                                                       | 首轮 1x 账号弹窗帧 P95 33.7 ms，复跑 33.4 ms 满足 33.4 ms 阈值；记录边缘波动，不冒充首轮全绿或转发器资源实测 |
| 独立 review  | `full-scope-review-20260908.md`：六项问题已修复，最终小修复核后无已确认阻断项                                                                                                   | 独立源码与相关合同检查；reviewer 定点 31/31 PASS                                                             |

浏览器原先切换目标后使用通用订阅 region，可能操作退出中的 Claude 区域。修为先定位精确的 Codex 父区域并等待其保存按钮后再选择账号；未增加 sleep、force 或放宽 timeout，原 payload 与应用次数断言保留。

完整门禁最初在 Clippy 的测试 fixture `is_some + expect` 处中止；仅改为 `if let` 后从后端门禁续跑。之后全量测试发现新增 IPC 的 handler 数量冻结仍为 367；核对新增绑定命令后更新为 368，并增加精确命令存在断言，保留权限集合相等和远端限制。最终后端与合同门禁均通过。前端内容未因此变化，不重复已通过的前端全量测试。这里分别报告各组件最终结果，不把首次聚合非零退出冒充一次全绿运行。

6 项 Rust ignored 为原有性能诊断、外部 S3、真实 Codex 语料和 OS 凭据库 HIL；本轮没有增加跳过。结构 hash seal 仅更新经 review 的 `lib.rs` 和 `services/provider/mod.rs` 两项，扫描器未放宽。

性能复跑命令为 `mise exec -- pnpm exec playwright test tests/browser/state-performance.spec.ts --config=config/playwright.performance.config.ts`；没有调整动画实现、测试次数、CPU throttle 或阈值。35 个性能场景中其余 34 项首轮通过，复跑包括 1x/4x 两档账号弹窗；该检查是浏览器补充观测，不是原生 WebView 或代理资源验收。

## 本机集成具体覆盖

新增订阅集成测试使用临时 DB、隔离配置目录、合成授权、真实 Provider/Change Plan writer 与实际 loopback listener/router/协议转换器：

- 选择账号、purpose/consumer/refresh-owner 检查；撤销、过期、计划后变更和同模型多账号。
- Claude Messages 和 Codex Responses 的非流式、流式工具调用和结束事件；在替换为假上游前断言真实订阅 host/path。
- 所选模型形成最终路由头，传入的伪造认证/路由头与 Codex 自带错误模型不会覆盖它。
- 账号不可用时不调用上游，不退回 legacy JSON/default account/API key。
- Provider/文件/运行态回读与补偿，端口/写入失败，状态篡改报告 unknown。
- Claude 失败与 Codex Change Plan、手动接管两种并发入口；一个失败事务不能停止另一个目标已采用的服务。
- Codex 原生 auth 的保存、CLI 刷新/切换后当前字节及恢复边界。

## 真实环境与残余验证

只读查看已安装 FyAgent 的“账号与认证”时，账号数为 0；已请用户补充登录，当前没有取得可用的真实订阅测试身份。没有读取、复制或输出凭据。现有正式安装是 0.4.4，不代表新分支运行证据。

本次未验证真实 Grok 上游请求、额度扣除或具体账号 entitlement，未启动已安装的 Claude Code/Codex CLI 做端到端请求，也未在 Windows 上运行。HTTP fixture 的真实 native 链路不能替代这些结果。模型建议 `grok-build` 来源于官方文档，页面明确它不构成账号模型名单。

后续应使用可恢复的真实账号环境分别验证两种 CLI 的选定账号/模型、流式工具调用、刷新、额度/限额响应以及退出恢复，再决定解除实验性标识。现有 listener 只随 FyAgent 运行；真正退出恢复配置、下次启动按既有 enabled 状态接管，手动停止恢复并清除 enabled。没有引入新的常驻进程。

## 日志与 PR

本机完整执行日志保存在临时目录，文件前缀为 `fyagent-grok-`，包括 `prearchive-final-20260908.log`、`backend-verified-20260908.log` 和 `contracts-prearchive-final-20260908.log`。浏览器日志为 `browser-final-pass-20260908.log` 与 `webkit-subscription-20260908.log`。标准矩阵通过后，仅将新增订阅 spec 纳入既有 WebKit 项目，再运行 `mise exec -- pnpm exec playwright test tests/browser/xai-subscription.spec.ts --config=config/playwright.config.ts --project=webkit-1232x700` 验证新增两项；其余已通过测试内容未改，不重复整套矩阵。浏览器 trace/图片由 Playwright 写入其临时 artifacts 目录，未提交用户数据。

本记录截至提交前全部本地检查收尾；归档后仍执行无排除的 `check:contracts` 并在 PR 中记录。最终 work SHA、归档/journal SHA 与新 PR 的 head/状态以 Git 历史和 PR 回读为准；不修改或关闭原 PR #172，不关闭 #42/#43。
