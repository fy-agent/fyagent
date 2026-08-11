---
type: audit
status: reviewed
updated: 2026-08-10
review_on: 2026-09-10
authority: product_capability_audit
source: VibeKey local pitch materials, FyAgent v0.3.1 source, Git history, public release evidence
---

# VibeKey → FyAgent 产品能力差距审计

## 结论

FyAgent 不缺“再支持一个模型或工具”，当前最明显的缺口是：用户还不能稳定地从安装应用走到第一次成功调用，也不能确信密钥、配置变更和跨设备恢复都可预览、可回退、可证明。

VibeKey 当年最值得保留的不是硬件键盘，而是三项产品要求：

1. **把复杂准备压缩为一条完成路径**：检测环境、安装工具、配置凭据、验证成功。
2. **把高风险动作变得可见、可确认、可撤销**：用户知道将修改什么、何时生效、怎样恢复。
3. **让工作状态可迁移**：换设备后恢复的不只是 API Key，而是供应商、扩展、项目习惯和工具状态。

当前 FyAgent 已经比 VibeKey 原型更接近这些目标：它真实拥有多工具 Provider、MCP、Prompts、Skills、代理、故障转移、用量、会话、Profile 和同步能力。但这些能力仍是若干独立面板，没有形成“首次成功”“安全变更”“跨设备恢复”三个端到端产品合同。

推荐的产品重心是：

> **把 FyAgent 从“多工具配置管理器”推进为“本地优先、可验证、可恢复的 AI 开发工具控制面”。**

通用 AI 工作流编排、硬件、语音、API 转售和 Agent Marketplace 不应进入最近两个迭代；它们会扩大范围，却不能先解决信任和激活问题。

## 1. 证据边界

### 1.1 VibeKey 历史材料

| 证据 | 发现 | 证据等级 |
|---|---|---|
| `VibeKey-路演PPT-25页 (完整版)(1).pdf` | 实际为 23 页，创建于 2026-01-28；SHA-256 `07D7D573F49BD99BA26AEA021BEA7C58398143B8631E030FF72FA8A6E034CEF6` | `local_artifact_audit + visual_inspection` |
| `C:\Users\wq241\Desktop\submission\drafts\VibeKey-商业计划书-正式版.md` | 记录产品、商业和路线图假设；正文最终明确“软件开发、用户验证、众筹上线”为待完成项 | `local_artifact_audit` |
| `C:\Users\wq241\Downloads\vibekey-project.tar.gz` | 包含早期市场、产品、硬件、软件、UI/UX 文档和驱动脚手架；SHA-256 `9C54280EB1EB700800AB2022CEF32C392690ECB301D21DC7BBCB07A2BDE9F0C1` | `local_artifact_audit` |
| `C:\Users\wq241\Downloads\vibekey-driver` | 无 `.git`；缺少 `index.html`、TypeScript/Vite/Tailwind 配置和锁文件；存在试用激活、密钥持久化等 TODO | `code_audit` |
| 本机 Git / GitHub 账户搜索 | FyAgent 历史只在后来的设计边界中提到 VibeKey；未找到独立 VibeKey 仓库或代码结果 | `git_history_audit + remote_git_audit` |

PPT 和商业计划中的“Mac mini 购买证明控制焦虑”“高置信度”“数千 Stars”、成本、毛利、合作、试用额度和付费意愿，没有原始样本、访谈记录、订单或第三方来源。它们只能作为待验证假设，不能成为 FyAgent 的功能优先级证据。

### 1.2 FyAgent 当前基线

- 运行时代码基线：`origin/dev/laiyongjie` 的 `b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527`，Cargo 版本 `0.3.1`。
- 当前工作分支 HEAD：`df88ec9a331e858b5b912b5cd44585e7860b9af6`；相对 `b6f60dfe` 没有 `src/`、`src-tauri/` 运行时代码变化。
- `v0.3.1` 是 annotated tag，剥离后指向 `99738a00260da3ea095f8d8750c6d8af97e07cf5`；截至审计时没有对应 GitHub Release。
- 最新公开 Release 是 `v0.3.0`，有 13 个正式附件，但发布说明明确 Windows、macOS 均未签名，macOS 未公证。
- 本轮对 FyAgent 的判断是 `code_audit + remote_release_audit`，没有把源码审计说成真实新机安装或首次使用验收。

## 2. VibeKey 主张的可信度分层

