# Navigation integration check

已修正两个测试文件的旧期望，未修改产品、模型路由或平台契约，未提交。

- `tests/renderer/app/architecture.test.ts`：九个主入口仍逐一要求字面量动态导入；Projects 对应应用组合层 `./ProjectsWorkspace`。静态导入禁令改用已有 TypeScript AST 检查，避免漏过多行静态导入。
- `tests/renderer/widgets/app-shell/SideNavigation.test.tsx`：明确断言九个路由叶子、六个顶层控件及 Projects 链接。Home 聚焦客户项目，ArrowDown 到 AI 软件配置；原有折叠、隐藏叶子、Tab、End、aria-current、唯一选中层等断言保留。

验证：格式化完成后，于 2026-09-19 17:51 运行 `rtk proxy mise run test:unit tests/renderer/app/architecture.test.ts tests/renderer/widgets/app-shell/SideNavigation.test.tsx`，2 文件 / 30 测试全部通过，exit 0。定向 `git diff --check` 通过。没有执行全库测试。

## Vitest 4 / matchMedia 局部 fixture 修复

jsdom 没有原生 `window.matchMedia`；Vitest 4 的 spyOn 会拒绝对 undefined 建立函数 spy。仅在 `tests/renderer/features/useAppearance.test.tsx`、`tests/renderer/shared/motionPreferences.test.tsx` 内改用显式返回 `MediaQueryList` 的局部 `vi.stubGlobal`，并在 afterEach 恢复 globals。没有添加影响所有 renderer 测试的默认媒体环境，也没有改 setup 或产品。

原有浅色/深色动态切换、用户明确选择、storage 不重复写入、reduced-motion 动态变化、卸载移除 listener 数量和无 matchMedia fallback 断言全部保留。

验证：2026-09-19 17:53 运行 `rtk proxy mise run test:unit tests/renderer/features/useAppearance.test.tsx tests/renderer/shared/motionPreferences.test.tsx`，2 文件 / 5 测试通过，exit 0。两文件 prettier 完成，定向 git diff --check 通过。未提交。
