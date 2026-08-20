---
type: audit
status: complete
updated: 2026-08-11
review_on: 2026-09-11
authority: git:b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527
source: git:b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527
evidence: code_audit + local_artifact_audit
---

# FyAgent 文档重构执行审计

## 结论

本轮以 `origin/dev/laiyongjie` 的
`b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527` 为基线，完成了公开入口、三语用户手册、
历史资料和营销视觉规划的重构。当前分支仍包含该基线，产品运行时代码、许可证、社区
治理文件和用户的 `.omo/run-continuation/` 都没有进入本轮改动。

现在的公开入口只保留中、英、日三种语言。三份 README 不再拿旧品牌截图证明 FyAgent，
也不再各自保存一套容易过期的工程说明。用户手册统一为六章，每种语言 28 篇正文；旧图
没有被悄悄当成新图，而是全部登记为待真实重拍。对外视觉部分已经有资产矩阵、提示词卡、
VibeKey 对照审计和一张当前候选样例，但候选样例仍是概念图，不是产品界面或发布成品。

## 执行边界

| 项目           | 结果                                                                             |
| -------------- | -------------------------------------------------------------------------------- |
| 目标基线       | `b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527`                                       |
| 工作分支       | `codex/docs-restructure-current`                                                 |
| 基线祖先检查   | 通过，`git merge-base --is-ancestor` 退出码 0                                    |
| 产品运行时代码 | `src/`、`src-tauri/` 未修改                                                      |
| 法律与社区文件 | 未修改                                                                           |
| 用户目录       | `.omo/run-continuation/` 未修改、未暂存                                          |
| 运行环境       | portable `mise 2026.8.2`；仓库已信任；`bootstrap` 与 Windows `system:check` 通过 |

首次合同检查还暴露了两个原有 Windows 可移植性缺口：`.mjs` 没有固定 LF，Vitest 会在
CRLF shebang 上解析失败；macOS shell 合同测试在 Windows 上错误调用兼容启动器，并
直接读取 NTFS 的 Unix 执行位。修正只落在 `.gitattributes`、任务脚本分类和测试层，没有
改变产品功能。完整性测试的 CRLF fixture 也改为先归一化再转换，避免生成 `CRCRLF`。

## 公开入口

### README

- 删除 `README_DE.md`，并清理三份语言导航、文档合同脚本和边界测试中的活动依赖。
- 中、英、日 README 都改为：产品定位 → 能做什么 → 安装与信任说明 → 最短上手 →
  开发入口 → 项目历史与许可证。
- 三份 README 均保留 `mise run dev`、`mise run build` 和 `mise run check` 入口，详细环境、
  架构、测试和发布合同统一交给
  [current-state 开发文档](../development/README.md)与 Trellis 规范。
- 6 张旧 `assets/screenshots/*` 图片仍保留在仓库，但根 README 已没有引用。没有用生成图、
  网页预览或临时画面替代真实 FyAgent 运行时证据。

### 历史与发布记录

- 根目录 Session Manager PRD 已移到
  [历史资料](../history/session-manager-prd.md)，首屏说明它不是当前产品合同。
- 新增 [Release Notes 索引](../../release-notes/README.md)，解释 FyAgent v0.3.x 与
  CC Switch v3.6–v3.19.1 历史记录的关系。
- `docs/guides/` 中保留 6 处有证据的旧名称：3 处上游 PR #5071，3 处 CC Switch
  v3.19.1 来源说明。`deplink.html` 没有旧身份命中。

## 三语用户手册

### 最终拓扑

| 语言    | 索引 | 正文 | 章节分布              |
| ------- | ---: | ---: | --------------------- |
| 中文    |    1 |   28 | 5 + 2 + 6 + 6 + 5 + 4 |
| English |    1 |   28 | 5 + 2 + 6 + 6 + 5 + 4 |
| 日本語  |    1 |   28 | 5 + 2 + 6 + 6 + 5 + 4 |

旧目录 `2-providers`、`3-extensions`、`4-proxy`、`5-faq` 已全部消失。提交
`a12ef395` 把三种语言各 20 个旧章节、共 60 个文件识别为 `R100`，保留了纯移动证据。

每种语言新增：

- `2-agent-tools/2.1-install.md`
- `2-agent-tools/2.2-update-diagnose.md`
- `4-extensions/4.6-workbuddy.md`

三份 `4-extensions/4.3-skills.md` 同步补充了六个应用、`~/.fyagent/skills` 单一事实来源、
60 秒同步、Auto/Symlink/Copy 行为、安全边界和最近 20 份备份。四个索引都删掉了滚动的
“当前亮点”，改为从用户要完成的任务进入。

### 活动身份白名单

活动手册中没有 `cc-switch` 仓库、`ccswitch://` 或 `.cc-switch` 数据路径。允许保留的
`CC Switch` 只有以下两类：

