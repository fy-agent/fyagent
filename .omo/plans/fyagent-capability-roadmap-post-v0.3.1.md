---
type: plan
status: proposed
updated: 2026-08-10
review_on: 2026-08-24
authority: product_roadmap_proposal
source: docs/fyagent/audits/vibekey-to-fyagent-capability-gap.md
---

# FyAgent v0.3.1 后能力路线图

## 0. 推荐结论

接下来不应以“再加 Provider、再加一个工具入口”为主线。建议把产品迭代集中在三个可验收结果：

1. **第一次成功**：新用户能从首次打开走到一次真实成功调用。
2. **每次变更可控**：写配置前能预览，失败可恢复，完成后可追溯。
3. **工作状态可迁移**：在另一台设备恢复完整工作区，同时不泄露秘密。

路线图的依赖顺序是：

```text
真实发布与能力基线
  ├─ 普通用户运行边界 ─┐
  └─ Secret Vault ─────┼→ 首次成功向导 + Health Center
                       └→ Change Plan + Undo
                                  ↓
                       Workspace Manifest / Pack
                                  ↓
                     Scenario Packs + 团队策略
                                  ↓
                      受信任注册表 / Marketplace
```

没有完成左侧基础合同，就不进入右侧生态功能。

## 1. 规划前提

- 产品定位：本地优先的 AI 开发工具控制面，不是通用 Agent 执行器。
- 当前运行时基线：FyAgent `0.3.1`，目标代码 SHA `b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527`。
- 当前公开发布基线：`v0.3.0`，正式附件完整但 Windows/macOS 未签名；`v0.3.1` 尚无公开 Release。
- 主要用户：已使用一到多个 AI 开发工具的开发者和技术用户。
- 估期按单一主开发者、可获得必要平台测试设备计算；阶段以验收门禁为准，不以日期强行放行。
- 所有“完成”必须标明证据等级：`code_audit`、`runtime_screenshot`、`native_runtime`、`remote_release` 或 `user_research`。

## 2. 成功指标

### 北极星指标

`Time to First Verified Request`：从首次打开 FyAgent 到用户选择的目标工具完成一次真实、最小、无副作用请求的时间。

### 首轮目标值

这些值是产品验收门槛，不是已实现数据：

| 指标                               |            Beta 门槛 |
| ---------------------------------- | -------------------: |
| 首次成功中位时间                   |            ≤ 10 分钟 |
| 受支持 happy path 首次成功率       |                ≥ 80% |
| 错误后可继续或恢复的向导步骤       |                 100% |
| Change Plan 失败后的自动恢复成功率 |  ≥ 99%（故障注入集） |
| Workspace Pack 干净环境恢复成功率  |  ≥ 95%（受支持矩阵） |
| 导出、同步、日志、诊断包秘密扫描   |               0 泄露 |
| 正式 Windows/macOS 安装信任        | 平台原生签名验证通过 |

若没有合规的 opt-in 事件采集，首次成功指标先由本地测试日志和 10–20 名封闭 Beta 用户记录，不为了数据先引入远程遥测。

## 3. Phase 0：事实与发行信任（2–3 周）

### CAP-001 当前能力矩阵与首次成功漏斗

**目标**：为每个受支持工具明确 FyAgent 能管理什么、不能管理什么，以及第一次成功需要哪些步骤。

**范围**：

- 建立工具 × Provider × MCP × Prompt × Skill × Profile × 安装 × 会话 × 代理矩阵；
- 为 Claude Code、Codex、Gemini CLI、OpenCode 各定义一个最小 happy path；
- 把“安装完成”“配置写入”“端点连通”“目标工具成功请求”分成不同状态；
- 固化结构化错误码和不含秘密的本地漏斗事件草案。

**验收**：

- 矩阵每个“支持”项都有源码或测试锚点；
- 四条 happy path 都有正常、缺依赖、凭据失败、端点失败和需要重启的验收用例；
- 不再用“八个工具全方位支持”掩盖各工具语义差异。

### CAP-002 签名发布与宿主更新决策

**目标**：先让用户能够信任和持续更新 FyAgent，再扩大对外投放。

**范围**：

- Windows Authenticode；
- macOS Developer ID、notarization、staple；
- `signing-status.json` 与现有 attestation/release 合同闭环；
- 决定自更新是进入下一阶段，还是先采用签名安装包的人工升级；
- 记录证书、账号、CI secret、吊销和轮换责任。

**验收**：

- 真实 x64/ARM64 Windows 安装包通过 Authenticode 验证；
- 真实 macOS 包通过 `codesign`、`spctl` 和 notarization 验证；
- 一次公开或候选 Release 的重新下载验证通过；
- 没有 `native_runtime + remote_release` 证据不得标记完成。