| 分层 | 历史主张 | 本轮处理 |
|---|---|---|
| 可继承的问题定义 | 配置和安装摩擦高；用户需要明确状态；换设备重配成本高 | 进入 FyAgent 路线图 |
| 可改造的交互原则 | 一键安装、接受/拒绝、模式灯、随身配置 | 改为软件向导、变更预览/撤销、健康中心、Workspace Pack |
| 未完成的技术设想 | 系统密钥链、AES-256、远程擦除、本地 Agent、语音识别、权限钩子 | 不当作历史交付；重新立项才可进入源码 |
| 未验证的商业假设 | 众筹、价格、毛利、试用额度、API 转售、企业定价 | 不进入近期能力路线图 |
| 与当前定位冲突 | Claude-only、实体键盘、家庭用户、通用数字员工 | 放弃 |

## 3. FyAgent 已经具备的能力

| 能力域 | 当前证据 | 判断 |
|---|---|---|
| 多工具配置 | 支持 Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes | **强项**；远超 VibeKey 的 Claude-only 原型 |
| Provider 与模型 | 预设、自定义端点、OAuth/Key 形态、切换、模型发现与连通性检查 | **强项**；不应再以 Provider 数量作为主路线图 |
| 扩展管理 | MCP、Prompts、Skills 的安装、启用和多应用同步 | **强项**；适合成为 Workspace Pack 的基础 |
| 本地路由 | 本地代理、应用接管、故障转移、熔断、健康状态 | **强项**；已有“控制面”雏形 |
| 用量与会话 | token/成本看板、请求日志、跨工具会话与工作区入口 | **已具备**；需要与诊断、项目状态联动 |
| 配置可靠性 | SQLite、原子写入、自动备份、导入/导出、WebDAV/S3 同步 | **已具备底座**；秘密管理和便携语义仍有缺口 |
| 不可信导入 | Deep Link 预览、字段限制、显式启用批准 | **局部成熟**；可复用到统一变更预览 |
| 工具生命周期 | 七种 CLI 的版本/安装分布探测；除 Codex 外可安装/升级；多安装冲突诊断 | **部分具备**；正式 Windows 发行边界仍禁用 |

## 4. 能力差距矩阵

| ID | 用户目标 | 当前状态 | 关键证据 / 限制 | 缺口等级 | 建议 |
|---|---|---|---|---:|---|
| G1 | 安装 FyAgent 后完成第一次成功调用 | **部分具备** | 首启仅是通知弹窗；安装、Provider、检查分散在不同页面；没有可恢复向导状态 | P0 | 建立首次成功向导和统一 Health Center |
| G2 | Windows 正式版检测、安装、升级 CLI | **受阻** | 正式 Windows 构建因提权安全边界，在进入 PATH/WSL/用户目录前 fail closed；Codex 生命周期始终只读 | P0 | 设计普通用户 worker 或调整宿主安装边界；禁止用放宽安全检查解决 |
| G3 | 密钥在本机、备份和同步中都可证明安全 | **缺失统一合同** | Provider `settings_config` 以 JSON 写入 SQLite；同步导出包含 Provider 表；未发现 OS keyring/SQLCipher/应用层密文依赖 | P0 | 密钥引用化 + OS 密钥链；默认导出/同步不带密钥；另设加密便携 Vault |
| G4 | 每次配置变更可预览、确认、撤销 | **局部具备** | Deep Link 有预览；Provider/Profile/接管/批量扩展没有一个统一变更计划和撤销账本 | P0 | 建立 Change Plan、影响说明、备份 ID、Undo 与活动记录 |
| G5 | 换电脑恢复完整工作状态 | **部分具备** | WebDAV/S3 可同步整库；Profile 只覆盖 Claude、Claude Desktop、Codex，且 payload 主要保存现有实体 ID | P1 | 建立版本化 Workspace Manifest/Pack，分离定义、引用与秘密 |
| G6 | 所有支持工具拥有一致的项目切换体验 | **部分具备** | Profile scope 未覆盖 Gemini、Grok Build、OpenCode、OpenClaw、Hermes；各工具对 MCP/Prompt/Skill 的支持也不同 | P1 | 先发布 Capability Matrix，再按工具语义补齐；不做虚假“全功能一致” |
| G7 | 一个地方判断“为什么不能用、怎样修” | **部分具备** | 工具版本、安装冲突、模型检查、Provider 健康、代理状态和日志分散 | P1 | Health Center 汇总依赖、凭据、端点、配置生效、重启要求和修复动作 |
| G8 | 分享一套可复用工作方式 | **基础存在** | Skills 仓库、MCP/Prompt、Profile 已存在，但没有可审计的组合包、版本约束、风险声明和秘密占位 | P1 | 先做声明式 Scenario Pack；不先做拖拽工作流引擎 |
| G9 | 为团队统一策略并追踪变更 | **缺失** | 当前为本地单用户模型，没有组织、RBAC、策略签名、审计导出 | P2 | 个人工作区稳定后再做 Team Policy 和审计 |
| G10 | 通过真实数据验证激活和失败原因 | **缺失** | 未发现产品分析或崩溃上报；现有日志不是产品漏斗 | P1/P2 | 先定义本地事件和隐私边界，再做明确 opt-in 的诊断/统计 |
| G11 | 安装包容易信任和持续升级 | **部分具备** | v0.3.0 有供应链证据但未签名；v0.3.1 无公开 Release；宿主自更新未启用 | P0 | 代码签名、公证、真实发布闭环优先于扩大营销投放 |
| G12 | 直接执行晨报、会议、消息分拣、DevOps 等任务 | **缺失且非当前核心** | FyAgent 管理工具与配置，并不拥有通用 Agent 执行运行时 | 暂缓 | 先用 Scenario Pack 配置外部工具；是否自建执行引擎必须单独发现 |

