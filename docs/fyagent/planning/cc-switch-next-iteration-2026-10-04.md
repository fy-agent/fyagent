# FyAgent 下一轮：上游能力迁移、产品取舍与原生入口
日期：2026-10-04。状态：供认领与评审的迭代规划。总议题：[GitHub #204](https://github.com/fy-agent/fyagent/issues/204)。

本轮的产品目标是：**人在 Codex、Claude 等原生工具中完成工作，FyAgent 让模型、账号、Skills 与工具配置跟着人走，并承担跨工具安装、分配与恢复。** 先迁移能减少失败、减少往返操作的能力；复用已有本地底座。原生入口的收益应通过同一任务对照来证明。

## 1. 核查基线

| 对象 | 固定证据 |
|---|---|
| FyAgent | main `5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798`；[0.4.10](https://github.com/fy-agent/fyagent/releases/tag/v0.4.10)；当前数据库 schema 26 |
| 已吸收 CC Switch | [v3.19.2 账本](https://github.com/fy-agent/fyagent/blob/5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798/docs/upstream/cc-switch-v3.19.2.md)；peeled `43eaf07355af145aebfee301801779e824d4c221`，本轮确认是 main 祖先 |
| 当前上游稳定版 | [v3.20.4](https://github.com/farion1231/cc-switch/releases/tag/v3.20.4)，2026-09-22；tag object `93994110505d4d4aae3a7a3582797aae48c2dc74`；peeled `43e1d99084ed9b2f5dc252fd35c5adaf29d6876e` |
| 新候选 | [v4.0.0](https://github.com/farion1231/cc-switch/releases/tag/v4.0.0)，2026-10-04，GitHub 标为 **prerelease**；tag object `c5560d18eb659647ff1d1fe1b275a19413b5a03f`；peeled `412579f9ad81ed3c045476198aa9f8d1de36a2ae` |
| 上游 main | `a189980f35a4568f8cf3585747895c748b8cec6a`；相比 4.0 多两条文档提交，本轮不把它作为运行时目标 |

覆盖区间：[稳定段](https://github.com/farion1231/cc-switch/compare/v3.19.2...v3.20.4) 226 commits；[4.0 候选段](https://github.com/farion1231/cc-switch/compare/v3.20.4...v4.0.0) 199 commits。此次覆盖发布记录、提交清单与关键源码路径，**不代表逐行证明这 425 条提交均缺失或均能无冲突移植**。各能力在来源集成后仍要完成 FyAgent 的语义适配和运行验收。

## 2. 产品取舍表

| 对象 | 取舍 | 可感知结果 / 边界 |
|---|---|---|
| 软件发现、安装、版本与来源识别 | 保留；吸收上游兼容修复 | 看清本机实际用了哪份软件，升级不多装一份 |
| 账号、模型与供应商配置 | 保留；复用 SecretRef、配置预览、已有 writer | 配置可带到多个工具；不会因换入口丢掉凭据和个人设置 |
| Skills、MCP、提示词的跨工具分配 | 保留并加强 | 同一资源更新后知道哪些工具变了、保留了什么，失败能回来 |
| ChangePlan、Codex owned-field patch、恢复收据 | 复用、必要时扩展 | 单一变更和恢复事实；不另造一套底层引擎 |
| 多供应商聚合到原生模型列表 | 新增候选 | 在当前客户端选模型；FyAgent负责配置与路由，不要求回管理页反复切换 |
| 会话阅读/统计 | 增量升级 | 读清工具调用、失败步骤和用量；既有 native restore 保持独立 |
| 薄本地工具 + 宿主已有 diff | 优先验证 | 在对话里检查/预览，明确确认后应用；本地状态仍可重开 |
| 双目标原生面板 | 条件投入 | 仅当薄入口反复错认对象、漏冲突或无法恢复定位，且面板能解决时投入 |
| 上游橙色壳层/第二套安装中心 | 不整体移植 | 沿 FyAgent 已有信息架构改进实际任务流程 |
| 通用聊天、文件编辑、跨宿主任务/记忆总管 | 本轮不扩建 | 继续利用宿主已有能力，保留现有提示词/记忆用途 |
| 通用配置片段、整文件覆盖、成本倍率等旧机制 | 按存量和替代路径退役 | 先明确归属、迁移、导出和恢复，再移除；不照抄上游删除操作 |
| Linux 新支持、推广渠道/跟踪字段 | 不进入本轮正式产品 | 保持现有 Windows/macOS 与许可、品牌边界；保留必要来源署名 |

## 3. 上游能力覆盖矩阵

“迁移”指进入对应工作包；“复用适配”指 FyAgent 已有同类底座，不再重造；4.0 条目需要候选线验证。已存在的功能也要用上游新增用例检查差值。

| 能力族 | 来源 | FyAgent 处置 | 工作包 |
|---|---|---|---|
| Codex 0.149 写入前检查、凭据与current保留 | 3.20 稳定段 | 复用 provider/patch/SecretRef，补边界回归 | [#205](https://github.com/fy-agent/fyagent/issues/205) |
| additional_tools、original截图、Claude工具/stop/effort兼容 | 3.20.4 | 协议适配与回归 | [#205](https://github.com/fy-agent/fyagent/issues/205) |
| ChatGPT workspace身份与删除后重登录绑定 | 3.20 稳定段 | 复用 managed auth，核验identity差值 | [#205](https://github.com/fy-agent/fyagent/issues/205) |
| WSL原子写、备份保真、暂存校验、同步互斥 | 3.20 稳定段 | 复用备份/WebDAV/S3，补可靠性差值 | [#205](https://github.com/fy-agent/fyagent/issues/205) |
| 安装探测、版本噪声、重复npm副本 | 稳定段/4.0 | 现有AgentDirectory/安装所有权适配 | [#208](https://github.com/fy-agent/fyagent/issues/208) |
| MiniMax Code provider/MCP/Skills/Prompt/会话/用量 | 3.20.4 | 新增客户端adapter，明确不支持的域 | [#208](https://github.com/fy-agent/fyagent/issues/208) |
| MiniMax Code安装/升级 | 4.0 | 候选；复用安装框架 | [#208](https://github.com/fy-agent/fyagent/issues/208) |
| Pi客户端全链路、去重与模板 | 3.20 稳定段 | 当前AppType无Pi；按域补齐，不以卡片代替功能 | [#208](https://github.com/fy-agent/fyagent/issues/208) / [#210](https://github.com/fy-agent/fyagent/issues/210) |
| OpenCode JSONC保留、2.x会话/用量、Hermes读取 | 稳定段/4.0 | 现有adapter/parser增量适配 | [#208](https://github.com/fy-agent/fyagent/issues/208) / [#210](https://github.com/fy-agent/fyagent/issues/210) |
| Skill缺文件修复、目录ID、受限大仓库读取 | 3.20 稳定段 | 迁移安装/发现行为 | [#207](https://github.com/fy-agent/fyagent/issues/207) |
| Skill链接/复制、批量更新、OpenClaw目录所有权 | 4.0；部分机制FyAgent已有 | 复用已有模式，先补逐目标备份/确认/结果 | [#207](https://github.com/fy-agent/fyagent/issues/207) |
| MCP多格式批量导入、独立重试、Codex type兼容 | 4.0 | 复用MCP管理与各目标adapter | [#207](https://github.com/fy-agent/fyagent/issues/207) |
| Prompt回采外部修改、无启用时恢复不写、手写保护 | 稳定段/4.0 | 现有模板/文件边界适配 | [#207](https://github.com/fy-agent/fyagent/issues/207) |
| Prompt跨工具复制、导入、删除撤销 | 4.0 | 适配现有页面与恢复语义 | [#207](https://github.com/fy-agent/fyagent/issues/207) |
| 所有权字段patch、格式/注释保留、外部变更检查 | 4.0；Codex已有patch | 复用ChangePlan/patch，补其他目标和差值 | [#206](https://github.com/fy-agent/fyagent/issues/206) |
| 首写备份、多文件中断恢复、登录stash/退出恢复 | 4.0 | 与现有DB/凭据/恢复权威合并，不平行维护 | [#206](https://github.com/fy-agent/fyagent/issues/206) |
| 通用片段退役、live状态迁移、旧新设备同步/降级 | 4.0 | 独立数据兼容决策，不因schema号不变就直接回滚 | [#206](https://github.com/fy-agent/fyagent/issues/206) / [#213](https://github.com/fy-agent/fyagent/issues/213) |
| 直连/路由/聚合模式、按provider前缀模型ID路由 | 4.0 | 新增聚合；普通failover保留 | [#209](https://github.com/fy-agent/fyagent/issues/209) |
| 官方/第三方混用、压缩/reasoning、catalog重载 | 4.0 | 以实际宿主版本验证，不能只测接口JSON | [#209](https://github.com/fy-agent/fyagent/issues/209) |
| 结构化会话、按轮摘要、失败定位、查找/导出 | 4.0 | 新增只读typed timeline；native restore复用 | [#210](https://github.com/fy-agent/fyagent/issues/210) |
| usage去重、字节游标、cache定价、OpenCode SQL | 3.20 稳定段 | 复用统计底座，补计量正确性 | [#210](https://github.com/fy-agent/fyagent/issues/210) |
| 分页/大范围性能/热力图、真实与估算输出速度 | 4.0 | 适配；估算明确标记，成本不当实际账单 | [#210](https://github.com/fy-agent/fyagent/issues/210) |
| 模型搜索/批量选择/只填空参数 | 稳定段/4.0 | 复用model fetch和现有表单 | [#211](https://github.com/fy-agent/fyagent/issues/211) |
| 额度剩余/倒计时、重登录、托盘、设置分组 | 4.0 | 统一已有状态来源，避免完整壳层移植 | [#211](https://github.com/fy-agent/fyagent/issues/211) |
| 预设与价格维护、原有卡片/历史费用保护 | 稳定段/4.0 | 官方来源核对；不导入商业推广元数据 | [#211](https://github.com/fy-agent/fyagent/issues/211) / [#210](https://github.com/fy-agent/fyagent/issues/210) |
| ClaudeDesktop模型映射路由生命周期 | 4.0 | 复用代理启停，纳入退出/注销回归 | [#206](https://github.com/fy-agent/fyagent/issues/206) |
| 原生受限工具/专用面板 | DevDay后产品实验，非CC Switch已交付 | inspect/preview先行，apply依赖资源adapter | [#212](https://github.com/fy-agent/fyagent/issues/212) |
| 跨工具分配与Windows/Mac双版本恢复 | FyAgent保留能力 | 原生、数据与平台证据分开验收 | [#213](https://github.com/fy-agent/fyagent/issues/213) |

64组细项见[上游审计](research/2026-10-04-upstream-audit.md)，现有实现见[主干审计](research/2026-10-04-current-main-audit.md)。详表保留各组源码/PR证据；上述矩阵是便于产品评审的归组。

详细来源由 [CC Switch 发布列表](https://github.com/farion1231/cc-switch/releases)、固定标签源码与各工作包定位链接支撑。发生新版本时追加差值，不把本规划的固定SHA悄悄替换成main。

## 4. 三个影响顺序的源码发现

1. **配置patch不是空白。** [source_switch.rs](https://github.com/fy-agent/fyagent/blob/5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798/src-tauri/src/codex_config/source_switch.rs) 已有 `patch_source/project_source`，保留profiles/MCP等无关表。上游相似能力以新增用例与最小修复吸收。
2. **Skills更新不能直接作为外部工具。** [skill.rs](https://github.com/fy-agent/fyagent/blob/5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798/src-tauri/src/services/skill.rs) 的更新忽略备份失败，逐目标同步错误只记日志，备份优先单个主库来源。已有记录身份/分配状态保护，但不足以保留各副本个人修改。
3. **变更引擎和恢复已有边界。** [ChangePlan domain](https://github.com/fy-agent/fyagent/blob/5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798/src-tauri/src/services/change_plan/domain.rs) 当前仅3种操作。Skills需专用adapter与逐目标快照；会话timeline需新的只读DTO，不要改掉现有迁移中有意剔除tool/reasoning的规则。

## 5. 交互草图与对照任务

可点击离线草图：[fyagent-next-iteration.html](previews/2026-10-04-fyagent.html)。下载HTML后用浏览器打开；不连接本机配置、账号或网络。它是**模拟设计**，不证明插件连接或上游迁移已完成。

### 桌面端：清楚地完成本地管理

```text
AI软件配置  账号与认证  模型  Skills  MCP  提示词  记忆
资源 review-code     源v1 → v2
Codex  复制：增加了中文偏好       [查看变化]
Claude 复制：修改了同一规则       [处理冲突]
完整拟写入内容 → 确认预览 → 逐目标结果 → 本次备份/恢复
```

旧基线缺失时标“无法判断改动来源”。共享链接标明改一次会影响哪些工具。先备份全部实际受影响位置，再写；失败显示实际部分结果，不能以一个成功提示遮住失败目标。

### 薄原生入口：复用宿主差异审阅

```text
用户：比较新版与我的两个工具，先不要更新。
助手：源变化、两个目标、个人差异、冲突。[打开差异]
用户确认完整预览P1。
若Claude在外部被改：P1失效，本轮未写入 → 刷新P2、重新确认。
完成后：Codex回读成功；Claude失败/未执行/已恢复各自显示。
```

接口和本地事实共用，不在对话里重新发明恢复状态。面板候选仅改变辨认与确认的位置。先在A薄入口走正常、过期、部分失败三个任务；有具体问题才建设B双目标面板，用相同任务对照；记录错认目标、漏冲突、重新选文件、恢复定位。没有明确改善则保留A。

### 模型入口：在原生软件里选来源

```text
FyAgent：浏览 [直连] [路由] [聚合·候选]   ← 浏览不写配置
来源A / 模型X    来源B / 模型Y     默认来源A
[预览启用聚合的配置变化] → 确认 → 回读 → 必要的客户端重载提示
Codex/Claude原生模型选择：模型X·来源A / 模型Y·来源B
```

聚合按选中的模型分发，不提供普通路由的故障转移。必须实测原生选择到请求路由的完整链路，不能把草图或模型列表接口作为完成。

## 6. 工作包、顺序与并行方式

| 工作包 | 依赖 / 起步方式 | 对应审计组 |
|---|---|---|
| [#205 集成 CC Switch 3.20.4 稳定基线与兼容修复](https://github.com/fy-agent/fyagent/issues/205) | 可先认领；唯一来源集成owner | S01–S04、S09–S21；S06/S32/S36的底层兼容和测试部分 |
| [#207 补齐 Skills、MCP、提示词分配与逐目标恢复](https://github.com/fy-agent/fyagent/issues/207) | 稳定基线后；4.0子项等候选SHA | S26–S27、C16–C19；与工具票交接每客户端adapter |
| [#208 补齐 MiniMax Code 与其他客户端的能力差值](https://github.com/fy-agent/fyagent/issues/208) | 稳定基线后；4.0子项等候选SHA | S05–S07、S29–S31、C20/C22；C12的客户端格式交会话票 |
| [#206 建立 4.0 候选线并统一配置写入与升级恢复](https://github.com/fy-agent/fyagent/issues/206) | 稳定修复合入后，独立候选线 | C01–C04、C09、C23–C24、C28 |
| [#209 在 Codex／Claude 原生模型列表验证多供应商聚合](https://github.com/fy-agent/fyagent/issues/209) | 配置/数据候选通过后 | C05–C08 |
| [#210 升级会话阅读与用量统计，保留既有恢复流程](https://github.com/fy-agent/fyagent/issues/210) | 稳定与候选分轨；先对齐PR198 | S08、S22–S24、S35、C10–C15 |
| [#211 吸收额度、模型表单与模式提示，保持 FyAgent 现有界面](https://github.com/fy-agent/fyagent/issues/211) | 草图先行，接线随相关底座；同文件串行 | S25、S28、S32–S34、S36的呈现、C21、C25–C27 |
| [#212 先跑通受限本地工具，再决定是否建设专用面板](https://github.com/fy-agent/fyagent/issues/212) | 只读/预览先行；真实写入等资源adapter | FyAgent独立入口实验；复用资源票，不是上游已有插件 |
| [#213 验证 Windows／Mac 升级、原生入口与双版本恢复](https://github.com/fy-agent/fyagent/issues/213) | 样本/设备先行，实际验收等冻结构建 | 全组；平台/跨设备/升级降级与各票验收 |

- **现在能开始：** 稳定版来源集成；独立准备资源失败样例、原生只读/预览试验、界面草图和设备验收样本。
- **集成SHA固定后：** 资源、客户端、会话/统计适配按文件责任并行；shared DTO、DB、writer、注册入口只有一个集成owner协调。
- **4.0候选：** 先配置归属/升级恢复，再聚合；各域在固定候选SHA适配。不会因为上游今天发版就跳过旧新设备兼容验证。
- **发布收口：** 按已通过的能力与平台声明范围。版本号与日期在真实工作量和验收结果明确后决定，本规划不虚构“几天必发”。

来源合并必须遵循 [.trellis/spec/backend/upstream-sync.md](https://github.com/fy-agent/fyagent/blob/5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798/.trellis/spec/backend/upstream-sync.md)：显式two-parent、完整tag身份、来源账本、FyAgent数据/身份/许可优先。功能适配可拆票/PR，但不能用零散cherry-pick冒充来源集成。4.0来源合并阶段不要与其他人在同一批共享文件上并发写入。

## 7. 其他设备和同学如何接手

在对应Issue评论认领：设备/系统架构、负责模块、起点SHA、依赖状态、预计回传时间。代码从最新canonical main或明确冻结的集成SHA起步，建立独立 `codex/<issue>-<topic>` 分支；不得假定另一台机器的路径、账号与工具版本。

| 角色 | 主要负责 | 回传 |
|---|---|---|
| 集成负责人 | 稳定/候选来源、共享文件、DB与写入权威 | 固定SHA、来源账本、冲突决策、后续可开始的票 |
| Windows端 | 当前用户安装/升级、路径/复制/链接、退出恢复 | x64/ARM64各自证据与未覆盖范围 |
| Mac A | Keychain、符号链接、退出/注销、原生入口 | 本机OS/架构/客户端版本和完整任务链 |
| Mac B | 隔离数据、旧新版升级、双向同步、降级恢复 | 一致性结果与恢复边界 |
| 产品/前端 | 原生/桌面对照、状态与可用性 | 同任务对照记录、真实运行截图与剩余问题 |

这是角色建议，未替任何同学实名认领，也未远程访问两台Mac。没有设备不伪造PASS。公开共享材料去除个人账号、秘密及绝对路径。

旧票 [#67](https://github.com/fy-agent/fyagent/issues/67)、[#68](https://github.com/fy-agent/fyagent/issues/68) 继续管理发布归属/签名；[#198](https://github.com/fy-agent/fyagent/pull/198) 是未合并的会话恢复/Claude安装识别修复。8月旧V2规划及旧分支报告不作为新任务事实来源。

## 8. 本轮交付边界

已完成：最新来源/主干审计、产品取舍、能力覆盖与任务依赖、离线交互草图、GitHub协作清单。审计是 `code_audit`；HTML是mock。本轮没有迁移运行时代码、替换正式安装、验证真实账号/厂商请求，亦没有Windows/Mac原生迁移验收。

实施时每票回填可审阅PR、输入/预期/实际、适用版本、测试与真实运行证据、未覆盖边界、恢复办法。争议在相应Issue用“用户任务→当前阻碍→来源→现有实现→最小建议→验证”补充；产品取舍变化汇总到#204。

