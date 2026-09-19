# Navigation integration check

已修正两个测试文件的旧期望，未修改产品、模型路由或平台契约，未提交。

- `tests/renderer/app/architecture.test.ts`：九个主入口仍逐一要求字面量动态导入；Projects 对应应用组合层 `./ProjectsWorkspace`。静态导入禁令改用已有 TypeScript AST 检查，避免漏过多行静态导入。
- `tests/renderer/widgets/app-shell/SideNavigation.test.tsx`：明确断言九个路由叶子、六个顶层控件及 Projects 链接。Home 聚焦客户项目，ArrowDown 到 AI 软件配置；原有折叠、隐藏叶子、Tab、End、aria-current、唯一选中层等断言保留。

验证：格式化完成后，于 2026-09-19 17:51 运行 `rtk proxy mise run test:unit tests/renderer/app/architecture.test.ts tests/renderer/widgets/app-shell/SideNavigation.test.tsx`，2 文件 / 30 测试全部通过，exit 0。定向 `git diff --check` 通过。没有执行全库测试。

## Vitest 4 / matchMedia 局部 fixture 修复

jsdom 没有原生 `window.matchMedia`；Vitest 4 的 spyOn 会拒绝对 undefined 建立函数 spy。仅在 `tests/renderer/features/useAppearance.test.tsx`、`tests/renderer/shared/motionPreferences.test.tsx` 内改用显式返回 `MediaQueryList` 的局部 `vi.stubGlobal`，并在 afterEach 恢复 globals。没有添加影响所有 renderer 测试的默认媒体环境，也没有改 setup 或产品。

原有浅色/深色动态切换、用户明确选择、storage 不重复写入、reduced-motion 动态变化、卸载移除 listener 数量和无 matchMedia fallback 断言全部保留。

验证：2026-09-19 17:53 运行 `rtk proxy mise run test:unit tests/renderer/features/useAppearance.test.tsx tests/renderer/shared/motionPreferences.test.tsx`，2 文件 / 5 测试通过，exit 0。两文件 prettier 完成，定向 git diff --check 通过。未提交。

## 九路由生产守卫与首屏预算

原生产 manifest 已包含 ProjectsWorkspace 独立动态入口，但守卫仍硬编码八页。补齐第九页后，守卫正确暴露 initial eager graph 为 688646 JS bytes，超过现有 665600 上限 23046 bytes。原因是 native FeaturePorts 在启动时静态加载 Projects、DeliveryKits、Verification 等未使用功能及其校验模块；browser facade 仅有类型引用和拒绝操作的 stub，不是这次超限来源。

最小修复保留全部功能与 Promise 接口：native facade 对 Projects、DeliveryKits、Verification、Models、ConfigRecovery 使用字面量动态导入和显式逐方法参数转发，原 adapter 校验、错误处理和 invoke 保持不变。已有 Health 延迟边界保留。逐一核查五个工厂均无需要跨调用共享的实例状态、侦听器或取消控制器；取消所需 projectId/previewId、恢复请求及覆盖令牌均由调用参数交给 native。没有引入缓存框架。

生产守卫现在明确覆盖九个独立页面入口和六个指定 deferred ports；新增 Projects 缺失、独立入口冲突、每个延迟模块进入启动闭包等拒绝用例。原 JS/CSS 总预算、每 chunk 预算、共享 vendor 闭包统计、未知动态入口拒绝规则均未放宽。三处 frontend spec 同步为九路由，并明确 ProjectsWorkspace 组合入口与延迟模块边界。

本阶段文件：
- `scripts/verify-route-chunks.mjs`
- `src/shared/platform/tauri/features.ts`
- `tests/renderer/scripts/verify-route-chunks.test.ts`
- `tests/renderer/platform/deferredProjectPorts.test.ts`（新增）
- `.trellis/spec/frontend/navigation.md`
- `.trellis/spec/frontend/security-boundaries.md`
- `.trellis/spec/frontend/quality-guidelines.md`

最终验证（2026-09-19 18:04，全部 exit 0）：
- `rtk proxy mise run build:renderer`：9 routes，664440 initial JS bytes，46664 initial CSS bytes。较初始减少 24206 JS bytes，距原上限仍有 1160 bytes。前序超限结果由此最终构建结果取代。
- `rtk proxy mise run test:unit tests/renderer/platform/deferredProjectPorts.test.ts tests/renderer/platform/projects.test.ts tests/renderer/platform/deliveryKits.test.ts tests/renderer/platform/verificationPort.test.ts tests/renderer/platform/featurePorts.test.ts tests/renderer/scripts/verify-route-chunks.test.ts`：6 文件 / 40 测试通过。覆盖首次调用前不加载、首次调用加载、后续模块缓存、参数/布尔结果/错误原样转发及原 adapter 合同。
- `rtk proxy pnpm typecheck` 通过。
- `rtk proxy pnpm exec eslint src/shared/platform/tauri/features.ts tests/renderer/platform/deferredProjectPorts.test.ts tests/renderer/scripts/verify-route-chunks.test.ts` 通过。
- 上述变更的定向 `git diff --check` 通过。

没有修改 ProjectsWorkspace、VerificationPanel、native Rust 或构建预算，没有执行 native build、实机操作或全库测试，未提交。该证据属于 renderer 生产构建及定向静态/单元检查；整体 native 集成由主控接续。
