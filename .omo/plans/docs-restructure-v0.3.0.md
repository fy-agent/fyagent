# FyAgent 文档体系重构计划（v6 — v0.3.1 执行基线版）

> **Plan state**
>
> - intent: clear
> - status: completed
> - review_completed: 2026-08-10
> - gate_0_completed: 2026-08-10
> - review_evidence: code_audit
> - sample_evidence: generated_asset_visual_inspection
> - implementation_approval_required: false
> - implementation_approved: 2026-08-10
> - implementation_completed: 2026-08-11
> - post_completion_visual_revision: 2026-08-11（v2 线路问题修正为 v3）
> - slug: docs-restructure-v0.3.0
> - source_of_truth: `origin/dev/laiyongjie` / `b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527`
> - delta_report: `docs/fyagent/audits/docs-restructure-v0.3.0.md`
> - environment_note: 用户已信任仓库；portable `mise 2026.8.2`、`bootstrap`、Windows `system:check`、`check:contracts` 和完整 `mise run check` 均已通过

## v6 执行基线修订摘要

1. 用户已接受推荐方案，目标基线锁定为 `origin/dev/laiyongjie` 的
   `b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527`。
2. 只读快照复核结果为：三语手册 78 份 Markdown、40 张共享 PNG、84 处图片引用；
   三份 README、手册和当前开发文档共检查 372 个本地链接，失效 0 个。
3. 目标基线已经删除 `docs/fyagent/dev/` 旧设计包，并建立 12 份
   `docs/fyagent/development/` 当前文档；旧版“再拆 9 份开发正文”方案取消，README 直接
   瘦身并链接现有 current-state 文档。
4. 每种语言仍有 25 篇手册正文，六章重组的 60 个移动和 9 个新增页经重算后保留；内容
   以目标基线为准，不沿用旧正文哈希。
5. `README_DE.md` 已删除，Session Manager PRD 已归档，三语 README 与四个手册索引已
   瘦身，README 不再引用 6 张旧品牌截图。
6. 三语手册已重组为每语 28 篇；40 张旧截图、84 处引用和 16 份 shot-card 文档已登记。
   营销资产矩阵、提示词库、VibeKey 对照和概念样例也已落地；完成后又根据视觉复核将 v2 的线路系统修正为 v3。

## v5 评审修订摘要

1. 删除 `README_DE.md` 的级联范围补齐到 `scripts/tasks/docs-contract-check.mjs`、`tests/localBuildBoundary.test.ts`、`tests/desktopSecurityBoundary.test.ts`；否则合同检查和测试会直接读取已删除文件。
2. 当时曾把 README 瘦身设计为 9 个语言正文 + 1 个索引；该文件方案已在 v6 被目标基线现有的 current-state 文档取代。
3. 当前基线修正为 75 个既有章节、40 张 PNG、84 处图片引用；40 张图片均被引用，不存在“34 张”基线。
4. Agent 工具能力按源码修正为：7 个工具参与版本探测，6 个工具支持安装/升级，Codex CLI 仅探测、不进入安装/升级动作。
5. WorkBuddy 前端错误码枚举修正为 22 个；源码锚点改用符号名，不再依赖易漂移行号。
6. 章节路径和可见编号更新要求使用一次性映射或按 `5→6、4→5、3→4、2→3` 降序处理，防止连续替换级联误伤。
7. 验证命令统一使用 `rg`、PowerShell 和仓库 `mise run` 入口；删除 Bash 专属的 `test`、`grep`、`ls`、`wc`、进程替换。
8. `git` 不记录“是否执行过 git mv”；验收改为 `git diff --summary -M` / `git show --summary -M` 的重命名识别结果。
9. 审计结果必须写入稳定文件，不能只放 commit message；历史设计包、上游来源、旧 Release Notes 的合法旧名称显式排除。
10. 计划不再宣称“零代码改动”：不改产品运行时代码，但会做 1 个合同脚本和 2 个测试清单的必要维护。
11. 新增对外营销与讲解视觉资产工作流：资产矩阵、提示词合同、品牌/版权边界、1 张当前候选样例及其被替代探索记录；批量生产其余资产仍需独立评审。
12. 补查远端后确认：本计划 v4 曾经 PR #9 合并，随后被 PR #10 显式 revert；`origin/main` 当前不包含该计划和样例。
13. 远端存在 `v0.3.1` 标签与 `origin/dev/laiyongjie` 开发线，其文档拓扑、合同脚本和手册正文已大幅变化；原 v4 的数量、路径和 Waves 1–4 不再可直接执行。
14. 新增目标基线 Gate：必须先在 `origin/main` 与 v0.3.1 开发线之间确定目标 SHA，再重跑基线、生成 delta 任务和验证命令。
15. 完成本地 VibeKey 历史材料审计，形成“继承 / 改造 / 放弃”决策；继续遵守 ADR-006，不复制旧生产代码，也不恢复硬件耦合。
16. 新增 VibeKey 经验驱动的主视觉样例。v1 保留近似 Logo 的失败记录；v2 验证空白徽章位、软件 token 和数据轨道，后因左右线路粗糙改为 `superseded`；v3 用统一网格重做六路输入和六路输出，成为当前 `concept_candidate`。
17. 将 README 现有截图中的 `CC Switch` 可见身份列为 P0 宣发可信度问题；任何正式 Hero 必须和真实 FyAgent 运行时截图成对出现。

---

## 1. 目标与成功标准

先锁定当前产品目标 SHA，再把公开 README、三语用户手册和开发者入口整理为该基线的现行文档体系，并建立可复用、可验证的对外营销/讲解视觉方向，同时保留 CC Switch 上游来源和历史决定的可追溯性。

完成时必须同时满足：

