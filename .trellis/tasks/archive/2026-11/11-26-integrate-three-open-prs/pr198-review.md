# #198 剩余价值与关闭建议

**建议不要整支合，也不要现在直接关。** 把确认缺失的修复作为小提交移植到当前实现，附回归和原 PR 作者来源；替代提交进入 `main` 并核对后，由维护者授权关闭旧 PR。时间久只说明需要重审，不是关闭依据。

比较输入：[PR #198](https://github.com/fy-agent/fyagent/pull/198) `367c62ec4d5a746aed4c739459fc3f38ecc80f5c` 与 [主线 `82d62049`](https://github.com/fy-agent/fyagent/commit/82d6204908ea28d2dfb8a8af61cc7b67b65ea161)。查询时 PR 为 OPEN/DIRTY，最后更新 2026-09-22。`git rev-list --left-right --count main...PR` 对固定 SHA 返回 `847 4`；`git cherry -v` 的四个提交中只有 Claude 提交标 `-`（补丁等价）；其余 `+` 只表示没有相同补丁，不能据此判断行为没进主线。以下为静态判断，未运行产品测试。

| ID / 行为 | 主线证据与建议 |
| --- | --- |
| P198-A · Claude 同 prefix npm 归属 | [`75a13f42`](https://github.com/fy-agent/fyagent/commit/75a13f4281af5424438a94c9cd7b2f1be88adf06) 明确只复用 #198 的 Claude 修复；`src-tauri/user-helper/src/claude.rs` 与 #198 head 无内容差异。实现、正反例和 spec 已有。不要重移植；按 T6/H7 验证不回归。 |
| P198-B · Gemini 的提取与恢复门独立 | 主线 `src/shared/features/session-migration.ts:724–756` 已按 `installed` 与 `writeSupported` 决定恢复，不因 `extractionSupported=false` 否决已验证的 writer。回归验证写允许/不允许两边，不重新替换生产门。 |
| P198-C · 中文错误反馈 | 主线 `src/pages/sessions/components/FailureFeedback.tsx` 已处理 alert、程序化焦点和滚动；`ImportPackageDialog.tsx` 中解析/恢复使用它。保留现有分阶段错误与写入状态提示；若有具体失败分支未覆盖，补用例，不用旧错误布局替换。 |
| P198-D · Codex 首启前禁用与告知 | 主线 `migrate/capability.rs:139–160` 的版本 probe 未读 native ID；`migrate/native/codex.rs:41` 只在 native store identity 路径读，已有写入拒绝，不代表 UI 提前禁用。`Page.tsx` 挂载探测与按钮/键盘打开导入分离。拟在受支持的 writer 上添加有界只读原生 ID 预探测和准确提示，并让两个打开路径使用同一重新探测入口；等待结果时不复用过期放行。移植不得绕开现行写入门；仅客户端自身生成生产身份。 |
| P198-E · Codex 标题 fallback | 主线 `src-tauri/src/session_manager/providers/codex.rs:1951–1961` 的 JSONL fallback 对 `role=user` 取文本但未检查结构化来源种类。拟用合成上下文 fixture 验证；只在 JSONL fallback 忽略非纯用户内容，兼容无来源字段的旧格式，保留显式 thread 名优先级。核对快速扫描、缓存和详细读取是否共用规则，不改变会话包内容协议。 |
| P198-F · 老结构摘要 | `5c4ac904` 是给旧预探测源码配的结构快照；不直接移植。若 D/E 触发当前结构门，用最终树审核、更新现行清单。 |

**重新探测的两条防回归约束：** `require_same_store` 也会为恢复过程中发生的身份变化返回 `targetStoreUnidentified`（`restore.rs:191–209`），首次启动提示仅用于 Codex 的就绪探测，不能全局覆盖该错误。`Page.tsx:906–929` 条件挂载对话框，现有 reopen 用例并不是完整页面：加上真实页面关闭/重开期间若已有结果未确认，同一请求仍冻结、只能核对/重试原请求的用例。源码显示有清空状态的风险，但本轮未运行复现；若证实，记录为设计裁决，禁止以重新探测绕开未决写入。

## 关闭门槛

1. 在批准后的执行阶段，用最终集成树验证 D/E 的原始失败与定向修复；若更新后的主线已有等价行为，则用确切源码/用例证明，不再重复移植。A/B/C 给出防回归证据。
2. 替代 PR 备注 `Source: fy-agent/fyagent#198`、原 SHA、行为取舍表及作者致谢；保留可追溯的提交/测试和 macOS 验收 Issue。主线发生漂移时复核版本和数据边界，不能静默带回过期实现。
3. 原 PR 保持开放，直到有替代去向、门禁通过并核对实际合入 `main` 的 SHA；获远端操作授权后，评论 A/B/C 已在主线、D/E 已由哪个 PR 接续、链接验收，再关闭为「被替代」，不删远端分支。不允许只因超期关闭，也不在 planning 阶段发布评论。
