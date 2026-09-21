> 最终状态：实现与本机门禁已完成；当前证据与能力边界见 implementation/delivery-readiness.md。下列分工表保留实施期间责任记录，具体旧状态已 superseded。

# 实施顺序与责任边界

用户已授权实施至功能完成和 PR 交付。生产工作树基于组织仓库 `fy-agent/fyagent` 的 `origin/main c0b2ec21`，分支 `codex/session-cross-device-recovery-20260922`；不混入原目录已有改动或独立 PR #196。

| 工作包 | 实际执行配置 | 当前验收边界 |
|---|---|---|
| 后端迁移、SQLite 回执、编排 | Cursor / Claude Opus 5 300K High | 实现中；必须关闭集成复核发现后再验收 |
| UI/UX 与前端 | Antigravity / gemini-3.8-flash-high / high | 定向修复导入重试、批量预览、能力与文件选择 |
| 测试实现 | Cursor / GPT-5.6 Sol 272K High | 真实 schema/port/组件/浏览器与 Rust 行为测试，不把计划用例记为通过 |
| Codex 独立评审 | Cursor / Grok 4.7 256K High | 首轮无 P0/P1，P2 由协调者裁定和修复；整合后复核 |
| 总产品/架构与关键实现 | GPT-6 协调者 | Codex 原生新会话投影、版本与无覆盖边界、身份/跨包问题、最终 PR |
| 首屏构建与导航集成 | 原生执行子任务 | 同预算构建通过；最终 browser boot 仍须整合验证 |
| 原生接续验证 | 原生执行子任务 | OpenCode/Hermes 本机隔离证据；Gemini exact-version 验证中 |
| 定向资料 | 原生 Luna / medium | 已有研究与官方来源补充，不重复全机盘点 |

## 集成完成条件

1. 严格正文与 schema 校验、来源/快照身份、真实缺答复和不确定最终答复分离。
2. 前端完整预览、真实文件选择、同请求重试不重复、来源对应目标、准确失败与恢复状态。
3. 现有数据库单张回执表，写前登记、无覆盖、未知副作用禁止重写、目标与设备映射核验。
4. provider 逐一版本能力判断，已有可行路径做完实现和原生验证；未验证或原生格式限制给出具体原因与证据。
5. 最后相关修改后的 canonical type/lint/format/unit/browser/build、Rust fmt/check/clippy/test 与必要架构检查。
6. 精简并回读规范/交付证据；按任务路径提交，创建指向 main 的 PR，检查 CI/冲突并附加 PR 到当前任务。

## 已替代的旧判断

- `src/v2` 计划为旧基线；当前实现使用单 renderer `src/pages` / `src/shared`。
- Codex inject-only 无可见历史被新 legacy event+response 原生投影证据替代；不是放宽未知 final 判定。
- OpenCode 任意历史占位模型不会影响接续的判断被实跑反例替代，需目标本机 session.model。
- Hermes 没有官方非交互读回的判断被 `sessions export` 实跑替代；现成 foreign import 有损，必须修正。
- 首屏 670259 bytes 超限已通过合理依赖拆分修复，预算不变。

## Windows 与回退

用户已明确允许 Windows 入口暂不可用时略过实机验收。保留跨平台实现与 CI，真实 Windows 验收单独标为未验证；不绕过现有正式 Windows 普通用户执行边界。
版本或原文契约不符时阻止相关导出/写入，副作用不确定时保留回执供对账，不删除旧原生会话或 Memory 文件。用户授权 PR，不包含合并或发布。