## 4. Phase 1：第一次成功与安全变更（6–10 周）

### CAP-101 Windows 普通用户 Worker

**目标**：在不削弱现有安全边界的前提下，让正式 Windows 构建能够检查和管理用户自己的 CLI、PATH 与 WSL。

**推荐设计**：提权宿主只保留机器级职责；用户 CLI 操作交给有身份绑定、命令白名单、长度限制、超时、结构化回执和重放保护的普通用户 worker。

**明确禁止**：

- 删除正式构建的 elevated guard；
- 让高权限进程直接执行用户 PATH 或 shim；
- 允许前端传任意命令行；
- 把可见终端当作可靠执行结果。

**验收**：

- 正式 Windows x64/ARM64 包在普通用户上下文完成探测、安装、升级和失败回执；
- PATH 注入、WSL 伪装、陈旧 worker、重放、超时和进程所有者漂移均 fail closed；
- Codex CLI 是否开放写生命周期另立产品/安全决策，不因 worker 上线自动放开。

### CAP-102 Secret Vault v1

**目标**：把 Provider、同步和未来 Workspace Pack 中的秘密从普通配置对象中分离。

**推荐数据模型**：

```text
Provider definition  → 非秘密字段 + secretRef
secretRef            → OS keychain / Credential Manager / Secret Service
portable secret      → 用户显式创建、口令派生、版本化加密 Vault
export / sync        → 默认只携带 definition，不携带 secret
```

**范围**：

- 盘点 Provider、WebDAV、S3、OAuth token 和第三方配置里的秘密字段；
- 从 SQLite 明文配置迁移到操作系统密钥存储；
- 默认导出、自动备份和 WebDAV/S3 同步不包含秘密；
- 对确需跨设备携带的秘密提供独立、显式、可撤销的加密 Vault；
- 旧库迁移必须有备份、幂等、回退和失败后不删原值的策略。

**验收**：

- 数据库、SQL 导出、同步 artifact、日志和诊断包的秘密 fixture 扫描为 0；
- macOS、Windows、Linux 各有真实 keychain round-trip；
- 迁移中断、密钥存储不可用、旧版本回退和跨设备缺 secret 均有明确结果；
- 文档不得只写“AES-256”，必须记录算法、KDF、nonce、版本、完整性和轮换合同。

### CAP-103 首次成功向导 + Health Center

**目标**：把分散能力串成一条可恢复的完成路径。

**向导步骤**：

1. 选择目标工具；
2. 检查 FyAgent、目标 CLI、运行时、PATH/WSL 与冲突安装；
3. 安装、升级或给出平台正确的人工动作；
4. 选择登录方式或 Provider，写入 Secret Vault；
5. 预览将修改的配置；
6. 执行端点检查和目标工具真实最小请求；
7. 展示成功、重启要求和下一步。

**Health Center 汇总**：工具版本、冲突安装、凭据状态、端点、Provider 健康、代理接管、配置生效、需要重启、最近失败。

**验收**：

- 每个步骤可重试、跳过、返回，重启应用后能继续；
- “端点可用”和“目标工具成功请求”不得合并为一个绿灯；
- Windows、macOS、Linux 至少各一条 `native_runtime` happy path；
- 封闭 Beta 达到本计划的首次成功门槛。

### CAP-104 Change Plan、Undo 与活动记录

**目标**：把 VibeKey 的“接受/拒绝/重试/中断”变成软件中的可逆变更合同。

**首批覆盖**：Provider 启用/切换、Profile 应用、代理接管、MCP/Prompt/Skill 批量变更、Deep Link 导入。

**Change Plan 至少包含**：

- 将读写的文件或数据库域；
- 语义化差异，不显示完整 secret；
- 是否需要重启目标工具；
- 将创建的备份 ID；
- 可回退边界和不可逆动作；
- 预期成功信号。

**验收**：

- 同一后端计划对象驱动预览和执行，前端不重复推断；
- 执行中断、原子写失败、目标配置非法和部分应用失败均能恢复或保留明确现场；
- Undo 在版本和文件指纹不漂移时恢复，漂移时拒绝覆盖并给出冲突说明；
- 活动记录默认不含 Key、Token、完整 URL query、用户 Prompt 或消息正文。

## 5. Phase 2：可迁移工作区（6–8 周）

### CAP-201 Workspace Manifest / Pack v1

**目标**：把“数字人格随身携带”改造成安全、可审计、可分享的项目工作环境。

**Pack 内容**：

