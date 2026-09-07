# 材质与容器响应复核

## 基线与处理

旧画面实际合成抽样出现约1.77–3.66的正文/辅助文本对比，登录选择描述约4.12–4.28。
不是单纯字号问题。保留蓝灰气质但压低背景亮度，正文与辅助层级改为可读实色；
同类控件、列表项、卡片、大面板、对话框与圆形/胶囊分别映射少量语义token。
使用PostCSS测试收束整个V2 CSS，未用正则写第二个CSS解析器。

`GlassMaterial.tsx`由原`LiquidGlassLens.tsx`按真实职责更名，保留开发样品接口，
新增产品`FrostedSurface`。实际复用已装MIT 0.1.1库的material分支：不传refract、
filterResolution或视频等选择另一分支的参数；表单不作为反射副本，背景与文字分离。
没有新增运行时依赖、截图引擎或第二玻璃实现。CSS为库不适用时的标准降级。

新增`@axe-core/playwright`4.13.0，MPL-2.0，仅dev dependency，传递axe-core；
使用官方Playwright集成，不自行实现可访问性扫描器。透明/渐变的incomplete项由
固定几何截图的WCAG亮度算式补充抽样，不能作为完整应用认证。未给扫描加忽略名单。

Models/账号详情成为命名inline-size容器，基础grid本身能在窄空间落到单列。
保留窗口布局、目录流程与分栏拖拽机制，未为了掩盖文字逃逸全局加overflow:hidden。
同一1232宽窗口下将detail限制到320px、加入长中文和无空格URL，表单与按钮仍在容器内；
761/759/616窗口压力也通过。616仅为横向200%缩放等价压力，不冒充系统字体缩放实测。

## 验证记录

- V2 type/lint、77文件535项单测通过。
- 生产启动回归、196项四尺寸浏览器回归通过（包含七页合成文字、材质与320px容器）。
- 增补关键输入边界3:1、侧栏及hover文字抽样后，196项浏览器回归再次通过；完整
  `check:prearchive --exclude-active-task .trellis/tasks/09-05-frontend-surfaces-responsive`
  退出0。未改检查预算、未屏蔽告警；日志`/tmp/fyagent-round4-surface-{browser-final,prearchive}.log`。
- pnpm audit结果0（`/tmp/fyagent-round4-audit.json`）；旧维护性依赖提示未屏蔽。
- 当前模态截图：`node_modules/.cache/fyagent-ux-round4/`；真实运行夹具，无用户凭据。

## 依据与边界

官方资料：https://github.com/samasante/liquid-glass 、https://playwright.dev/docs/accessibility-testing 、
https://github.com/dequelabs/axe-core-npm/tree/develop/packages/playwright 、
https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum 、
https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_containment/Container_queries 。

可见抽样≥4.5不代表所有不可见文本、禁用控件、品牌图、错误数据与平台均认证。
减少透明度/forced-colors有独立回归；最低版本原生WebView与真实GPU合成留明确验收边界。
