# 一手研究与初查

2026-09-06，基线4f8973ef；当前依赖Vite7/Vitest3，按匹配主版本文档复核。

- https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/overflow
  overflow需要约束空间；hidden不能替代用户可滚动auto。不能用scrollIntoView证明wheel可达。
- https://www.radix-ui.com/primitives/docs/components/dialog
  复用现有modal/focus/portal，来源几何只属于呈现层。
- https://www.radix-ui.com/primitives/docs/components/dropdown-menu
  瞬态内容有自己的焦点恢复与卸载生命周期，不能假定菜单项会在Dialog挂载后存活。
- https://v7.vite.dev/config/ 与 https://v7.vite.dev/config/shared-options#css-postcss
  --config相对cwd；PostCSS可内联且会关闭其他配置搜索，适合唯一autoprefixer配置。
- https://v3.vitest.dev/config/ 与 https://v3.vitest.dev/guide/projects
  支持显式配置入口；root/global与测试环境projects分离，移动不重建第二套测试入口。
- https://playwright.dev/docs/api/class-testconfig 与 https://playwright.dev/docs/test-webserver
  testDir及webServer cwd必须在配置迁移时显式固定到项目，性能仍串行。
- https://eslint.org/docs/latest/use/configure/configuration-files
  根eslint.config.mjs为编辑器/CLI发现入口，保留有实际工程收益。
- https://www.typescriptlang.org/docs/handbook/tsconfig-json.html
  tsconfig标识项目根且tsc无参数向上发现，保留根tsconfig而非制造空代理。

初查：Skills存在FeatureTabPanel中间盒，page:has(workspace)规则却将非直接workspace
设为flex:0 0 auto；发现页还反向覆盖overflow，须通过长数据复现确认。账号移除
按钮/对话框已传originRef，因此不是简单缺属性；快速preview更新会改变Dialog尺寸，
现有useDialogResize在!settled时调用originSettler，须测实际动画是否被提前终止。
另有WorkBuddyTrustDialog未提供来源，菜单/嵌套来源须逐项追踪。以上是待验证假设。

根目录当前没有未跟踪MJS/TS临时脚本；现有脚本均为工具配置。.DS_Store是Finder
元数据，不属于代码或配置。保留与移动的详细清单在治理子任务记录。
