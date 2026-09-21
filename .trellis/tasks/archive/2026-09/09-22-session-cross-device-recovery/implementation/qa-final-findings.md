# QA final findings

## QA-FINAL-UI-001：900×600 下新增 Sessions 后主导航底部越界

- 复现：
  `rtk mise run test:browser -- tests/browser/session-migration.spec.ts tests/browser/shell.spec.ts`
- 定向结果：31 passed、5 failed。修正迁移按钮定位后，确定的生产失败为
  `tests/browser/shell.spec.ts` 的 `keeps the complete shell visible, separate, and overflow-free`。
- 精确证据：900×600 视口中某个主导航控件 `box.y + box.height = 605`，合同上限为 `viewport.height + 1 = 601`。
- 1152×640、1232×700、1440×900 的同一 shell 用例通过；导航哈希、选中态、键盘顺序、折叠展开和路由保活均通过。
- QA 已把 `/sessions` 作为明确路由插入 `navigationContract` 和 `primaryControlTestIds`，并验证真实文档顺序；没有只把 6/12 机械改成 7/13，也没有放宽可见性边界。
- 影响：最小支持视口底部主导航存在 4px 溢出，键盘仍可达但控件不完全可见。
- 边界：生产布局不在 QA writer 范围，留给 root/前端 owner 修复；在修复前不运行完整 browser。

## 已关闭：QA-IMP-008 capability 门控

- 同轮定向测试已通过“`writeSupported=false` 时恢复按钮 disabled”断言，说明上一轮确认的门控旁路已修复。
- 后续失败发生在手动标记按钮的旧定位器；QA 已改用当前真实可访问名称“标记：我已手动续聊”，不改变能力断言。
- 定位器修正后单独复测：production boot 3/3、迁移浏览器测试 12/12 通过，覆盖四个 Chromium 视口。

## Rust 状态

- `backend-rework-exit.json` 尚未出现；本轮未反复编译根级后端，也未把 path-include harness 冒充 test-hooks 验收。
- Windows 实机按用户授权略过，继续标记未验证，不进行远程重试。
