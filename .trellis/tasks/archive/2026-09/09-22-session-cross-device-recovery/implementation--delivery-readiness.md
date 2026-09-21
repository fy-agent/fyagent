> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Session 跨设备恢复交付说明

状态：实现、本机检查与独立评审已完成；正在提交 PR 并跟进远端检查。仓库 `fy-agent/fyagent`，分支 `codex/session-cross-device-recovery-20260922`，基线 `c0b2ec21`。

## 功能

新增 Session 主入口，提供会话选择、原文预览、迁移包导出/导入、目标工作目录选择、重复处理及原生历史核验；原有 Memory 文件与编辑入口保留。

按来源版本提取真实用户文本和 AI 最终答复，保留角色、顺序、空白和真实未回答问题；未知完成状态拒绝导出。工具、推理过程、附件、工作文件、凭据及源模型配置不进入迁移包。恢复使用目标本机账号、模型和工作目录。

恢复先登记整个明确选择的请求与目标 ID；空选择拒绝，重试复用回执，不覆盖原生会话。机器/用户/存储实例绑定防止复制配置继承错误身份；读回不一致进入待处理状态，未知副作用不能盲重试。界面分开显示写入与核验，人工确认不能提升系统证据。

## 能力与限制

| 软件 | 核验版本 | 严格原生导出 | 原生恢复证据 |
| --- | --- | --- | --- |
| Codex | 0.154.0 | 已实现，要求可靠来源版本及 final 标记 | 官方读回、重启读回、下一请求原文保真。 |
| OpenCode | 1.18.30 | 已实现，核对完成状态与消息边界 | 实际 Rust 写入器、官方读回、下一请求保真；竞争时全事务回滚且不覆盖已有内容。 |
| Hermes | 0.20.5 | 已实现，要求来源/完成字段 | 已安装 SDK 写入与官方读回；拒绝原生显示/续聊会修剪或合并的消息形状。 |
| Gemini | 0.46.0 | 关闭：任意原始历史缺少可靠 final 证据 | 已校验迁移包写入及重启读回；拒绝被解释为命令或上下文而丢失的正文。尚无任意原始历史的完整导出→恢复链路。 |
| Claude / OpenClaw / Grok Build | 无已核验版本 | 无已核验规则 | 无已核验原生恢复器；保留明确能力缺口。 |

版本与内容形状均独立设门槛。四个写入器不代表七种软件全部支持。所有原生实验使用隔离合成历史和本机端点；HTTP 400 捕获只证明请求携带了什么，**没有真实模型回复或用户桌面 UAT**。

**Windows 实机验收按用户决定略过。正式 Windows 提权宿主还缺普通用户 CLI 执行通道，因此相关能力探测/恢复保持禁用。** 这是实现边界，独立于实机验收豁免；不能绕过现有权限限制。

## 最终本机检查

| 检查 | 实际结果 |
| --- | --- |
| 类型 / lint / 前端格式 | 全部 exit 0。 |
| 前端规范 aggregate | 223 文件，2,134 passed / 1 既有 skipped；桌面 mock 7/7、视觉清单 preflight 通过。 |
| Rust | fmt:check、check、clippy 通过；最终全量 4,109 passed / 0 failed / 7 ignored，含真实生产库 migration integration 11/11。原生 probe 的 ignored 项已另行显式隔离运行。 |
| 浏览器 | 完整初次 666/667；唯一动画用例的数据就绪问题修复后，相关文件 25/25。最后 Session + shell 36/36、生产启动 3/3。分别记录，不冒充一次完整 667/667。 |
| 预归档与平台 | composite exit 0：664 passed / 1 既有 skipped，native-fetch 4/4，平台检查 3,001 文件；未放宽门槛。 |
| 视觉 | 合成界面 900×600 与 1440×900 的 8 张最终截图已检查，无页面溢出或运行错误。 |

Grok 4.7 High 最终提出的“空选择扩大为整包”“读回不一致状态”两项均修复，并由生产编排中的计数写入器回归证明没有额外写入。评审未发现第二套回执库或重复并行导入器。OpenCode 官方导入仅在临时数据库运行，目标库使用 INSERT-only 事务；临时数据库不保存第二份长期回执或正文缓存。

Antigravity 承担 UI/UX 及前端实现，Cursor Opus 承担技术方案和后端主体，Cursor Sol 承担测试；GPT-6 负责产品/架构、关键原生投影与跨包整合，原生执行子任务完成身份、原生接续和规范验证。Cursor Opus 最后一轮退出非零且未交最终报告，已明确交接并由协调者完成核验；不把执行者退出等同验收。

本次不含云同步、合并、部署、真实模型推理或工作文件迁移。早期 src/v2、Codex inject-only、Hermes foreign import 和 OpenCode 占位模型方案已被后续反例及当前实现替代；研究文件仅为决策来源。

## 审阅证据

入口：[证据索引](implementation--evidence-index.md)、[前端结果](implementation--final-frontend-validation.md)、[Rust 结果](implementation--final-rust-validation.md)、[平台结果](implementation--contracts-integration-report.md)、[Grok 处置](implementation--grok-final-resolution.md)、[OpenCode 实际写入器](implementation--opencode-create-only-report.md)、[身份并发修复](implementation--identity-concurrency-fix.md)。原始 CLI 流、过程进程信息和未筛选运行目录保留于仓库外，不纳入 PR。
