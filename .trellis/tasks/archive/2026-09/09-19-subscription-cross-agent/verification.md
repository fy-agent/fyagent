# 订阅跨 Agent 修复与验收记录

日期：2026-09-19。基线：`origin/main` 的 `64f4d8f6`（0.4.5）。
候选分支：`codex/subscription-cross-agent-20260919`。

历史候选说明：以下安装包和 schema 22 验证属于订阅单项版本。2026-09-20
与 FDE 的组合代码以 PR #193、#192 及 `09-20-release-integration` 验证记录为准，
组合迁移使用 schema 23；旧安装包不作为两项功能整合后的验收候选。

## 代码结论

此前已经存在内部登录、凭据保险库、刷新，以及 OpenAI/xAI 订阅经 FyAgent
本地代理提供给 Claude Code、Codex、Grok Build 的实现。缺失的是 OpenCode
这条完整接入链。全链评审还发现请求规范、非流响应、恢复覆盖外部修改以及
非默认账号连接显示的问题；本次修复覆盖这些实际入口，不只增加目录按钮。

订阅复用通过本机 FyAgent 代理提供。目标 Agent 获得本地地址和模型配置，
不会获得用户的长期 OAuth 凭据。使用时需要 FyAgent 在后台运行。

| 目标        | 本次完成的代码路径                                                   | 用户确认方式                              |
| ----------- | -------------------------------------------------------------------- | ----------------------------------------- |
| Claude Code | OpenAI/xAI → 统一请求策略 → Messages 转换；配置恢复保护              | Models 中选择账号、模型并确认文件写入     |
| Grok Build  | OpenAI/xAI → Responses；非流 JSON/流式/工具调用                      | Models 中选择账号、模型并确认文件写入     |
| Codex       | 保存托管来源、既有 Change Plan 应用；保留原生认证刷新                | Models 保存后，到账号连接中预览并应用来源 |
| OpenCode    | 新增独立 Responses 路由、revision 检查、配置投影、启动恢复与退出还原 | Models 中选择账号、模型并确认文件写入     |

## 自动验证

最终 `check:prearchive` 已返回 0。环境、前端、后端、权限、平台源清单、
任务、文档、依赖锁、版本与发布约束全部通过。日志保存在本工作树的
`.trellis/.runtime/verification/prearchive-final.log`。以下验证均不等同于
真实订阅调用：

- Rust：3,611 项通过，6 项忽略；格式、编译和严格 Clippy 检查通过。
  真实本地监听器 + 合成上游服务 + 临时配置目录，验证两种账号、
  目标隔离、协议、工具回放、故障补偿、恢复、无 API Key 回退和秘密不外泄。
- Renderer：1,752 项通过，1 项跳过；类型、Lint、格式检查通过。覆盖
  边界解析、确认、账号/模型选择、revision、目标切换及
  未确认状态阻断。
- 桌面模拟：7 项通过，视觉基线前置检查通过；不作为原生截图验收。
- Browser：636 项通过；最后恢复提示变更另补测 20 项，全部通过。
  Chromium/WebKit 页面使用合成 IPC 数据，另有生产构建启动检查。
- 真实账号：尚未完成。已按用户授权尝试连接已登录 Chrome/X/Grok，浏览器
  连接和定向打开均超时；未读取浏览器凭据，也未把网页已登录当作 CLI 权限。

## 今晚用户验收

本地 Apple Silicon 候选已构建完成，应用及 ZIP 解包后的签名完整性检查通过。
验收副本使用本地 ad-hoc 签名，版本号仍为 0.4.5；它是本分支的本地候选，
不是正式发行包。包路径为本工作树的
`.trellis/.runtime/artifacts/FyAgent-subscription-UAT-20260919-arm64.zip`，
SHA-256 为 `4e43a212f020f74db8423e8af570abc3aa54d892ff8c876aee305001d5db91f8`。
已解包的应用位于 `.trellis/.runtime/artifacts/subscription-uat-20260919/FyAgent.app`。
候选二进制对应功能提交 `610400312a6ce227de5e9561653efed8993dc85a`；
此后的验收文档更新不改变上述应用、ZIP 或校验值。