- 目标 Git SHA 被写入审计报告；所有数量、路径和验证命令均在该 SHA 上重新计算，禁止沿用 v4 旧基线。
- 德语 README、三语手册章节数、README 开发内容迁移方式以目标基线 delta 为准；若 v0.3.1 已实现同类迁移，不重复实施。
- 活动文档中的仓库链接、Deep Link 和数据目录使用 FyAgent 身份；历史/法律来源不被改写。
- 目标基线的全部截图和图片引用都有审计结论；README 首屏不再展示旧品牌截图。
- 未来重截目标有可执行 shot card；截图只有在真实运行时拍摄后才能标记为 `runtime_screenshot`。
- 对外视觉资产有明确用途矩阵、提示词模板、生成/确定性合成边界、VibeKey 对照审计和 1 张当前候选主视觉样例。
- 计划涉及的合同检查、目标测试和完整当前宿主质量门禁全部通过。

---

## 2. 已核实基线

下表记录 Gate 0 锁定后的目标事实。旧基线只用于解释计划怎样演进，不能再驱动实施。

| 项目                                            |                                     当前值 | 证据                                                                                       |
| ----------------------------------------------- | -----------------------------------------: | ------------------------------------------------------------------------------------------ |
| 规划工作分支                                    |                                 `ccde71d1` | `codex/docs-restructure-visual-plan`，只保存规划与营销样例，尚未迁入目标基线               |
| 当前远端 main                                   |                                 `ed20d04a` | PR #9 已被 PR #10 revert，不含本计划资产                                                   |
| 目标基线                                        |                                 `b6f60dfe` | `origin/dev/laiyongjie` 当前提交，包含 v0.3.1 标签后的仓库迁移                             |
| 目标三语手册 Markdown                           |                                         78 | zh/en/ja 各 26 个，含各语言索引                                                            |
| 目标三语图片引用                                |                                         84 | zh/en/ja 各 28 处；共享 PNG 共 40 张                                                       |
| 活动文档本地链接                                |                               372 / 0 失效 | 三份 README、三语手册和 12 份当前开发文档                                                  |
| 当前开发文档                                    |                                         12 | `docs/fyagent/development/`；`docs/fyagent/dev/` 在目标基线已删除                          |
| v4 旧基线                                       |        75 个既有章节、40 张 PNG、84 处引用 | 仅适用于 `3adc72ae` 附近，不得当作目标基线事实                                             |
| `docs/guides/` 文件                             |                                         22 | 其中 6 个旧名称命中均为上游来源说明                                                        |
| `deplink.html` / `flatpak/README.md` 旧名称命中 |                                      0 / 0 | 评审时 `rg` 检查                                                                           |
| 工具版本探测                                    |                                       7 个 | `TOOL_NAMES`                                                                               |
| 可安装/升级工具                                 |                                       6 个 | `LIFECYCLE_TOOLS`，排除 Codex                                                              |
| WorkBuddy 错误码                                |                                      22 个 | `WorkBuddyErrorCode`                                                                       |
| 目标产品线                                      |                              v0.3.1 开发线 | 文案避免把滚动开发分支写成已经发布的 Release                                               |
| README 截图身份                                 |                                     不合格 | 6 张中/英/日截图的 Git blob 均未变化，可见身份仍属于 `CC Switch`                           |
| VibeKey 本地归档                                |                              178,272 bytes | SHA-256 `9C54280EB1EB700800AB2022CEF32C392690ECB301D21DC7BBCB07A2BDE9F0C1`                 |
| 营销样例                                        | v1/v2 `superseded`；v3 `concept_candidate` | v3 为 1672×941，SHA-256 `C0EBE3C401B077DE804A37C3C0D4CC65000125121D4E19A366BF9FD2E5E78555` |

下表是 v4 在旧工作树上记录的 README 边界，只保留为历史证据。实施必须在 Gate 0 选定的 SHA 上按 `<summary>` 或标题语义重新定位，不得按这些行号切割：

| 文件           | 架构    | 开发指南 | 项目结构 |
| -------------- | ------- | -------- | -------- |
| `README.md`    | 233–274 | 276–401  | 403–442  |
| `README_ZH.md` | 233–274 | 276–395  | 397–436  |
| `README_JA.md` | 237–278 | 280–404  | 406–445  |

---

## 3. 范围

### Gate 0 — 目标基线与 delta 重算

2026-08-10，用户接受推荐方案 B。下表保留为决策记录：

| 选项                          | 适用条件                               | 影响                                                   |
| ----------------------------- | -------------------------------------- | ------------------------------------------------------ |
| A. `origin/main` / `ed20d04a` | 必须立即在当前主线完成 v0.3.0 文档收口 | 可继续使用部分 v4 任务，但需承认 v0.3.1 开发线尚未纳入 |
| B. v0.3.1 开发线 / `b6f60dfe` | 面向“现在的项目”和下一发布线开展       | **已选择**；delta 见稳定审计报告                       |

Gate 0 已完成：

- 目标 SHA、章节、图片、链接、旧身份命中、合同脚本和目标测试入口已复核。
- v4 动作的“保留 / 已完成 / 取消 / 改写”结论见
  `docs/fyagent/audits/docs-restructure-v0.3.0.md`。
- Preflight 已通过；实施分支保持目标基线祖先关系，并避开用户的
  `.omo/run-continuation/`。
- VibeKey 审计、提示词研究和 `concept` 样例可以保留在规划分支，但不得直接向 `main` 发布。

### Wave 1 — 身份与活动文件闭包

- 删除 `README_DE.md`。
- 从 `README.md`、`README_ZH.md`、`README_JA.md` 移除 Deutsch 链接。
- 从以下活动文件清单移除 `README_DE.md`：
  - `scripts/tasks/docs-contract-check.mjs` 的 `LEGACY_ENTRYPOINT_HANDOFF` 和 `activeDocs`；
  - `tests/localBuildBoundary.test.ts` 的 `CURRENT_DOCUMENTS`；
  - `tests/desktopSecurityBoundary.test.ts` 的 `activeWindowsInstallDocs`。
