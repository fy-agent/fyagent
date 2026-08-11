# 文档重构与视觉资产规划实施清单

## Phase A：预检与规范

- [x] 确认当前分支包含 `b6f60dfe`，工作树只保留已知用户未跟踪目录。
- [x] 运行 `mise run bootstrap`、Trellis context，并读取 shared/backend/frontend 规范索引及
  与文档合同、任务入口、截图验证有关的指南。
- [x] 运行目标基线计数和链接检查，结果与审计报告一致。

## Phase B：活动入口闭包

- [x] 删除 `README_DE.md`，清理三份 README 的 Deutsch 导航。
- [x] 更新 `docs-contract-check.mjs` 与两个边界测试中的活动文档列表。
- [x] 运行：

```powershell
mise run test:unit -- tests/localBuildBoundary.test.ts tests/desktopSecurityBoundary.test.ts
mise run check:contracts
```

- [x] 纯移动 `session-manager.md` 到 `docs/fyagent/history/session-manager-prd.md`，确认 rename，
  再补历史标记。
- [x] 新建 `docs/release-notes/README.md`。

## Phase C：README 瘦身

- [x] 为三份 README 建立“保留 / 删除并链接 / 必须迁补”段落清单。
- [x] 核对所有被删工程信息在 `docs/fyagent/development/` 或 Trellis 规范中的唯一落点。
- [x] 重写三份 README 的开场、功能、快速开始、文档和贡献入口；保留法律与上游来源。
- [x] 移除旧品牌截图引用；如果真实 FyAgent 截图已验收，再引用新图。
- [x] 复核中文、英文、日文事实一致，中文无模板化 AI 文案。

## Phase D：三语手册结构

- [x] 先用 Rename Map 纯移动中、英、日各 20 篇正文，并记录 `git diff --summary -M`。
- [x] 新建三语 `2.1-install.md`、`2.2-update-diagnose.md`、`4.6-workbuddy.md`。
- [x] 扩展三语 `4.3-skills.md`。
- [x] 一次性更新标题、章节号、相对链接、跨文档引用和三语索引。
- [x] 删除索引中的滚动“当前亮点”，改成按用户任务找入口的短导航。
- [x] 验证每种语言 28 篇正文，旧目录和旧路径引用为 0。

## Phase E：截图与营销资产

- [x] 建立 40 张 PNG / 84 处引用审计表。
- [x] 新建 shot-card 索引和 15 张 shot card。
- [x] 预检真实 FyAgent 运行环境；只有获得固定数据、脱敏且稳定的三语界面时，才按中/英/日各 2 张拍摄 README proof
  frame，并标记 `runtime_screenshot`。
- [x] 未取得合格运行时证据时，记录环境边界，保持 README 无旧品牌截图，不伪造替代图。
- [x] 新建 `docs/fyagent/marketing/visual-asset-plan.md`。
- [x] 新建 `docs/fyagent/marketing/prompts/README.md`，收录四类结构化提示词卡。
- [x] 复核 VibeKey 边界、v1/v2 状态、Logo/文字确定性合成门禁和响应式安全区。

## Phase F：审计、质量与收口

- [x] 更新 `docs-restructure-audit-v0.3.0.md`：最终数量、白名单、截图表、命令、退出码、
  证据等级和未完成阻塞。
- [x] 更新 `.omo` 计划 C0–C13 和状态，清理被取消动作的陈旧引用。
- [x] 运行活动身份、DE 引用、旧目录、文件计数、本地链接、图片引用和 Markdown 结构检查。
- [x] 运行：

```powershell
git diff --summary -M
git diff --check
mise run check:contracts
mise run check
```

- [x] 使用 `trellis-check` 做独立质量复核；失败则修复并重新运行。
- [x] 通过 `trellis-update-spec` 判断是否需要更新规范；`.mjs` LF/shebang 约束已写入任务运行器合同。
- [x] 只提交本任务文件，确认 `.omo/run-continuation/` 未暂存。

## 风险文件与回退点

| 范围 | 风险 | 回退点 |
|---|---|---|
| DE 闭包 | 合同脚本或测试继续读取已删除文件 | 单独提交，目标测试通过后再继续 |
| 60 个移动 | 路径替换级联、Git 不识别 rename | 纯移动与正文修改分提交 |
| README 瘦身 | 有效工程信息丢失 | 保存责任映射，逐段复核后删除 |
| 真实截图 | 本机 UI、数据或语言状态不稳定 | 不发布假证据，README 暂时无截图 |
| 批量文档修改 | 三语链接漂移 | 每个 Phase 后运行本地链接解析 |

## 启动前检查

- [x] PRD、设计、实施清单经过最终复读，无未决产品问题。
- [x] 用户在看到最新规划摘要后的下一条消息中明确批准实施。
- [x] `mise run trellis:validate -- .trellis/tasks/08-10-docs-restructure-current` 通过。