### 启动前必须完成隔离检查

本订阅候选只能使用独立的 `FYAGENT_TEST_HOME` 测试目录，不能读取或迁移
公共安装使用的数据库，也不能与 FDE 候选交替使用同一目录。两分支均为
schema 22，但结构不同，具体风险见下节。不能通过同为 0.4.5 判断二进制
是否包含订阅修复；公共安装的维护由并行 FDE 任务负责。

1. 退出公共 FyAgent 和目标 Agent，保留现有安装及数据。准备一个新的、
   仅供本订阅候选使用的空目录；不要复制正式数据库或其他分支的测试数据库。
2. 启动前检查 macOS 应用 Store 的 `app_paths.json`：
   `app_config_dir_override` 若解析为测试目录外的现有路径，会优先于
   `FYAGENT_TEST_HOME` 生效。当前候选与公共应用使用相同的应用标识，
   这个 Store 并不会随测试 home 自动隔离。不得为本次验收修改公共 Store；
   如存在上述覆盖、无法确认该设置，或无法确认路径未通过符号链接指向正式
   数据，停止启动，保留真实账号 UAT 待执行状态，先准备可验证的隔离入口。
3. 检查目标 Agent 的最终配置路径均位于本次测试范围，确认写入提示后再绑定。
   `FYAGENT_TEST_HOME` 是 FyAgent 的目录覆盖，不会自动改变其他 CLI 的
   配置目录。目标客户端必须用各自支持的测试配置入口读取同一份投影；
   仅在空白项目目录启动 CLI 不能证明配置隔离。

前述检查完成后，才能通过下面带进程级环境变量的入口启动已签名候选。
先将 `<isolated-test-home>` 和 `<subscription-uat-app>` 分别替换为已核对的
独立测试目录和候选应用路径；不要直接双击应用或将它复制到
`/Applications/FyAgent.app`。后续重启也使用同一入口。

```sh
env FYAGENT_TEST_HOME="<isolated-test-home>" \
  "<subscription-uat-app>/Contents/MacOS/fyagent"
```

启动入口尚未完成真实账号 UAT。正式发行、Windows 实机及真实订阅额度均未
由本次本地自动检查证明。现有普通 API Key 接管到订阅的切换，以及旧版崩溃
备份升级，应在独立测试目录中构造：先停止并还原普通接管，再设置订阅；
缺少写入归属记录的旧订阅备份会保留文件并停止自动恢复，需要人工核对。
本指南不要求修改正式环境来构造这些场景。

### 隔离完成后的验收步骤

1. 在 FyAgent 的“账号”中登录自己的 OpenAI 或 xAI 账号，确认账号可选。
   OpenAI 手动输入账号实际支持的 Codex 模型 ID；xAI 页面提供的模型选项
   来自官方示例，最终仍以账号实际返回为准。
2. 在“模型”中分别选择 Claude Code、Grok Build、OpenCode，明确选中同一
   账号及模型，阅读写入文件提示并确认。应出现已应用状态，账号连接应指向
   所选账号。OpenCode 不需要重新导入同一个订阅的原生认证文件。
3. Codex 在 Models 保存后应显示“草稿”含义；进入现有账号连接页面，选择
   该来源，预览并确认应用。保存草稿本身不算切换成功。
4. 为每个目标打开一个空白测试目录，先请求只回复指定短句，再请求创建
   `subscription-acceptance.txt` 并读回内容。分别记录文本、流式输出和工具
   调用是否成功；不要只记录 FyAgent 的成功提示。
5. 如有两个账号，用不同账号绑定不同目标。确认连接展示与所选账号一致，
   修改默认账号不会让另一个目标悄悄更换来源。
6. 关闭目标 Agent 后退出并重启 FyAgent，再重新打开目标验证恢复与续用。
   测试配置中的原有账号、MCP、权限与其他配置应保留。若外部工具已改动同一文件，
   FyAgent 应保留该修改并提示需要处理，不能静默覆盖。
7. 在 OpenCode 模型面板选择“恢复之前的模型配置”，确认恢复后再保存普通
   API Key 模型。订阅配置应还原，API Key 编辑重新可用，其他 Agent 继续运行。

