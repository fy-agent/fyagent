# 一手来源与依赖选择

核查日期2026-09-06。仓库实际安装React18.3.1、framer-motion12.23.25、
Radix Tabs1.1.13、Vite7.3.6、Vitest3.2.7、Playwright1.62.1、
dependency-cruiser18.2.0。当前官方文档若展示更高版本API，不代表本仓库能直接用。

## 布局与绘制

- https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/display
- https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Positioned_layout/Stacking_context
- https://www.radix-ui.com/primitives/docs/components/tabs
  display:contents不是可承载视觉层级的真实盒；Radix保留受控状态、键盘和ARIA。
  F1已有两浏览器因果实验，优先修共享绘制边界，不为遮挡更换tabs库。
- https://github.com/bvaughn/react-resizable-panels
- https://github.com/bvaughn/react-resizable-panels/releases
- https://react-resizable-panels.vercel.app/common-questions
  选用4.12.3作为SplitPanes适配候选，npm元数据确认MIT、React18/19 peer，
  发布说明含约束比较、CSSStyleSheet兼容和pointerup孤儿组修复。其单位规则
  与旧版不同，适配显式使用px/%而非猜数字含义。unpackedSize不是最终bundle
  开销；实现阶段须跑license/advisory/lock、体积与WebKit测试，未通过不宣称采用。

## 单一工程入口

- https://v3.vitest.dev/guide/projects
  对应已安装3.2.7；单一配置下用projects区分环境，不需要保留旧/新两套测试
  配置。inline项目注意extends:true，根配置不会自动成为测试项目。
- https://vite.dev/guide/build
  应用HTML与常规构建/preview服务器是正式入口，独立离线生成器不是必需组成。
- https://github.com/sverweij/dependency-cruiser/blob/main/doc/cli.md
  使用已有可达图工具。type-only边与runtime边分开，不把no-orphans等价为
  所有入口可达性，也不把旧UI安全测试自动归类为可删。
- https://git-scm.com/docs/git-diff
  Git diff的rename识别依相似性；代码移动与内容重写分提交便于复核，不重写历史。

## 动效和主题

- https://developer.chrome.com/docs/web-platform/view-transitions/same-document
- https://developer.mozilla.org/en-US/docs/Web/API/Document/startViewTransition
- https://developer.mozilla.org/en-US/docs/Web/API/ViewTransition/skipTransition
- https://developer.mozilla.org/en-US/docs/Web/API/Animation/cancel
  原生视图过渡/WAAPI承担捕获与插值，应用只编排来源和提交/清理。失败或
  API缺失不能阻止主题生效；旧异步结果不能清除新请求。
- https://motion.dev/docs/performance
- https://motion.dev/docs/react-layout-animations
- https://motion.dev/docs/press
  复用现有手势、插值和layout能力；尺寸变化需限定子树并量测，不能宣称所有
  layout/clip-path都是免费合成。当前Motion文档版本与本地12.x需逐API对照。
- https://github.com/pacocoursey/next-themes
  比较过现成主题提供者，但仓库已有本地偏好/native主题合同，当前更小的
  选择是复用并迁移该owner，不加第二个provider或引入Next相关框架。
- https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html
  正常正文/辅助文字4.5:1；原生禁用和品牌装饰单列。另加遮挡/字形实际绘制
  证据，不能用声明颜色值覆盖用户截图指出的渲染问题。

## 决策结果

继续现有Radix/Motion/TanStack/WAAPI；单一SplitPanes适配成熟分栏库是唯一
拟新增运行时依赖。不新建动画引擎、玻璃引擎、通用DOM克隆器、主题设置框架
或独立用户测试网站。以上是方案选择，不是尚未执行的依赖审计/性能结果。