### 4.1 G3 的安全判断说明

当前代码对前端回传会清空 WebDAV 密码和 S3 Secret Access Key，也有日志脱敏、导入 SQL 边界和原子写入等安全措施；这不能等同于“Provider API Key 已加密存储”。

关键数据流是：

```text
Provider 表单
  → settings_config JSON
  → SQLite providers.settings_config
  → export_sql_string_for_sync()
  → WebDAV / S3 db.sql
```

因此路线图必须先定义“秘密是什么、由谁保存、何时可导出”，再改 UI。不能只给数据库文件加一个营销上的“已加密”标签。

### 4.2 G2 的安全判断说明

正式 Windows 发行版当前以受保护的提权宿主运行。代码明确拒绝在该边界内探测或执行用户可控的 PATH、WSL 和工具管理器 shim。这是有意的 fail-closed，不是普通 UI bug。

后续设计必须把以下两类责任分开：

```text
机器级安装 / 受保护操作  → 受限高权限边界
用户 CLI / PATH / WSL     → 已认证的普通用户 worker
```

删除 guard 或直接让高权限进程运行用户 PATH 都不属于可接受修复。

## 5. 建议的产品定位

### 当前主用户

未来两个版本继续服务已经在使用一到多个 AI 开发工具的人：开发者、技术管理者和高频 AI 工具用户。VibeKey 面向“非技术职场人”的扩张，在首次成功率、安装成功率和恢复成功率有真实数据前不进入主目标。

### 核心任务

1. **第一次成功**：用户选工具和 Provider，FyAgent 完成检测、准备、写入和一次验证。
2. **安全变更**：用户应用 Provider、Profile、代理或扩展前，能看到影响；失败后能恢复。
3. **带走工作区**：用户能在另一台设备恢复一套不泄露秘密的工作环境。

### 北极星指标

> `Time to First Verified Request`：从首次打开 FyAgent 到目标工具通过一次真实、最小、无副作用请求的时间。

配套指标：

- 首次成功完成率与各步骤失败分布；
- 变更撤销成功率；
- Workspace Pack 在干净环境的恢复成功率；
- 导出、日志、诊断包中的秘密泄露测试为 0；
- 正式 Windows/macOS 安装的签名与信任状态。

## 6. 继承、改造与放弃

| 决策 | VibeKey 元素 | FyAgent 处理 |
|---|---|---|
| 继承 | 环境检测 → 安装 → 配置 → 完成 | 做成可中断、可恢复、可诊断的首次成功向导 |
| 继承 | 状态灯 | 改为 Health Center 与明确状态，不只靠颜色 |
| 继承 | 接受 / 拒绝 / 重试 / 中断 | 改为变更预览、取消、重试、Undo 和失败保留现场 |
| 继承 | 30 秒恢复工作状态 | 改为有版本和风险扫描的 Workspace Pack；时间指标由实测决定 |
| 改造 | Normal / Auto / Plan | 近期只约束 FyAgent 自己的配置变更；不宣称控制外部 Agent 的每个动作 |
| 改造 | 本地化模型和通道 | 继续提供国内 Provider 和 OpenClaw/WorkBuddy 配置支持，不自建消息平台 |
| 改造 | 模板市场 | 先做内置签名样例和仓库导入；形成信任模型后再考虑 Marketplace |
| 放弃 | 实体键盘、LED、硬件序列号 | 不进入最近路线图 |
| 放弃 | 语音输入 | 不属于配置控制面核心任务 |
| 放弃 | Claude-only | 与当前八工具方向冲突 |
| 放弃 | 通用工作流引擎 | 先让外部工具执行，FyAgent 管理其配置、能力包和安全边界 |
| 放弃 | API Key 批发、试用额度、众筹 | 无证据且带来合规、资金和支持负担 |

## 7. 路线图输入

详细的阶段、任务依赖和验收证据见 [FyAgent v0.3.1 后能力路线图](../../../.omo/plans/fyagent-capability-roadmap-post-v0.3.1.md)。

相关历史宣发和视觉边界见 [VibeKey 宣发与产品设计对照审计](../marketing/vibekey-reference-audit.md)。