- schema/version、创建工具版本、适用 OS/架构；
- Provider definition 和 secret placeholder；
- MCP、Prompts、Skills、工具可见性与项目设置；
- 所需 CLI、最低版本、重启要求；
- 权限与网络风险声明；
- 内容哈希、来源和可选签名。

**不包含**：默认不带 API Key、OAuth token、聊天记录、请求日志、用户绝对路径和机器标识。

**验收**：

- 导出 → 清空测试环境 → 导入 → 预览 → 应用 → 验证的 round-trip 通过；
- 未知 schema、缺依赖、悬空引用、冲突名称、危险 MCP/Skill 和缺 secret 都有结构化结果；
- Pack 应用复用 CAP-104 Change Plan，不另建一套写入逻辑。

### CAP-202 Profile 覆盖与工具语义补齐

**目标**：让 Profile 从三个应用的 ID 快照升级为 Workspace Pack 的本地快速切换视图。

**范围**：

- 评估 Gemini、Grok Build、OpenCode、OpenClaw、Hermes 各自可支持的 Provider/MCP/Prompt/Skill/Workspace 槽位；
- 明确“unsupported”“not captured”“captured empty”的区别；
- 解决 Profile 只保存 ID、换设备后悬空的问题；
- 保留按工具隔离和切走自动保存的有价值语义。

**验收**：

- Capability Matrix 与前后端 scope 枚举、序列化和测试一致；
- 所有支持槽位有跨应用 round-trip；
- 不支持能力在 UI 中明确显示，不静默清空。

### CAP-203 Scenario Packs 与内置样例

**目标**：用可复用配置组合替代“大而全的自动化引擎”。

首批只做 3 个真实故事：

1. 国内 Provider + Codex 的最短可用配置；
2. Claude/Codex 项目切换与共享 MCP/Skills；
3. OpenClaw/WorkBuddy 的受限模型和通道配置。

每个 Pack 都必须包含来源、维护者、版本、所需秘密、网络端点、写入范围、卸载/撤销方式和真实验证步骤。

**验收**：

- Pack 通过相同风险扫描、Change Plan 和 Undo；
- 不内置未经审查的远程脚本；
- 每个故事有一条 `native_runtime` 证据和一份可公开复现说明。

### MKT-201 讲解与营销证据包

**目标**：把 Phase 1–2 的真实用户路径变成对外说明，而不是继续罗列功能。

**范围**：

- 30/60/90 秒脚本各一版；
- “第一次成功”“安全变更”“工作区迁移”三组真实 proof frame；
- ChatGPT 生图只生成主视觉背景、关系插图和非界面元素；
- 界面讲解元素使用真实截图、状态标签、步骤连线和局部放大，由确定性模板合成；
- Logo、标题、标签和 UI 必须使用确定性合成和真实截图；
- 提示词、种子图、来源、状态、画幅和发布许可进入资产登记。

**现有起点**：

- [VibeKey 对照审计](../../docs/fyagent/marketing/vibekey-reference-audit.md)
- [对外视觉资产计划](../../docs/fyagent/marketing/visual-asset-plan.md)
- [ChatGPT 生图提示词卡](../../docs/fyagent/marketing/prompts/README.md)
- [对称线路软件编排 v3 样例](../../docs/fyagent/marketing/visual-direction-sample-v3.md)

**验收**：

- 概念图标记为 `concept_candidate`，不冒充运行时；
- 每个对外功能主张都能回到目标发布 SHA、真实截图和验收记录；
- 发布前完成 16:9、1200×630、4:5、1:1 独立安全区检查。

## 6. Phase 3：运维、团队与生态（Phase 2 稳定后）

### CAP-301 可选诊断包与产品反馈

- 一键生成脱敏诊断 ZIP：版本、平台、能力矩阵、结构化错误、最近活动、配置指纹，不含秘密和内容正文；
- 用户预览后自行保存或上传；默认不开启远程遥测；
- 若加入崩溃/产品事件上报，必须独立同意、可关闭、可删除，并公布字段清单；
- 用 Beta 数据复核首次成功率和前三类失败，不以下载量替代激活。

### CAP-302 Team Policy 与审计

- 团队分发已签名 Workspace/Scenario Pack；
- Provider/MCP/Skill allowlist 与版本固定；
- 变更审批、审计导出、设备撤销和最小权限；
- 先完成单用户 Secret Vault、Change Plan 和 Pack schema，再选择是否需要服务端。

### CAP-303 受信任注册表 / Marketplace 决策

只有以下门禁全部通过才立项：

- Pack schema 稳定并有兼容策略；
- 来源、签名、风险扫描、撤销和更新策略存在；
- 至少 20 个真实用户反复使用 3 个以上 Scenario Pack；
- 维护、举报、下架和恶意包响应有人负责。