如出现 401/403、额度或模型不可用，记录目标、模型、时间和界面的安全错误，
不要提交 Token、完整认证文件或含凭据的日志。X 网站已登录并不能单独证明
该账号拥有 xAI CLI/订阅端点所需的访问资格。

## 后续合并：两个 schema 22 不兼容

已对照订阅功能提交 `61040031` 与 FDE 提交 `cd8b4ced` 的源码：

| 分支         | schema 21 → 22 的结构变化                                                                                    |
| ------------ | ------------------------------------------------------------------------------------------------------------ |
| 订阅 PR #192 | 重建 `proxy_config` 的约束并增加 OpenCode 默认行                                                             |
| 并行 FDE     | 客户/项目、资源代际及验证记录结构，包括 `fde_customers`、`fde_resource_generations`、`verification_evidence` |

两边的 `SCHEMA_VERSION` 都是 22。仅凭 `user_version=22` 无法判断哪些结构
已经存在，顺序启动两种候选也不能保证两边迁移均执行。当前约束是分开使用
数据目录；不覆盖数据库、不手动改版本号，也不把这两个候选视为互相兼容。

正式合并前，迁移需要统一到新的版本号，并根据实际结构补齐两种已存在的
22 状态。至少覆盖 schema 21、订阅版 22、FDE 版 22 的升级，以及重复运行
迁移后的结构与数据保留检查。该合并迁移尚未实现，不属于本次文档补充；
公共 0.4.5 与本订阅隔离候选继续分别保留。

## 目录扩展建议

用户已澄清目标是新的 AI 编程/办公工具。优先 Antigravity，其次 Hermes；
仓库已有部分配置/MCP 基础。Google 已宣布自 2026-06-18 起，个人免费及
Google AI Pro/Ultra 请求从 Gemini CLI 迁出，因此 Gemini CLI 仅保留为
企业/付费 API 场景候选。OpenClaw 随后以 Gateway 感知方式接入；Cursor
需要更多新的目录/安装/认证适配。此次不把调研候选标成已接入产品。
详见 `research/catalog-candidates.md` 的官方来源和最小接入范围。


## 2026-09-20: 0.4.6 merge follow-up

The earlier schema-22 incompatibility finding above is **superseded** by the
schema-23 integration in PR #192 and the archived release-integration record.
Both historical layouts have forward-migration and data-preservation coverage.

A new pre-merge regression was reproduced against an unchanged v0.4.5 Claude
subscription binding: the new restore path required a proof format v0.4.5 did
not store. The failing native test was preserved before repair. The fix uses
existing path-bound writer receipts and reconstructs the complete legacy
subscription projection. It rejects later edits, including another FyAgent
writer changing only the listener endpoint; it preserves a literal `null`
Claude preimage and absent original files. Codex login bytes and catalog data
without a historical preimage are not invented or overwritten by this repair.

Independent review found and closed the null-preimage and endpoint-only cases.
The inherited v0.4.5 local database remains the restore authority for original
fields hidden by repeated subscription projections: no historical signature
exists for those fields. This is explicitly documented in proxy-runtime SPEC;
it is not a claim to detect every possible database edit.

Validation on the combined 0.4.6 candidate, with backend source at
`2d1c08296ebf2470ed5612aa58111a32ed627f89`:

- Native upgrade regression: 7 passed, 0 failed, 0 ignored.
- Full canonical `mise run check:backend`: exit 0; formatting, Cargo check,
  Clippy and all 19 test suites completed. 3,714 passed, 0 failed, 6 ignored.
- Ignored cases remain the existing explicit performance, live S3, real Codex
  corpus and native credential-store hardware/integration checks. They are not
  evidence of real subscription generation or Windows desktop acceptance.
- All four relevant archived task context manifests validate. Product source
  was frozen before the full backend run; subsequent changes are release notes.

This PR carries the same five-file repair as commit `d6b92663`, based directly
on the previous PR head `fec6db08`. Only the two reviewed recovery source hashes
in the platform structure inventory were refreshed. Final combination,
post-archive contracts and exact-head/merge-queue CI are owned by the merge
coordinator and must be read back before mainline admission.