- `git mv session-manager.md docs/fyagent/history/session-manager-prd.md`，并在正文最前添加“历史 PRD、非当前产品合同”的醒目标记；原正文信息保持不变。
- 精简 `docs/user-manual/README.md` 和三语索引，删除容易过期的“当前亮点”；版本事实只链接对应 GitHub Release，不在手册索引维护滚动版本号。
- 审计 75 个既有章节中的仓库链接、Deep Link、数据路径和产品名；只修改仍把旧身份当作当前产品的内容，历史与上游来源进入白名单。
- 新建 `docs/release-notes/README.md`，说明 FyAgent 与上游历史版本边界。
- 审计 `docs/guides/`、`deplink.html`、`flatpak/README.md`，将结果写入 `docs/fyagent/audits/docs-restructure-v0.3.0.md`。

### Wave 2 — README 三语瘦身

- 保留目标基线已有的 12 份 `docs/fyagent/development/` 当前文档，不再另建按语言复制的架构、指南和结构文档。
- 三份根 README 删除重复的环境、构建、测试和技术栈长段，只保留面向贡献者的短入口。
- 英文 README 链接 `docs/fyagent/development/README.md`、活动 Trellis 规范和生成的 `mise-tasks.md`；中、日文 README 用本语言简短说明同一入口，不复制会持续变化的工程合同。
- 删除内容前逐段确认：如果某条工程信息在现有开发文档或规范中没有落点，先补到唯一责任文档，再从 README 删除。
- `mise-tasks.md` 是生成文件，本计划只链接，不手工修改。

### Wave 3 — 用户手册六章重组

- 60 个既有章节重编号并保留 Git 重命名识别。
- 新增三语章节：
  - `2-agent-tools/2.1-install.md`
  - `2-agent-tools/2.2-update-diagnose.md`
  - `4-extensions/4.6-workbuddy.md`
- 扩展三语 `4-extensions/4.3-skills.md`。
- 重写三语手册索引，更新全部路径链接、锚点和可见章节编号。
- 新章节不得引用尚不存在的未来截图；人物卡中的文件名是拍摄规格，不是可嵌入链接。

### Wave 4 — 截图审计、人物卡与验证

- 审计目标基线的全部现有截图和引用（v4 旧基线为 40 张 / 84 处），在审计报告中逐图记录：引用位置、语言、品牌/UI 状态、结论（保留/重截/未来本地化）、对应人物卡（如有）。
- 将 README 首屏截图的旧品牌身份作为 P0：先在目标 SHA 上重拍真实 FyAgent 主界面和添加供应商界面，再让 README 或营销页引用；不得用生图替代。
- 新建 `docs/user-manual/assets/shot-cards/README.md` 和 15 张人物卡。
- 运行手册结构、链接、品牌、内容无损、Git 重命名、合同检查、目标测试和完整质量门禁。

### Wave 5 — 对外营销与讲解视觉资产

- 以 `docs/fyagent/marketing/vibekey-reference-audit.md` 为历史参考边界，落实“继承 / 改造 / 放弃”，不得把硬件、语音、试用额度、众筹或 Claude-only 叙事带入当前产品。
- 新建 `docs/fyagent/marketing/visual-asset-plan.md`，记录受众、渠道、资产矩阵、尺寸/裁切、文案承载方式、优先级和负责人。
- 新建 `docs/fyagent/marketing/prompts/README.md`，沉淀主视觉、功能插图、讲解图和 UI 辅助插图的结构化提示词卡。
- 保留并区分三轮概念样例：
  - v1：`fyagent-unified-control-hero-v1.png` + `visual-direction-sample-v1.md`，状态 `superseded`；
  - v2：`fyagent-tactile-orchestration-hero-v2.png` + `visual-direction-sample-v2.md`，线路规则不一致，状态 `superseded`；
  - v3：`fyagent-tactile-orchestration-hero-v3.png` + `visual-direction-sample-v3.md`，统一六入六出线路网格，状态 `concept_candidate`。
- 对样例做构图、品牌一致性、第三方标识、文字、响应式裁切、文件体积和可访问性评审；决定保留、定向迭代或废弃。
- 正式 Hero 采用“生成背景 + 原始 Logo/文字确定性合成 + 真实 UI proof frame”；只有三部分一起评审通过才可发布。
- 本 Wave 只完成规划、提示词库、历史审计和 1 张当前候选主视觉样例；矩阵内其余正式资产另行排期，不以占位图冒充完成。

### Out of scope

- 除 README 三语主界面/添加供应商这 6 张 P0 proof frame 外，实际重截或替换用户手册截图；其余截图只审计并产出 shot card。
- 批量生成视觉资产矩阵中的全部正式图片，以及把概念样例直接发布到官网/商店/社媒。
- Grok Build 独立章节；它只在工具管理或供应商语境中同级说明。
- AgentsPanel 文档；当前仍是 Coming Soon。
- v3.16→v3.19 的上游功能正文搬运。
- 既有章节对 FyAgent 当前 UI 的逐章全面重写。
- 德语用户手册或德语 README 的后续维护。
- 产品运行时代码：`src/`、`src-tauri/`。
- `.trellis/`、`.agents/`、`.github/`。
- 目标基线已经删除的 `docs/fyagent/dev/**` 旧设计包；不得为了保留旧资料重新恢复。

### Must-NOT-Have

- 不删除或改写 `docs/release-notes/v3.*.md`、`docs/upstream/**`、`CHANGELOG.md` 中的历史事实。
- 不修改 `LICENSE`、`LICENSING.md`、`COMMERCIAL-LICENSE.md`、`THIRD_PARTY_NOTICES.md`。
- 不修改 `CONTRIBUTING.md`、`CODE_OF_CONDUCT.md`、`SECURITY.md`、`SUPPORT.md`。
- 不让仍然有效的工程合同因为 README 瘦身而失去唯一落点。
- 不保留重命名后旧章节路径的副本。
- 不在新增章节中写 TODO、占位符或虚构未实现能力。
- 不把计划中的源码行号当长期合同；以符号、类型和行为为准。

---

## 4. 设计决策

### 4.1 身份替换规则

活动替换范围仅为：

- `README.md`、`README_ZH.md`、`README_JA.md`；
- `docs/user-manual/README.md`；
- `docs/user-manual/{zh,en,ja}/**/*.md`。