门禁未通过时，继续使用受审查的内置样例和用户自选 Git 仓库。

## 7. 暂缓项与重新进入条件

| 暂缓项              | 原因                                                       | 重新进入条件                                                       |
| ------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------ |
| VibeKey 实体硬件    | 供应链、固件、IP、支持面与当前软件主线无关                 | 至少 30 名目标用户访谈和可工作的软件控制面原型证明实体输入不可替代 |
| 语音输入            | 系统权限、隐私、跨平台质量负担高，不解决配置控制面核心问题 | 首次成功/工作区场景出现持续的无障碍或免手需求数据                  |
| 通用工作流编排器    | 会把 FyAgent 变成执行平台，安全和产品边界完全不同          | Scenario Pack 被证明不足，并完成独立 PRD、威胁模型和执行沙箱设计   |
| 内置本地 Agent      | 与 Provider/配置管理不是同一职责；性能和模型分发成本高     | 用户明确需要离线执行，且仅配置 Ollama 等外部运行时无法满足         |
| API 转售与试用额度  | 涉及支付、额度、服务质量、合规和资金风险                   | 商业模型、主体、法务和支持能力独立验证通过                         |
| 家庭/泛职场用户扩张 | 当前交互和术语仍服务技术用户                               | 技术用户首次成功率稳定，且非技术用户研究证明同一产品可服务         |

## 8. 任务拆分与启动顺序

建议每个 `CAP-*` 建独立 Trellis 任务卡；不要把 Phase 1 合成一个长分支。

| 顺序 | 任务                            | 依赖                                        | 可并行性                            |
| ---: | ------------------------------- | ------------------------------------------- | ----------------------------------- |
|    1 | CAP-001 能力矩阵与漏斗          | 无                                          | 可与 CAP-002 并行                   |
|    2 | CAP-002 签名发布                | 外部证书/账号                               | 可与 CAP-001 并行                   |
|    3 | CAP-101 Windows 普通用户 Worker | 当前 Windows 安全合同                       | 可与 CAP-102 设计并行，实施分支分开 |
|    4 | CAP-102 Secret Vault            | 秘密盘点、迁移合同                          | 与 CAP-101 分开实施                 |
|    5 | CAP-104 Change Plan / Undo      | CAP-102 的 secret projection                | 可先覆盖 Provider/Deep Link         |
|    6 | CAP-103 首次成功向导            | CAP-001、CAP-101、CAP-102、CAP-104 基础 API | 集成任务                            |
|    7 | CAP-201 Workspace Pack          | CAP-102、CAP-104                            | Phase 2 起点                        |
|    8 | CAP-202 Profile 覆盖            | CAP-001、CAP-201 schema                     | 可按工具拆子任务                    |
|    9 | CAP-203 Scenario Packs          | CAP-201、CAP-202                            | 可与 MKT-201 后半并行               |
|   10 | MKT-201 讲解证据包              | CAP-103/201 真实运行时                      | 不得提前用假 UI 占位                |
|   11 | CAP-301/302/303                 | Phase 2 指标与信任门禁                      | 后续决策                            |

## 9. 第一批建议任务卡

用户批准本路线图进入实施后，第一批只创建四张卡：

1. `CAP-001-current-capability-matrix`
2. `CAP-002-signed-release-readiness`
3. `CAP-101-windows-user-worker`
4. `CAP-102-secret-vault-v1`

CAP-103/104 在上述边界产出经过审查后再创建，避免向导先绑定不稳定 API。

## 10. 路线图验收与停止条件

每一阶段结束时回答：

- 用户是否更快完成第一次成功，而不只是多了一个页面？
- 失败时是否比之前更容易理解和恢复？
- 数据是否经过完整 round-trip，并且秘密没有跨越不该跨越的边界？
- 正式包是否在真实平台上通过，而不只是单元测试通过？
- 对外内容是否只展示目标发布 SHA 真实存在的能力？

若连续两个迭代没有改善首次成功率、恢复成功率或信任证据，停止扩大 Provider/Pack/Marketplace 范围，回到 CAP-001 的失败分布重新排序。

## 11. 审计来源

- [VibeKey → FyAgent 产品能力差距审计](../../docs/fyagent/audits/vibekey-to-fyagent-capability-gap.md)
- [FyAgent 开发文档入口](../../docs/fyagent/development/README.md)
- [Windows Runtime Security](../../.trellis/spec/backend/windows-runtime-security.md)
- [FyAgent 版本与发布资产合同](../../.trellis/spec/backend/fyagent-version-contract.md)
- [VibeKey 宣发与产品设计对照审计](../../docs/fyagent/marketing/vibekey-reference-audit.md)
