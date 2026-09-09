# 验证记录

基线：`origin/main@11c13339`（与 `81e06aae` 文件树一致），工作分支 `codex/issue-52-health-center`。
最终代码检查与原生复验：2026-09-10，macOS / Apple Silicon。以下证据分层记录。

## 产品与实现

- 比较卡片内展开、独立仪表盘、共享快照加独立运行状态页三种方案，选择第三种，详见 `design.md`。
- 七款 Agent 共用十二项闭合状态合同；关键未知事实阻止“本机检查正常”。呈现状态、固定原因、时间和适用的处理入口。
- 默认检查只读本机来源，不执行目标 CLI、刷新登录、请求模型、导入 usage 或进行配置修复。带副作用操作进入既有明确操作流程。
- 保留失败前有效快照与原始时间；五分钟过期，串行批量可停止，隐藏页面停止后续派发，不后台轮询。
- 来源复核覆盖 OpenCode 多来源选择、Codex 官方登录与第三方凭据区分、代理实际路由、请求记录归属、TRAE SQLite WAL 和既有模型流程容量。
- 超时后后台协调任务持有唯一许可，直到实际读取完成；阻塞超时、拒绝叠加重试和完成后恢复有针对性回归。

## 完整本地门禁

最后一次代码修复后，串行执行完整 `check:prearchive` 通过。当前宿主经锁定 Python 包装调用：

```text
TRELLIS_CONTEXT_ID=01a086ce-0347-7ce3-827a-018f8952006a mise run python:run -- mise run check:prearchive --exclude-active-task .trellis/tasks/09-09-issue-52-health-center
```

包含 TypeScript、ESLint、格式、1641 项前端/契约单元测试（1 项原有跳过）、桌面 mock/视觉清单预检、Rust 编译/Clippy/全量测试、平台来源及版本/发布契约。Rust 主库 3237 项通过、5 项原有忽略，其他 workspace 与集成测试通过（其中另有 1 项原有忽略）。日志：`/tmp/fyagent-issue-52-final-gate.log`。

此前与浏览器并行的一次门禁有 9 项 5 秒超时，保留于 `/tmp/fyagent-issue-52-concurrent-gate.log`；未改变测试超时或实现，改为串行后完整门禁通过。

## 浏览器与构建

`mise run test:browser`：生产构建启动 2 项、浏览器交互 576 项通过。日志：`/tmp/fyagent-issue-52-browser-final.log`。
交互使用开发服务和明确 IPC fixtures，覆盖四种 Chromium 尺寸及 WebKit，不能代替原生读取。
覆盖处理入口和返回重读、串行批量、部分失败、停止/隐藏、筛选、窄窗口键盘、明暗主题及必要可访问性。

原生 QA 发现列表重新排序后选中高亮停留旧位置；复用现有布局失效信号修复。新增浏览器回归在修复前复现约 139.91px 偏差，修复后验证选中项与高亮偏差不超过 1px、窄高度裁剪、返回重读与焦点保留；原生复验也通过。

生产初始 JavaScript 为 664590 bytes，既有上限 665600 bytes。严格健康端口延迟加载，页面和端口经真实构建依赖图检查。未提高预算、改变依赖或修改锁文件。

`mise run build:debug` 成功生成最终 Apple Silicon `.app` 与 `.dmg`，日志：`/tmp/fyagent-issue-52-native-build-final.log`。这是本机调试构建，不是签名 Release。

## 性能对照：33 项通过，2 项未达预算

`mise run test:performance` 与编译、其他浏览器测试串行运行。同宿主、同依赖锁的干净主线 `11c13339` 也在相同两项测试失败。预算、断言和超时保持原样；不能写作全部通过，也不能由一次对照断言改动完全没有性能影响。

| 指标                       | 干净主线 | 最终分支 | 预算 / 结果       |
| -------------------------- | -------- | -------- | ----------------- |
| 导航 p95，1x CPU           | 36.9ms   | 38.8ms   | ≤100ms，通过      |
| 导航 p95，4x CPU           | 46.0ms   | 51.1ms   | ≤100ms，通过      |
| 账号呈现动画 frame p95，1x | 36.2ms   | 36.8ms   | ≤33.4ms，两者失败 |
| 账号尺寸动画 frame p95，1x | 45.5ms   | 44.1ms   | ≤33.4ms，两者失败 |

以上导航及失败动画采样均无 long task。主线导航 42 次、分支 48 次，差异来自新增第八个主页面。失败动画访问账号页面；本次没有修改该动画实现。保留基线限制，按 PRD 的功能、只读、构建/导航和原生交互范围交付 #52，不扩展修改无关动画。

可移植摘要：[performance-summary.json](research/performance-summary.json)。原始日志：`/tmp/fyagent-issue-52-performance-baseline.log`、`/tmp/fyagent-issue-52-performance-final.log`。首次分支采样也失败相同两项，日志和 trace 保留在 `/tmp/fyagent-issue-52-performance.log` 与 `/tmp/fyagent-issue-52-performance-first/`。

## macOS 原生与独立回读

最终调试包通过既有 `FYAGENT_TEST_HOME` 使用独立测试目录，凭据为无效测试值，模型服务地址为关闭的本机端口。执行：

1. 从 Codex 卡片进入运行状态，显示十二项本机结果和安装版本。
2. 检查全部软件，7/7 更新：Codex、Claude Code 本机检查正常，其余五项尚未配置。
3. 外部将测试 TOML 改为无效格式，重新检查后 Codex 变为暂不可用，显示固定配置格式原因；检查没有修改无效文件。
4. 恢复原测试文件，点击配置处理入口到 `#/models?target=codex`，返回运行状态后自动重读并恢复正常。排序变化后没有错误高亮；滚动到 Codex 时高亮与实际选中项一致。
5. 独立比对 6 份配置 SHA-256、25 张业务表的数据摘要及 TRAE 目录项，均与本次启动后的基线一致，TRAE 没有生成 WAL/SHM/journal。最终回执见 [native-readback.json](research/native-readback.json)。
6. 退出隔离测试进程并独立确认退出；重新打开原 `/Applications/FyAgent.app`，没有覆盖安装包。

最终原生截图（隔离测试数据）：

![恢复正常后的 Codex 运行状态与选中高亮](../../../../../docs/images/health-center/native-health-recovered.png)

## 工作树与交付

原工作树的分支、HEAD、状态列表和 tracked diff 摘要均与任务开始一致，回执见 [original-worktree-readback.json](research/original-worktree-readback.json)。
功能提交 `7bd07e72211dd295cf0c474f40727e0121aba06a` 已推送，交付 [PR #187](https://github.com/fy-agent/fyagent/pull/187)（draft）。最终归档与日志提交也推送同一工作分支；远程 CI 结果以该 PR Checks 为准。归档格式核对发现附件目录不符合既有白名单，已将 JSON 回执移至 research、原生截图移至 docs/images 并登记准确摘要，未放宽归档或素材检查。归档后的 `mise run python:run -- mise run check:contracts` 已通过，日志为 `/tmp/fyagent-issue-52-postarchive-contracts-final.log`。

## 证据边界

本机检查不证明远端额度、服务端授权或真实模型请求成功。Windows 原生、真实账号 UAT、签名和正式 Release 未验证。本任务交付工作分支及可审阅 PR，不合并、不发布、不自行关闭 Issue。性能两项未达预算保留为明确限制。
