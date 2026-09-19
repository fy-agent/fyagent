# 交付包分享最小修复

2026-09-19，主控工作树中的未提交修改；独立阶段审查交由另一 Agent。原审查中的「导入后无法再交接」已按最小范围补齐。

## 行为

- 已严格解析并导入的不可变版本可以分享。native `get` 只解析确切内置或库中 id/version/digest；分享预览与最终 `export_bytes` 都调用它，缺失或变更不能沿用旧预览。只返回预览中的 canonical manifest，不拼入项目状态或本机凭据引用。
- 原有内容预览、来源标记、显式确认、保存取消、create-only 文件导出边界保留。第三方内容不会被标为内置合成内容；界面要求确认无客户资料或凭据，而不宣称任意导入文本已自动脱敏。
- `KitLibrary::confirm_identity(&KitIdentity) -> Result<()>` 复用 exact `get` 与 `compatible`，不写磁盘，供 root 的项目绑定 adapter 调用。
- `run` 和 validator 未改。自定义包即使再分享、再导入，仍不能产生 `local_fixture` 机器结果。
- 针对性失败用例发现保存错误原本显示在 modal 背后；现将预览期间的错误显示在对话框内，可直接重试。

## 修改路径

1. `src-tauri/src/services/delivery_kits/mod.rs`
2. `src-tauri/src/services/delivery_kits/tests.rs`
3. `src/domain/delivery-kits/index.ts`
4. `src/shared/features/delivery-kits-ui/ProjectDeliveryKitsPanel.tsx`
5. `tests/renderer/features/deliveryKits.test.ts`
6. `tests/renderer/pages/delivery-kits/Panel.test.tsx`
7. `.trellis/spec/backend/delivery-kits.md`
8. `.trellis/spec/frontend/delivery-kits.md`
9. 本报告

没有修改 commands、port、validator、项目/evidence 接线或 fingerprints，没有提交。

## 验证与边界

- `mise run test:unit -- tests/renderer/features/deliveryKits.test.ts tests/renderer/pages/delivery-kits/Panel.test.tsx`：最终 2 文件、8 项通过（Vitest 4.1.11）。覆盖严格 parser、已导入分享预览/确认、取消保存、失败错误脱敏及弹窗内可见、重试成功、自定义包运行禁用。首轮新测试误写按钮名，修正测试；第二轮实际发现 modal 外错误不可访问，修复产品后通过，没有放宽断言。
- 四个变更 TS/TSX 文件定向 ESLint、Prettier check 通过；两份 Rust 文件定向 rustfmt check 通过；`git diff --check` 通过。
- 新增/扩展 Rust 用例覆盖 custom 包导入后跨库原样交接、未导入拒绝分享、预览取消与种类校验、删除源版本后拒绝旧分享、exact/compatible 无写确认、custom 保持 UnsupportedValidator。
- Rust 过滤曾启动，在共享编译锁后按 root 最新要求取消，exit 130，**没有 native 测试通过结论**。由 root 待 composition/evidence 冻结后统一运行 `mise run rust:test -- delivery_kits`。
- 未运行全库、桌面原生 picker、Windows 或真实客户验收。保守 secret-pattern 仍不是任意文本脱敏保证。

`shared/features/delivery-kits.ts` 的旧 export_not_allowed 文案不在本 Agent ownership 中，已通知 root 做一行对应业务文案更新；正常新分享路径没有保留旧禁止逻辑。