精确替换：

| 原模式                              | 新模式                        | 规则                               |
| ----------------------------------- | ----------------------------- | ---------------------------------- |
| 旧的 `cc-switch` / 个人名下仓库链接 | `github.com/fy-agent/fyagent` | 当前项目仓库链接；上游来源链接除外 |
| `ccswitch://`                       | `fyagent://`                  | 当前 Deep Link                     |
| `~/.cc-switch/`                     | `~/.fyagent/`                 | 当前数据路径                       |
| `.cc-switch`                        | `.fyagent`                    | 其他当前路径语境                   |

`CC Switch`（带空格）按语境处理：

1. 链接、路径、协议中的当前产品身份：替换。
2. 根 README 法律声明、上游来源、历史版本事实：保留。
3. `docs/guides/` 中指向 `farion1231/cc-switch` 的上游 PR 和 v3.19.1 来源说明：保留并写入审计报告。
4. 其余指代当前产品的地方：替换为 `FyAgent`。

禁止用“全仓必须清零”验收。仓库已有大量冻结设计、上游来源和旧 Release Notes 合法命中；验收只扫描活动范围，并对活动范围内保留的 `CC Switch` 建立逐条白名单。

### 4.2 删除德语 README 的闭包

同一 Wave 内原子完成：根文件删除 → 三份语言切换器更新 → 合同脚本更新 → 两个测试清单更新 → 目标测试与合同检查。以下位置允许继续出现 `README_DE.md`，因为它们记录历史事实：

- `docs/release-notes/v3.16.0-*.md`；
- `.trellis/tasks/archive/**`。

### 4.3 README 瘦身与唯一事实来源

- 不按旧行号或旧 `<summary>` 结构切割；以目标基线的标题语义逐段判断。
- 产品定位、功能、安装入口和最短上手路径留在 README；工程环境、构建、测试、发布与架构细节回到现有 current-state 文档和 Trellis 规范。
- README 删除一段内容前，必须确认它已经有唯一、可点击、仍在维护的落点；没有落点就先补责任文档。
- 三种语言表达同一产品事实，但不要求逐字互译。中文要像在给真实用户说明，避免“全方位解决方案”一类空话；英文和日文也优先直接、可验证的表达。
- 三份根 README 最终都链接同一个 `docs/fyagent/development/README.md`，不再建立三套会漂移的工程说明。

### 4.4 六章结构

```text
docs/user-manual/{zh,en,ja}/
├── 1-getting-started/   5
├── 2-agent-tools/       2  NEW
├── 3-providers/         6  原 2-providers
├── 4-extensions/        6  原 3-extensions + WorkBuddy
├── 5-proxy/             5  原 4-proxy
└── 6-faq/               4  原 5-faq
```

每语 28 个章节；三语合计 84 个章节文件，不含各语言索引 README。

### 4.5 Rename Map

以下映射对 zh/en/ja 各执行一次：

```text
2-providers/2.1-add.md              → 3-providers/3.1-add.md
2-providers/2.2-switch.md           → 3-providers/3.2-switch.md
2-providers/2.3-edit.md             → 3-providers/3.3-edit.md
2-providers/2.4-sort-duplicate.md   → 3-providers/3.4-sort-duplicate.md
2-providers/2.5-usage-query.md      → 3-providers/3.5-usage-query.md
2-providers/2.6-claude-desktop.md   → 3-providers/3.6-claude-desktop.md

3-extensions/3.1-mcp.md             → 4-extensions/4.1-mcp.md
3-extensions/3.2-prompts.md         → 4-extensions/4.2-prompts.md
3-extensions/3.3-skills.md          → 4-extensions/4.3-skills.md
3-extensions/3.4-sessions.md        → 4-extensions/4.4-sessions.md
3-extensions/3.5-workspace.md       → 4-extensions/4.5-workspace.md

4-proxy/4.1-service.md              → 5-proxy/5.1-service.md
4-proxy/4.2-routing.md              → 5-proxy/5.2-routing.md
4-proxy/4.3-failover.md             → 5-proxy/5.3-failover.md
4-proxy/4.4-usage.md                → 5-proxy/5.4-usage.md
4-proxy/4.5-model-test.md           → 5-proxy/5.5-model-test.md

5-faq/5.1-config-files.md           → 6-faq/6.1-config-files.md
5-faq/5.2-questions.md              → 6-faq/6.2-questions.md
5-faq/5.3-deeplink.md               → 6-faq/6.3-deeplink.md
5-faq/5.4-env-conflict.md           → 6-faq/6.4-env-conflict.md
```

### 4.6 交叉引用更新

1. 路径和文件名使用一个映射表一次性转换；若执行者采用字符串替换，必须按 `5→6、4→5、3→4、2→3` 降序执行。
2. 对可见编号做同样的一次性映射，覆盖中文“第 N 章/N.M 节”、英文 `Chapter/Section`、日文 `第N章/N.M節` 及 Markdown 链接标签。
3. 用 `rg` 全仓查找指向 `docs/user-manual` 旧路径的反向引用；历史冻结目录只记录、不修改。
4. 更新后运行相对链接/图片路径解析检查，失效数量必须为 0。

### 4.7 新章节内容合同

#### `2.1-install.md` — 工具安装与版本状态

- 入口：设置 → 关于 → 工具管理。
- 7 个版本探测对象：Claude Code、Codex CLI、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes。
- 6 个可安装/升级对象：除 Codex CLI 外的其余工具；明确说明 Codex 卡片只探测版本。
- 一键命令复制、WSL shell/flag、安装后版本刷新与验证。
- 源码合同：`TOOL_NAMES`、`LIFECYCLE_TOOLS`、`ONE_CLICK_INSTALL_COMMANDS`、`handleCopyInstallCommands`、`refreshToolVersions`。

#### `2.2-update-diagnose.md` — 升级与安装冲突诊断

