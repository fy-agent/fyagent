# 执行当前基线文档重构与视觉资产规划

## Goal

以 `origin/dev/laiyongjie` 的 `b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527`
为基线，把 FyAgent 的公开入口、三语用户手册和当前开发文档整理成一套容易读、不会互相
打架的资料，同时建立真实截图与概念插图分层管理的对外视觉方案。

完成后，第一次打开仓库的人应能在 90 秒内明白 FyAgent 是什么、能解决什么问题、怎样
开始使用；维护者也应能从 README 进入唯一的工程事实来源，而不是在四种语言里维护四
套会过期的构建说明。

## Background

- 用户已经批准执行
  `.omo/plans/docs-restructure-v0.3.0.md`，并选择 v0.3.1 当前开发线。
- 基线审计见 `docs/fyagent/audits/docs-restructure-v0.3.0.md`：三语手册共有
  78 份 Markdown、40 张共享 PNG、84 处图片引用；活动文档检查了 372 个本地链接，
  当前失效数为 0。
- 目标基线已经删除 `docs/fyagent/dev/**` 旧设计包，并建立 12 份
  `docs/fyagent/development/**` current-state 文档。
- 根 README 仍有德语入口和大段重复工程内容；三语手册仍按五章组织，索引带有容易
  过期的“当前亮点”。
- README 的 6 张中/英/日首屏截图仍显示旧产品身份，不能作为 FyAgent 的运行时证据。
- VibeKey 只作为历史宣发与交互表达参考，不能把硬件形态、Claude-only、语音、众筹、
  试用额度、定价或未经验证的市场数字带回 FyAgent。

## Requirements

### R1. 基线与范围

- 所有实施与验证都以 `b6f60dfe` 及其当前工作分支为基线。
- 保留并避开用户的 `.omo/run-continuation/`。
- 不修改 `src/`、`src-tauri/`、`.github/`、许可证和社区治理文件。

### R2. 公开 README

- 删除 `README_DE.md`，并同步清理三份语言导航、文档合同脚本和两个边界测试中的活动
  依赖。
- 保留英文、中文、日文三份 README。
- 三份 README 只保留产品定位、核心能力、最短上手、安装入口、文档入口、贡献入口和
  法律/上游来源；工程环境、构建、测试、发布与架构细节链接到现有 current-state 文档。
- 中文必须自然、直接，避免“全方位解决方案”“赋能”等空泛表达；英文、日文也以清楚
  和可验证为先，不强求逐字互译。
- 有效工程信息在从 README 删除前必须已有唯一落点；不建立三套本地化工程合同。

### R3. 活动身份与历史边界

- 审计三份 README、四个手册索引和 75 篇现行手册正文中的仓库链接、Deep Link、数据
  路径和产品名。
- 仍把旧身份当作当前产品的内容改为 FyAgent；许可证、上游 PR、上游版本和历史 Release
  Notes 中的 `CC Switch` 保留并记录白名单。
- 将根目录 `session-manager.md` 移到
  `docs/fyagent/history/session-manager-prd.md`，首屏标明这是历史 PRD，不是当前产品合同。
- 新建 `docs/release-notes/README.md`，解释 FyAgent v0.3.x 与仓库内 CC Switch v3.x
  历史记录的关系。

### R4. 三语用户手册

- 删除四个手册索引中的滚动“当前亮点”，改为按任务找入口的简短导航。
- 每种语言从现有 25 篇正文重组为六章 28 篇正文：
  1. 快速入门；2. Agent 工具；3. 供应商；4. 扩展；5. 代理与高可用；6. 常见问题。
- 按计划中的一次性 Rename Map 移动每种语言 20 篇旧目录文件，共 60 篇；不得留下旧
  路径副本。
- 中、英、日各新增工具安装、升级诊断、WorkBuddy 三篇，并扩展 Skills 章节。
- 新内容只描述源码和测试能够证明的当前能力，不嵌入尚未拍摄的未来截图。
- 三语索引、正文交叉引用、图片路径和仓库反向引用全部同步更新。

### R5. 截图证据

