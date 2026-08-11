# 文档重构与视觉资产规划设计

## 1. 责任边界

文档事实按下面的顺序归属：

```text
源码 / 测试 / Trellis 规范
        ↓
docs/fyagent/development/ current-state 文档
        ↓
三语 README（入口与概览）
        ↓
三语用户手册（用户任务说明）
        ↓
营销概念图与讲解插图（关系表达，不证明功能）
```

README 不再保存会频繁变化的工程合同；手册不承担发布日志；概念图不承担运行时证据。

## 2. 文件与迁移边界

### 2.1 README 闭包

- 删除 `README_DE.md`。
- 同步修改 `README.md`、`README_ZH.md`、`README_JA.md`、
  `scripts/tasks/docs-contract-check.mjs`、`tests/localBuildBoundary.test.ts` 和
  `tests/desktopSecurityBoundary.test.ts`。
- 三份 README 保留同样的信息层级，但按各自语言自然表达。
- 现有 `docs/fyagent/development/README.md` 是唯一工程入口；不新建 `en/zh/ja` 三套开发
  文档。

### 2.2 历史材料

- `session-manager.md` 迁至 `docs/fyagent/history/session-manager-prd.md`。
- 历史标记与正文移动分开处理，便于 Git 识别 rename。
- `docs/release-notes/README.md` 只解释版本边界和导航，不改写任何既有 v3.x 文件。

### 2.3 手册拓扑

对中、英、日使用同一映射表：

```text
2-providers    → 3-providers
3-extensions   → 4-extensions
4-proxy        → 5-proxy
5-faq          → 6-faq
```

先完成纯移动，再统一改文件内标题、编号和链接，避免连续字符串替换产生
`2→3→4→5→6` 级联。新增 `2-agent-tools` 两篇和 `4.6-workbuddy.md`；Skills 在新路径
`4-extensions/4.3-skills.md` 扩展。

## 3. 内容写作原则

- 用户文档从“用户要做什么”开头，再解释入口、步骤、结果和失败处理。
- 不堆功能名，不用空泛形容词，不把源码符号直接扔给普通用户。
- 必须保留技术限制时，用一句人话说明影响；源码符号放到维护说明或审计证据里。
- 三种语言共享事实和结构，但允许符合各语言习惯的句式。
- 新增能力逐项对照当前源码符号和测试；找不到证据就不写。

## 4. 截图与生成图分层

| 层级 | 用途 | 允许来源 | 可证明什么 |
|---|---|---|---|
| `runtime_screenshot` | README、手册操作说明 | 目标基线真实运行时 | 当前界面和可见功能 |
| `concept_candidate` | Hero 背景、宣发方向 | ChatGPT 生图 | 视觉隐喻和品牌气质 |
| 确定性合成 | Logo、标题、箭头、标签 | 原始品牌资产、SVG/HTML | 准确文字与品牌身份 |

README 若暂时无法重拍，先移除旧图，不用假 UI 填空。v2 概念图继续保持空白徽章位，正式
发布前再合成原始 `assets/fyagent.png`。

## 5. 合同与验证

- DE 删除必须原子覆盖文件、导航、合同脚本和测试清单。
- 手册结构验收按实际文件计数，不凭计划手工估算。
- 本地链接解析覆盖三份 README、四个索引、84 篇正文、开发文档和新增营销文档。
- 图片审计以 40 个真实文件为主表，引用位置作为一对多关系记录。
- 纯移动阶段用 `git diff --summary -M` 检查 rename；正文修改完成后再跑完整检查。
- 最终质量入口固定为仓库 `mise` 任务，不直接调用 Trellis Python 或拼装替代命令。

## 6. 兼容与回退

- 不改运行时代码、数据库、配置格式或发布产物，产品兼容风险低。
- 每个 Wave 单独提交；DE 闭包、历史归档、README、手册移动、手册内容、截图规划、营销
  资产可以按提交回退。
- 如果目标基线在实施期间移动，不自动追随远端；先完成当前 SHA，再单独评估 rebase。

## 7. 取舍

- 不拆父子任务：这些改动共享大量链接和身份合同，单分支顺序执行更容易保持一致。
- 不维护四语 README：德语入口缺乏对应手册和维护闭环，保留只会继续漂移。
- 不把开发文档本地化：工程合同变化快，复制三套带来的漂移风险高于语言收益。
- 不批量生图：先验证资产系统和一张候选图，避免在产品证据不合格时扩大视觉债务。