- 版本与 latest 探测、单工具/批量串行升级、每个工具独立成败。
- 更新前多安装位置探测与确认。
- 全量诊断覆盖 7 个探测对象；安装/升级动作只覆盖 6 个生命周期工具。
- 升级后补诊；硬失败、版本未变、安装后不可运行三类结果。
- 源码合同：`probeToolInstallations`、`handleDiagnoseAll`、`executeRun`、`handleRunToolAction`。

#### `4.6-workbuddy.md` — WorkBuddy 模型配置注入

- WorkBuddy 定位和顶层应用切换入口。
- Base URL、API Key 显示/隐藏、允许无 Key、HTTP 非加密警告。
- 获取模型、截断提示、客户端搜索过滤、勾选/全选/手工模型 ID。
- revision 并发保护、overwrite token 二次确认、已有模型状态。
- 错误表覆盖 `WorkBuddyErrorCode` 当前 22 个码，按 URL/鉴权/网络响应/配置读写/并发覆盖/内部错误分组，不宣称固定数量是永久合同。
- 源码合同：`buildSaveRequest`、`handleFetch`、`handleSave`、`WorkBuddyErrorCode`、`WorkBuddySaveModelsResult`。

#### `4.3-skills.md` 扩展

- GitHub 下载（60 秒超时）→ `~/.fyagent/skills/` SSOT → 数据库记录/内容哈希 → 应用目录同步。
- 同步策略必须写成“按配置使用 symlink 或 copy；Auto 优先 symlink、失败回退 copy”，不能笼统写成永远使用软链接。
- 内容哈希更新检测、手动检查、更新标签。
- `~/.fyagent/skill-backups/`、最近 20 个备份、卸载顺序。
- 路径消毒、目录逃逸、归档/符号链接防护、同名冲突。
- 自定义仓库、启停、skills.sh 公共目录。
- 源码合同：`install`、`uninstall`、`check_updates`、`create_uninstall_backup`、`sync_to_app_dir`、路径与归档防护函数。

### 4.8 Release Notes 索引

`docs/release-notes/README.md` 至少包含：

| 范围                         | 产品      | 说明                   |
| ---------------------------- | --------- | ---------------------- |
| v0.3.0+                      | FyAgent   | 独立版本体系           |
| 仓库现存 v3.6.0–v3.19.1 文件 | CC Switch | 上游历史 Release Notes |

并说明：FyAgent v0.3.0 的源码基线包含 CC Switch v3.19.2；该来源记录位于 `docs/upstream/cc-switch-v3.19.2.md`，仓库当前没有 v3.19.2 Release Note 文件，不得虚构。

### 4.9 截图人物卡

- 现有 18 张 Claude Desktop 本地化截图继续保留 `-en` / `-ja`；不得用“en/ja 全部复用中文截图”覆盖既有事实。
- 其余 22 张无语言后缀截图当前由三语手册共用。
- 未来重截默认先产出中文裸文件名；英文/日文是否本地化由审计报告逐图决定。
- 人物卡命名：`NNN-<image-name>.md`；每张至少包含章节、目标文件名、尺寸、主题、语言、前置数据、界面状态、必显元素、隐私/脱敏要求、验收方式。

15 张人物卡：

|   # | 章节 | 目标文件名                    | 主题               |
| --: | ---- | ----------------------------- | ------------------ |
| 001 | 1.3  | `main-overview.png`           | 主界面全景         |
| 002 | 1.4  | `quickstart-add-provider.png` | 添加供应商流程     |
| 003 | 1.5  | `settings-general.png`        | 设置页通用区       |
| 004 | 2.1  | `about-tool-install.png`      | 工具安装区         |
| 005 | 2.2  | `about-diagnose-conflict.png` | 冲突诊断结果       |
| 006 | 3.1  | `provider-card-list.png`      | 供应商列表         |
| 007 | 3.3  | `provider-edit-form.png`      | 编辑供应商         |
| 008 | 4.1  | `mcp-panel.png`               | MCP 管理           |
| 009 | 4.2  | `prompts-editor.png`          | 提示词编辑器       |
| 010 | 4.3  | `skills-panel.png`            | Skills 管理        |
| 011 | 4.4  | `sessions-list.png`           | 会话列表           |
| 012 | 4.6  | `workbuddy-connection.png`    | WorkBuddy 连接     |
| 013 | 4.6  | `workbuddy-models.png`        | WorkBuddy 模型选择 |
| 014 | 5.1  | `proxy-service.png`           | 代理服务           |
| 015 | 5.3  | `failover-queue.png`          | 故障转移队列       |

### 4.10 对外营销与讲解视觉系统

视觉原型锁定为 **Developer Tool / AI Product**：深石墨背景、精确网格、单一青绿/电蓝高亮信号、少量暖橙状态点、克制的 3D 工程材质。禁止回退到 `Inter + 灰卡片 + 紫色渐变` 的通用 SaaS 模板。

VibeKey 迁移规则：

| 决策 | 内容                                                                                |
| ---- | ----------------------------------------------------------------------------------- |
| 继承 | “一个动作、一个状态、一个结果”的可读性；首次使用四步路径；90 秒内讲清价值           |
| 改造 | 触觉控制感转译为软件 token、选择器和数据轨道；“插上即用”改为“一处管理、快速切换”    |
| 放弃 | 键盘/屏幕/旋钮硬件、语音输入、Claude-only、试用额度、众筹、定价和未经验证的市场数字 |

真实证据规则：README、官网和发布文章的 Hero 下方必须出现至少一个来自目标 SHA 的 FyAgent 真实运行时 proof frame。概念插图只解释关系，不能证明功能；截图仍显示 `CC Switch` 时，不得发布新的 FyAgent 宣发页。

设计 token：

| Token              | 值/规则                              |
| ------------------ | ------------------------------------ |
| `bg`               | `#0B1017` 深石墨                     |
| `surface`          | `#121A26` / `#EEF5FA` 深浅两级表面   |
| `accent-primary`   | `#27D9C4` 青绿                       |
| `accent-secondary` | `#2F7DFF` 电蓝                       |
| `signal`           | `#FF9D2E`，只作少量状态点            |
| `radius`           | 圆形连接器 + 12–20 px 面板圆角       |
| `lighting`         | 单一柔和棚拍光，克制边缘高光         |
| `texture`          | 哑光石墨、雾面浅色表面、抛光连接管线 |