- 对目标基线 40 张手册 PNG 和 84 处引用逐项登记：引用位置、语言、品牌/UI 状态、
  保留或重拍结论、对应 shot card。
- 创建 `docs/user-manual/assets/shot-cards/README.md` 和计划内 15 张 shot card。
- README 的 6 张主界面/添加供应商图必须来自真实 FyAgent 运行时，只有真实拍摄后才可
  标记为 `runtime_screenshot`。
- 如果本机无法得到合格运行时证据，README 先移除旧品牌截图引用，不得用 ChatGPT 生图
  或假 UI 顶替；未完成的重拍项必须明确保留为后续阻塞。

### R6. 营销与讲解视觉

- 保留 `vibekey-reference-audit.md` 的“继承 / 改造 / 放弃”边界。
- 新建 `visual-asset-plan.md`，记录受众、渠道、用途、尺寸/裁切、文案承载方式、优先级和
  发布门禁。
- 新建 `prompts/README.md`，至少包含主视觉、功能插图、流程讲解图、UI 辅助插图四类
  可复用提示词卡。
- v1 继续标记为 `superseded`；v2 继续标记为 `concept_candidate`。正式发布必须使用原始
  Logo 和准确文字做确定性合成，并与至少一张真实 FyAgent proof frame 配对评审。
- 本任务不批量生产资产矩阵中的所有正式图片，也不直接发布官网、商店或社媒素材。

### R7. 验证与证据

- 德语 README 删除闭包后运行两个目标测试和 `mise run check:contracts`。
- 最终运行手册拓扑、旧路径、活动身份、本地链接、图片引用、Git rename、合同、完整当前
  宿主质量门禁和 `git diff --check`。
- 结果写回基线审计，区分 `code_audit`、`runtime_screenshot` 和概念图视觉检查，不把
  静态审计说成运行时验收。

## Acceptance Criteria

- [x] AC1：工作分支包含目标提交 `b6f60dfe`，且 `.omo/run-continuation/` 未被修改或提交。
- [x] AC2：`README_DE.md` 不存在，三份 README、合同脚本和两个测试清单没有活动依赖。
- [x] AC3：三份 README 已显著瘦身，产品叙事自然，工程细节都有唯一 current-state 落点。
- [x] AC4：历史 Session Manager PRD 已归档并带醒目标记；Release Notes 索引存在。
- [x] AC5：每种语言恰有 28 篇手册正文，旧目录不存在，三语索引与文件系统一致。
- [x] AC6：60 个既有章节在纯移动阶段可被 Git 识别为 rename；新增 9 篇和 3 处 Skills
  扩展符合源码合同。
- [x] AC7：活动身份扫描只剩人工批准的历史/法律白名单，本地 Markdown 链接失效数为 0。
- [x] AC8：40 张手册 PNG 和 84 处引用都有审计结论；16 个 shot-card Markdown 存在。
- [x] AC9：README 不再展示旧品牌截图；若 6 张新图存在，它们必须有真实运行时证据。
- [x] AC10：视觉资产矩阵、四类提示词卡、VibeKey 审计及 v1/v2 状态完整。
- [x] AC11：目标测试、`mise run check:contracts`、`mise run check` 和
  `git diff --check` 全部通过，或留下具体、可复现、未被掩盖的真实阻塞。
- [x] AC12：审计报告包含最终目标 SHA、命令、退出码、关键结果和证据等级。

## Out of Scope

- 产品运行时代码与 UI 改造。
- 除 6 张 README P0 proof frame 外的实际手册截图重拍。
- Grok Build 独立深度章节、尚未实现的 AgentsPanel 文档。
- 把 v3.16–v3.19 上游 Release Notes 改写成 FyAgent 当前版本。
- 德语文档的后续维护。
- 批量生成和对外发布完整营销资产。

## Technical Notes

- 详细文件映射、提示词边界和验证命令以
  `.omo/plans/docs-restructure-v0.3.0.md` 为执行参考；发现与目标源码不一致时，以目标源码、
  测试和 Trellis 规范为准，并同步修正规划与审计。
- 本任务使用 Codex inline 实施，不派发子 Agent；`implement.jsonl` / `check.jsonl` 不作为
  启动门禁。