1. 三份根 README 的项目来源与许可证说明；
2. 三语手册索引对旧截图可见身份的明确警告。

负向合同测试中可以出现字符串 `README_DE.md`，因为它用于断言文件不存在；这不是活动
导航依赖。

## 截图证据

[用户手册截图审计](user-manual-screenshots.md)保存了 40 张 PNG 的逐图表格，包括尺寸、
SHA-256 前缀、引用章节、当前语言和重拍结论。汇总结果为：

| 项目               | 数量 | 结论                    |
| ------------------ | ---: | ----------------------- |
| 手册 PNG           |   40 | 全部 `replace_required` |
| 三语引用           |   84 | `18 × 1 + 22 × 3`       |
| shot-card Markdown |   16 | 1 个索引 + 15 张任务卡  |
| README 旧图引用    |    0 | 已停止使用              |

抽查覆盖主界面、供应商、Skills、用量和 Claude Desktop 三语截图，能够看到旧品牌、旧
导航或中文界面被跨语言复用。证据等级是 `code_audit + sampled_visual_review`。当前主机
已通过 Visual Studio Developer PowerShell、`cl.exe` 与 WebView2 预检，但本轮没有准备
完成三语固定数据、脱敏和运行时逐图验收，因此没有产生或声称 `runtime_screenshot`。
后续按 [shot cards](../../user-manual/assets/shot-cards/README.md)独立重拍。

## 营销、讲解图与 VibeKey 对照

交付物包括：

- [视觉资产计划](../marketing/visual-asset-plan.md)：受众、渠道、画幅、优先级、生成层与
  真实证据层、品牌和版权门禁；
- [提示词库](../marketing/prompts/README.md)：Hero、多工具说明、工具生命周期、Skills
  生命周期、WorkBuddy、空状态六类卡片，以及单变量修订模板；
- [VibeKey 宣发与产品设计审计](../marketing/vibekey-reference-audit.md)：继承“一个动作、
  一个状态、一个结果”，改造成软件控制面，放弃硬件、语音、Claude-only、众筹、定价
  和未经验证的市场数字；
- [能力差距审计](vibekey-to-fyagent-capability-gap.md)。

概念样例状态保持清楚：

| 样例                                        | 状态                | 结论                                                               |
| ------------------------------------------- | ------------------- | ------------------------------------------------------------------ |
| `fyagent-unified-control-hero-v1.png`       | `superseded`        | 生成近似 Logo，仅保留探索记录                                      |
| `fyagent-tactile-orchestration-hero-v2.png` | `superseded`        | 构图和材质方向可用，但左右线路、端口和弯折规则不一致               |
| `fyagent-tactile-orchestration-hero-v3.png` | `concept_candidate` | 1672×941；统一六入六出线路网格；左侧标题安全区和中心空白徽章位保留 |

v3 SHA-256 为
`C0EBE3C401B077DE804A37C3C0D4CC65000125121D4E19A366BF9FD2E5E78555`。
正式发布前仍需使用未经修改的 `assets/fyagent.png` 做确定性合成，补齐准确文字、移动画幅、
压缩和可访问性检查，并与至少一张真实 FyAgent proof frame 配对。当前未获准发布。

## 验证记录

| 命令 / 检查                                        | 结果                                                                                          | 证据等级                  |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------- |
| `mise run bootstrap`                               | 通过                                                                                          | `local_runtime_check`     |
| `mise run system:check`（VS Developer PowerShell） | Git、`cl.exe`、WebView2 通过                                                                  | `local_runtime_check`     |
| 4 个聚焦测试文件                                   | 43 项通过                                                                                     | `code_audit`              |
| Windows/CRLF 回归测试                              | 66 项通过                                                                                     | `code_audit`              |
| 手册拓扑                                           | zh/en/ja 各 28；旧目录 0                                                                      | `code_audit`              |
| 图片清单                                           | 40 个文件、84 处引用、16 个 shot-card Markdown                                                | `code_audit`              |
| 活动旧路径扫描                                     | 0 个命中                                                                                      | `code_audit`              |
| 本地 Markdown 链接                                 | 由 `currentDocsContract` 检查，0 个失效                                                       | `code_audit`              |
| `mise run check:contracts`                         | 退出码 0；25 个测试文件、626 项通过、3 项按合同跳过，另有 4 项原生 Fetch 测试通过             | `code_audit`              |
| `mise run check`                                   | 退出码 0；耗时 528.2 秒，TypeScript、格式、全量 Vitest、合同、Rust check/clippy/test 全部通过 | `local_host_quality_gate` |
| `git diff --check`                                 | 退出码 0                                                                                      | `code_audit`              |

静态检查和概念图评审不能证明真实安装、运行时界面、签名或发布。本轮完成的是
`code_audit + local_host_quality_gate`；真实三语截图仍需在独立拍摄阶段提供
`runtime_screenshot` 证据。