优先资产矩阵：

| 优先级 | 资产                   | 主要用途                   | 画幅      | 生产方式                                          |
| ------ | ---------------------- | -------------------------- | --------- | ------------------------------------------------- |
| P0     | 统一管理主视觉         | README、官网首屏、发布文章 | 16:9      | ChatGPT 生图概念 + 原始 Logo/文案确定性合成       |
| P0     | OG / 社媒横图          | GitHub、X、公众号分享卡    | 1200×630  | 主视觉安全裁切 + 确定性标题                       |
| P1     | 多工具统一管理讲解图   | 产品介绍、路演             | 16:9      | `infographic-diagram` 无文字底图 + SVG/HTML 标签  |
| P1     | 安装/升级/冲突诊断插图 | 2.1、2.2 章节与功能营销    | 3:2       | `stylized-concept`，真实能力由源码合同约束        |
| P1     | Skills 生命周期插图    | 4.3 章节                   | 3:2       | 下载→SSOT→同步→备份流程；文字后置                 |
| P1     | WorkBuddy 模型注入插图 | 4.6 章节                   | 3:2       | 连接→获取→选择→写入流程；文字后置                 |
| P1     | 本地优先与配置安全图   | 官网“为什么选择”/演示稿    | 16:9      | 结构化讲解图 + 确定性数据/标签                    |
| P2     | 发布海报               | 中文社区、更新公告         | 4:5 / 1:1 | 主视觉变体 + 版本文案后置                         |
| P2     | 空状态/引导插图组      | 文档、未来 UI 候选         | 4:3       | 只生成插图主体；按钮、图标、控件保持代码/矢量原生 |

提示词合同：

1. 使用 `ads-marketing`、`infographic-diagram`、`stylized-concept` 或 `ui-mockup` 等明确 use case，不写一句话式模糊提示词。
2. 每张提示词卡包含用途、受众、画幅、场景、主体、构图、色板、材质、必保留项、禁用项和后期合成项。
3. 生图阶段默认不生成正文、按钮文字、流程标签或第三方 Logo；准确文字与原始 FyAgent Logo 使用 SVG/HTML/设计工具确定性合成。
4. 如果使用 `assets/fyagent.png` 作为参考，必须标注“项目自有品牌参考”；不得把第三方产品图标喂给模型后生成近似商标。
5. UI 截图属于真实运行时证据，不用生图替代；生图只能做概念插图、背景和讲解场景。
6. 每个输出保存提示词、模型路径（built-in/CLI）、参考图、尺寸、SHA-256、评审状态和已知限制。
7. 历史参考图必须登记来源与 SHA-256，并说明只借鉴哪些属性；来源不清或与文档决策冲突时，只能作内部 concept 参考。
8. 任何会让软件被误解为实体硬件、USB Hub、键盘或终端设备的输出必须定向迭代或淘汰。

三轮样例分别见 `docs/fyagent/marketing/visual-direction-sample-v1.md`、`visual-direction-sample-v2.md` 和 `visual-direction-sample-v3.md`。v1 因生成近似 Logo、v2 因线路和端口规则不一致被标记为 `superseded`；v3 保留空白徽章位并统一六入六出线路网格，是当前 `concept_candidate`。正式发布时仍必须以原始 `assets/fyagent.png` 确定性合成并复核清晰度。

---

## 5. 实施顺序与依赖

### Gate 0

0. [x] 目标基线锁定为 `b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527`。
1. [x] 在该提交的只读快照重跑文档、图片、链接、品牌命中和测试入口盘点。
2. [x] 将 v4 动作标记为“保留 / 已完成 / 取消 / 改写”，写入稳定 delta 报告。

### Preflight

3. [x] 用户已信任仓库，portable `mise 2026.8.2` 可用。
4. [x] `mise run bootstrap` 通过。
5. [x] 已检查工作树并持续避开用户的 `.omo/run-continuation/`。
6. [x] 已重跑基线计数，并按实际结果更新审计。

### Wave 1

1. 原子完成德语 README 删除闭包，并立即运行目标测试与 `check:contracts`。
2. 移动 `session-manager.md` 并添加历史标记。
3. 精简 4 个手册索引 README，删除会快速过期的亮点清单。
4. 审计 75 个现行章节，只修正仍把旧身份当作当前产品的内容。
5. 新建 Release Notes 索引。
6. 写入 guides/deplink/flatpak 审计结果。

### Wave 2

7. 核对三份 README 中环境、构建、测试、发布和技术栈内容在现有 current-state 文档或规范中的唯一落点。
8. 如有缺口，补充对应责任文档；不创建按语言复制的工程合同。
9. 删除三份 README 的重复长段，改成自然、简短、可点击的贡献入口。
10. 复核三语产品事实、开发入口和相对链接一致。

### Wave 3

11. 按 Rename Map 移动 60 个文件。
12. 新建 9 个章节文件并扩展 3 个 Skills 章节。
13. 重写三语手册索引。
14. 一次性更新路径、文件名和可见编号。
15. 运行旧路径扫描和相对链接解析；失败先修复再进入 Wave 4。

### Wave 4

16. [x] 审计 40 张图片与 84 处引用并写入稳定报告。
17. [x] 新建 shot-cards README + 15 张人物卡。
18. 运行最终验证、独立复读和 Git diff 检查。README 已停止引用旧图；真实三语重拍在
    数据固定、脱敏和运行时验收准备完成后按 shot card 独立执行，不用生成图占位。

### Wave 5

