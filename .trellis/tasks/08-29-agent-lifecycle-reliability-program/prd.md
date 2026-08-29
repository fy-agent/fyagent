# Agent 安装、认证与前端可靠性治理计划

## Goal

以当前 `dev/laiyongjie`（已同步最新 `main`）为规划基线，串行推进以下五个可独立验收的阶段：

1. 建立安装候选、目标选择和更新目标的权威合同；
2. 修复 macOS 安装/更新跨目录、跨 scope 写入的问题；
3. 补齐 Windows 多来源发现和一键安装；
4. 将“已打开登录入口”改造成可取消、可回读的认证会话；
5. 治理前端选中态、状态管理、复用、性能和误导性交互。

本计划不是重写 FyAgent。现有 Agent Catalog、Tooling、Codex Desktop Installer、Auth Center、FeaturePorts、TanStack Query、Radix 和 V2 shared UI 继续作为优先复用对象；只有现有边界无法表达用户已确认的语义时，才增加一个窄的共享所有者。

## Source Requirements

- 更新已安装应用时，默认目标必须是用户当前选定或已明确确认的那一份安装；不得把 `/Applications` 中的应用静默更新到 `~/Applications`，也不得在 Windows 的 user/system/custom scope 之间静默迁移。
- 多份安装必须作为显式候选返回。没有权威选择时状态是 `ambiguous`，不得按扫描顺序取第一项。
- Renderer 可以选择 backend 生成的候选 ID，但不得提交任意 URL、路径、命令、安装参数、token 或绕过字段。
- 安装、更新和登录的成功必须来自权威回读；启动安装器、打开终端、打开浏览器或复制文件均不能单独升级为成功。
- Windows 发现不能继续只靠固定目录和自研 PE 字符串扫描；应组合系统注册信息、App Paths、PackageManager/MSIX 和受控产品目录，并保留每条证据的来源。
- 前端选中状态必须由元素自身的稳定 CSS/ARIA 状态表达；液态 Lens 只能作为装饰增强，失效或延迟时不能让状态变暗或消失。
- 优先复用现有共享组件和已采用的开源基础设施。实现中发现第二个真实消费者时，应在同一任务内提升为共享所有者，不能等待重复代码继续增长。
- 测试过程中发现的同域缺陷应一并修复：根因必须属于当前子任务边界，补回归测试，并更新验收项。跨域或扩大产品范围的缺陷应记录到 #141 或创建窄任务，不得无边界扩张。

## Child Task Map

| 顺序 | 子任务 | 独立交付 |
| --- | --- | --- |
| 1 | `08-29-agent-install-target-authority` | 安装候选、scope、owner、revision、选择和回读合同 |
| 2 | `08-29-macos-agent-in-place-update` | macOS 原位置更新、staging、回滚和多候选处理 |
| 3 | `08-29-windows-agent-discovery-install` | Windows 多来源 inventory 与闭集安装执行 |
| 4 | `08-29-agent-auth-verification-state-machine` | Agent-owned/provider-owned Auth 会话和权威验证 |
| 5 | `08-29-frontend-reliability-architecture` | V2 交互、状态、复用、拆包和测试可靠性治理 |

阶段 1 是阶段 2、3 的硬依赖。阶段 4 可以在阶段 1 之后与平台安装工作并行开发，但不得复用安装 job 的语义来伪装 Auth。阶段 5 的独立可靠性修复可先做；安装候选和 Auth UI 的最终接线分别依赖阶段 1 和阶段 4 的 DTO。

## Architecture Requirements

- Agent Catalog 继续拥有产品 ID 和能力政策；不得创建第二个产品注册表。
- 安装发现、安装执行、安装后验证必须是三个可测试边界，不能继续由一个平台大文件同时承担全部职责。
- 现有 Codex Desktop 的可信候选、受控下载对象、job、取消、临时目录和平台安装经验应被复用或抽出通用内核；不得在 `agent_install` 内维护第二套安全策略。
- Windows inventory 应有一个项目级 owner，按证据 adapter 聚合、归一化和去重；不同 Agent 不得各写一套 registry/App Paths/MSIX 扫描器。
- Auth 应有独立的 session coordinator 和 closed adapter；安装阶段枚举不能直接复用为 Auth 阶段枚举。
- 前端继续通过 FeaturePorts 消费 DTO。页面不得直接 `invoke` 新命令，也不得直接依赖平台实现类型。
- `FeatureTabs` 保留为 FyAgent 共享适配器，但交互语义优先由已安装的 Radix Tabs 承担。
- 页面拆分应以领域职责和可测试边界为依据，不以行数为唯一标准；共享组件也必须有真实第二消费者或稳定的共同语义。

## Issue Alignment

- #31：多份安装、用户选择目标、更新失败保留可用版本。
- #47 / #101：已有安装默认保持，观测与写入分离，多安装先选择权威来源。
- #141：最新 `main` 的 UAT 缺陷复验总账；本计划只吸收阶段 1-5 同域问题。
- #68：FyAgent 自身 Windows Authenticode 发布；保持独立，不在本计划内重复实现。
- #71：FyAgent 自身更新渠道；保持独立。本计划处理的是外部 Agent 安装/更新目标与恢复语义。

## Non-goals

- 不把 Prompt live 文件清空和 Daily Memory 混合 Markdown 两个 Stage 0 blocker 偷渡到本计划；它们继续由 #141 和独立修复任务处理。
- 不实现通用 shell runner、任意 URL 下载器、任意路径安装器或全盘文件扫描。
- 不读取、复制或解析 Claude、Grok、OpenCode、QoderWork、TRAE Work、WorkBuddy 的凭据文件来推断登录态。
- 不重写 Codex Desktop Installer、Tooling 或 Auth Center。
- 不在没有真实 Windows/macOS HIL 时宣称平台安装或登录完整通过。
- 不处理 FyAgent 自身安装器签名、发布渠道和自动更新。

## Cross-child Acceptance Criteria

- [ ] 五个子任务都有完整 `prd.md`、`design.md`、`implement.md`、研究证据和上下文清单，且保持 `planning` 直到用户批准实施。
- [ ] 阶段 1 冻结候选身份、revision、scope、owner、ambiguity 和请求边界后，阶段 2/3 才开始实现。
- [ ] 所有更新路径都证明“更新的是用户选择的原候选”，没有静默跨 scope fallback。
- [ ] Windows 检测能保留多条证据和多份安装，不以 registry、目录或 PATH 中任意一条单独冒充唯一事实。
- [ ] Auth UI 不再把流程启动显示成登录成功；只有支持权威验证的 adapter 才能进入 `succeeded`。
- [ ] 左侧导航和 Tabs 在 Lens、动画、ResizeObserver 或异步测量不可用时仍有清晰选中态。
- [ ] 新的项目级公共能力有单一 owner、窄 facade、架构测试和至少两个真实消费者；没有为了“公共化”扩大 `pub`/IPC 表面。
- [ ] 每个子任务记录同域测试发现及其处置：已修复、拆出窄任务或明确不适用。
- [ ] 最终集成评审覆盖 Rust/TS contract、前端单元/浏览器测试、macOS HIL、Windows HIL、Auth 取消/超时以及 UAT 回读证据。

## Delivery Boundary

父任务只负责需求总账、依赖顺序和最终集成评审，不直接承载业务实现。实施时逐个启动子任务；不要启动父任务或一次性把五个阶段塞进一个实现分支。
