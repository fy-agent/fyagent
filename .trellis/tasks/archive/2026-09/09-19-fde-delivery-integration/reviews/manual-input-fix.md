# 人工登记中文句读与局部校验修复

结论：安全中文句读可保存；无效输入在表单内指出字段并可直接纠正，不再因尚未发出的无效请求让整个验证面板进入待复核。项目资料未保存或归档时，已有记录仍可查看、刷新、预览和导出，新的检查与登记暂停。本切片未提交。

## 修改

- `src/domain/verification/index.ts` 与 `src-tauri/src/services/verification/domain.rs`：同一自然文本规则新增 `：；。·！？“”‘’`。人工登记、人名/角色/作用范围、交接事项和责任人共用该规则；长度、首尾空格、URL/路径和 secret 标记限制保留，外部依据的 HTTPS 特例不变。
- `src/shared/features/verification/VerificationPanel.tsx`：人工登记先通过现有 `manualSchema.safeParse`；日期非法时也进入局部校验。交接事项同样使用 `handoffSchema.safeParse`。局部反馈显示字段名，不回显敏感输入，保留草稿，修正后直接提交。真实 native 操作失败的全局待复核行为保留。
- 同一面板新增可选 `mutationBlockedReason`。有原因时禁用新检查、人工登记、撤销与交接事项修改，动作入口也有守卫；显示原因且不卸载表单。刷新、预览、导出已保存记录仍可用，导出的 native 再核对规则保留。默认独立面板行为不变；组合层传参由 root 完成。
- `tests/renderer/features/VerificationPanel.test.tsx`、`tests/renderer/features/verification.test.ts`、`src-tauri/src/services/verification/tests.rs`：增加中文句读保存/读取/导出、路径和秘密拒绝、局部纠错、保留草稿、阻止修改但可读/导出的回归证据。

## 验证

执行目录：`~/.codex/worktrees/fyagent-fde-control/fyagent`。

- 修复前，新增两个表单局部校验用例均失败，证明旧表单会把不合法字段直接送往 port，无法产生局部错误反馈。
- `rtk proxy mise run test:unit -- tests/renderer/features/VerificationPanel.test.tsx tests/renderer/features/verification.test.ts tests/renderer/platform/verificationPort.test.ts`：最终 **3 文件 16/16 通过**。
- `rtk proxy mise run rust:test -- verification_manual`：**3/3 通过**，包括真实 service 保存、回读与 Markdown/JSON 预览，以及既有人工验收不能漂白 fixture 的回归。日志：`~/fyagent/tmp/fde-workstreams-20260919/control-evidence/manual-input-native-fix.log`。
- 四个改动 TS/TSX 文件的 ESLint 通过；范围内格式化、Rustfmt 与 `git diff --check` 通过。TypeScript 检查通过。

中间一次 renderer 运行 15/16，失败是测试文本查询同时命中证据“通过”和下拉选项“通过”；已把断言限定到配置结果区域，随后完整重跑上述三个定向文件通过。未修改产品逻辑掩盖此测试问题。

没有启动全库测试或修改 Page、Kit、组合文件。共享 native 构建已退出并通知 root；最终统一 Clippy、构建及实机复验由 root 接续。以上是定向 renderer/native 证据，不替代最终原生界面验收。

## 三标签页隐藏后的人工草稿补修

独立复核发现 `manualOpen && visible && data` 会在标签页隐藏时卸载真实 ManualForm，导致登记人、角色、范围及依据草稿丢失。最小修改仅去掉挂载条件中的 `visible`，由父层 `PersistentSurface` 的 hidden/inert 隐藏，现有 `disabled={!ready}` 继续阻止隐藏期间操作。

新增真实 `VerificationPanel + PersistentSurface` 回归：填写五个登记字段，隐藏后确认同一 form DOM 仍在但不可见且输入禁用；即使强制 submit 也不调用 record；恢复可见后五个字段均保留，没有自动提交。没有用替身表单。

此次增量只跑 `rtk proxy mise run test:unit -- tests/renderer/features/VerificationPanel.test.tsx`，**1 文件 10/10 通过**；两文件格式化与 diff 检查通过。未操作原生应用、未再次运行 native 或全库。产品源码再次冻结，交 root 重建。