19. 复核 VibeKey 历史审计的来源、哈希、证据冲突和“继承 / 改造 / 放弃”边界。
20. 写入视觉资产矩阵与渠道/尺寸/裁切合同。
21. 写入至少 4 类结构化提示词卡：主视觉、功能插图、讲解图、UI 辅助插图。
22. 登记 v1/v2 `superseded` 和 v3 `concept_candidate` 的提示词、参考图、尺寸、SHA-256、限制与替代关系。
23. 完成 v3 桌面安全区和线路完整性评审；移动画幅、原始 Logo 合成和真实 proof frame 配对仍是从
    `concept_candidate` 升级为可发布资产的门禁，本任务不直接发布。
24. 将未生产的正式资产转成后续任务，不在本计划内批量生成。

依赖关系：

```text
Gate 0 → Preflight → Wave 1 → Wave 2 → Wave 3 → Wave 4 → Wave 5
```

Wave 内也存在明确顺序；不得再假设“Wave 内 todos 无依赖”。

---

## 6. 收口清单

C1–C8 是 v4 问题域的条件项。若目标基线已经完成或取消某项，必须将其标为 `N/A` 并链接 delta 证据；不得为了“勾选完成”重复改动。

- [x] C0. 目标分支与 Git SHA 已锁定；基线重算和 v4 delta 清单已写入稳定报告。
- [x] C1. 删除 DE README 的文件、链接、合同脚本和测试依赖闭包完成。
- [x] C2. 4 个手册索引 README 已精简，三语滚动亮点段已删除，版本事实回到 Release。
- [x] C3. 75 个既有章节的活动身份审计完成，必要修正和历史事实白名单已记录。
- [x] C4. Session Manager 孤儿 PRD 已移动并标为历史，不再冒充当前合同。
- [x] C5. Release Notes 索引和综合审计报告存在。
- [x] C6. 三份根 README 已瘦身，并链接目标基线现有的 current-state 开发文档；有效工程信息没有失去唯一落点。
- [x] C7. 每语 28 个章节、旧目录不存在、三语索引与文件系统一致。
- [x] C8. 提交 `a12ef395` 将 60 个迁移文件识别为 100% rename，旧路径无副本。
- [x] C9. 40 张截图与 84 处引用均有审计结论，16 个 shot-card Markdown 存在；README
      已移除旧品牌截图引用，并明确不以概念图冒充运行时证据。真实三语重拍作为独立拍摄阶段保留。
- [x] C10. 活动范围品牌、版本、旧章节路径和本地链接检查全部通过。
- [x] C11. 目标测试、`check:contracts`、完整 `mise run check` 和 `git diff --check` 全部通过。
- [x] C12. 视觉资产矩阵、至少 4 类提示词卡和 1 张当前候选样例完成；v1/v2/v3 状态、替代关系与发布前限制明确。
- [x] C13. VibeKey 对照审计存在；所有借鉴点均标记为继承、改造或放弃，且没有恢复旧硬件/授权耦合。

---

## 7. 验收与验证

### 7.1 活动身份与 DE 闭包

```powershell
rg -n -i 'cc-switch|ccswitch|\.cc-switch' docs/user-manual README.md README_ZH.md README_JA.md
rg -n 'v3\.16\.0' docs/user-manual
rg -n --fixed-strings 'README_DE.md' README.md README_ZH.md README_JA.md scripts tests
```

预期：均无输出。`rg` 的 exit code 1 表示“无匹配”，在此为成功结果。

另行运行：

```powershell
rg -n --fixed-strings 'CC Switch' docs/user-manual README.md README_ZH.md README_JA.md
```

预期：只出现审计报告批准的法律/历史语境；每个命中均人工复核。

### 7.2 文件拓扑

```powershell
if (Test-Path 'README_DE.md') { throw 'README_DE.md still exists' }
if (Test-Path 'session-manager.md') { throw 'root session-manager.md still exists' }
if (-not (Test-Path 'docs/fyagent/history/session-manager-prd.md')) { throw 'moved session PRD missing' }

foreach ($lang in 'zh','en','ja') {
  $count = (Get-ChildItem "docs/user-manual/$lang" -Recurse -File -Filter '*.md' |
    Where-Object Name -ne 'README.md').Count
  if ($count -ne 28) { throw "$lang chapter count: $count" }
}

foreach ($lang in 'zh','en','ja') {
  foreach ($old in '2-providers','3-extensions','4-proxy','5-faq') {
    if (Test-Path "docs/user-manual/$lang/$old") { throw "stale dir: $lang/$old" }
  }
}

$cards = (Get-ChildItem 'docs/user-manual/assets/shot-cards' -File -Filter '*.md').Count
if ($cards -ne 16) { throw "shot-card count: $cards" }
```

### 7.3 旧路径与反向引用

```powershell
rg -n '2-providers/|3-extensions/|4-proxy/|5-faq/' docs/user-manual
rg -n 'user-manual.*(2-providers|3-extensions|4-proxy|5-faq)' . -g '*.md' -g '!.trellis/tasks/archive/**'
```

预期：无活动命中。

### 7.4 内容与链接完整性

- 三份 README 删除的工程细节都能在 current-state 文档或规范中找到唯一落点。
- 三语索引目录树与实际目录逐项一致。
- 所有 Markdown 相对链接和图片路径解析到现有文件，失效数为 0。
- 40 个现有 PNG 文件名全部出现在截图审计表；报告汇总引用数为 84。
- 新章节未嵌入不存在的未来截图路径。

### 7.5 Git 重命名与仓库门禁

```powershell
git diff --summary -M
git diff --check
mise run test:unit -- tests/localBuildBoundary.test.ts tests/desktopSecurityBoundary.test.ts
mise run check:contracts
mise run check
```

预期：

- `git diff --summary -M` 将 60 个既有章节识别为 rename；若相似度因内容扩展降低，先拆分“纯移动”和“正文修改”两个提交再复核。
- `git diff --check` 无 whitespace 错误。
- 两个目标测试、合同门禁和完整当前宿主门禁均通过。

提交后用 `git show --summary -M <commit>` 复核 rename；不要声称 Git 保存了 `git mv` 命令本身。

### 7.6 营销视觉样例

```powershell
$sample = 'docs/fyagent/marketing/assets/samples/fyagent-tactile-orchestration-hero-v3.png'
if (-not (Test-Path $sample)) { throw 'marketing sample missing' }
Get-FileHash -Algorithm SHA256 $sample
```

预期：

- v3 样例尺寸为 1672×941，约 16:9；左侧具备标题安全区，右侧表达“六路配置入口 → 一个控制中枢 → 六路工具结果”。
- 无内嵌文字、水印、第三方 Logo 或生成 Logo；中心徽章位为空，实际发布前用项目原始 Logo 做确定性合成。
- SHA-256 为 `C0EBE3C401B077DE804A37C3C0D4CC65000125121D4E19A366BF9FD2E5E78555`。
- `visual-direction-sample-v3.md` 保存定向修订提示词、编辑目标、生成方式、线路评审、已知限制和发布前工作。
- `visual-direction-sample-v1.md` 和 `visual-direction-sample-v2.md` 仍存在且状态为 `superseded`，不得被误当当前候选。

### 7.7 当前基线与 VibeKey 边界

```powershell
git rev-parse --verify b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527
git merge-base --is-ancestor b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527 HEAD
rg -n "status: completed|Gate 0|vibekey-reference-audit|concept_candidate" .omo/plans/docs-restructure-v0.3.0.md docs/fyagent/marketing
```

预期：

- 审计报告记录精确目标 SHA、Preflight 结果和最终门禁证据。
- `vibekey-reference-audit.md` 记录本地归档 SHA-256、证据冲突和继承/改造/放弃决策。
- 活动宣发文案中没有 VibeKey 的硬件、语音、试用额度、众筹、定价或官方合作承诺。

---

## 8. 审计报告最低结构

`docs/fyagent/audits/docs-restructure-v0.3.0.md` 至少包含：

1. 基线计数和执行日期。
2. `docs/guides/`、`deplink.html`、`flatpak/README.md` 的每个旧名称命中、语境和处理结论。
3. 活动范围 `CC Switch` 白名单。
4. 三份 README 被删开发段落到 current-state 文档/规范的责任映射。
5. 主报告汇总截图结论，并链接独立的 40 行逐图审计表；表中记录文件名、84 处引用
   位置汇总、语言、品牌/UI 状态、结论和人物卡。
6. 最终命令、exit code、关键输出摘要和证据等级。
7. 目标 Git SHA、v4 动作 delta（保留/已完成/取消/改写）及其依据。
8. VibeKey 审计路径、本地归档 SHA-256、迁移决策与禁止迁移项。
9. v1/v2/v3 营销样例的路径、提示词文档、SHA-256、状态、替代关系、视觉评审结论与是否获准发布。

审计报告是交付物；commit message 只链接它，不承载唯一证据。

---

## 9. 风险与回退

| 风险                                          | 影响                                     | 缓解/回退                                                               |
| --------------------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------- |
| 删除 DE README 未更新依赖清单                 | 合同检查/测试读文件失败                  | 同 Wave 原子修改 1 个脚本 + 2 个测试，先跑目标门禁                      |
| 三语 README 瘦身后入口不清楚                  | 中文/日文读者找不到工程资料              | 三语保留本语言短说明，并链接同一 current-state 入口                     |
| 连续替换造成 2→3→4→5→6 级联                   | 路径和章节号错乱                         | 一次性映射或降序替换；旧路径扫描 + 链接解析                             |
| 大量正文修改降低 rename 相似度                | Git 历史难追踪                           | 纯移动与正文修改分提交；用 `-M` 复核                                    |
| 当前产品替换误伤上游/法律事实                 | 来源失真                                 | 活动范围 + 逐条白名单；历史 Release Notes 和上游来源不改                |
| Skills 文档把 copy 回退写成“总是 symlink”     | 用户预期错误                             | 按 `sync_to_app_dir` 的 Auto/Symlink/Copy 行为写作                      |
| 新章节引用未来截图                            | 文档立即出现 404                         | 本计划只写人物卡，不嵌入未生成图片                                      |
| 截图审计发现大量需重截                        | 后续工作量扩大                           | 本计划只分类并产出人物卡；实际拍摄拆成后续任务                          |
| 生成式 Logo/文字变形                          | 对外品牌失真、可读性差                   | 生图只做概念主体；原始 Logo 和准确文字后期确定性合成                    |
| 生成近似第三方商标或虚构 UI                   | 法务/信任风险                            | 使用通用几何节点；发布前人工检查；真实 UI 只用运行时截图                |
| 目标基线在 v0.3.0 main 与 v0.3.1 开发线间漂移 | 旧计划重复删除、移动或改写已经变化的文档 | Gate 0 锁定 SHA；重算 delta；旧动作逐项取消或改写                       |
| README 截图仍显示 `CC Switch`                 | FyAgent 宣发自相矛盾                     | 真实运行时重拍为 P0；新 Hero 不得绕过 proof frame 门禁                  |
| VibeKey 概念图被当作实物或当前产品            | 恢复错误硬件叙事                         | 登记历史来源与矛盾；只迁移控制感，不迁移产品形态                        |
| 一张样例被误当完整营销系统                    | 资产覆盖不足                             | 样例标为 concept；矩阵未完成项必须进入后续任务                          |
| 实施环境缺少 `mise`                           | 无法提供质量门禁证据                     | 已用校验过的 portable mise 完成 Preflight；仍只使用 canonical task 入口 |

回退按 Wave 进行；每个 Wave 保持独立提交。文件移动和内容编辑分开提交时，优先回退当前 Wave，不回退无关用户改动。

---

## 10. 完成定义

只有 C0–C13 全部勾选、审计报告写入真实结果、最新验证命令全部通过，计划才转为
`completed`。本计划评审证据等级为 `code_audit + local_artifact_audit`，概念样例另做了
`generated_asset_visual_inspection`；实际 UI 截图重拍完成前仍不得宣称
`runtime_screenshot` 或 `pixel_diff` 验收，概念样例也不得冒充真实 UI 证据。
