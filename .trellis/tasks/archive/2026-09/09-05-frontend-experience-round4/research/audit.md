# 静态基线与观察边界

基线：`d2d3bc47`。读取的是当前checkout；以下不是改后结果。2026-09-05开始时没有活动任务或工作区改动。

## 已确认

1. `src/v2/shared/ui/motion.ts:8-13`：唯一fySpringTransition为stiffness=520、damping=42、mass=0.62，没有press/modal/navigation分角色参数。它的存在不能证明所有变化已有动画。
2. `src/v2/shared/ui/primitives.tsx`：Button/GlassButton/IconButton为原生button包装，Dialog没有触发来源/几何API或进入退出动效；同一个文件混合按钮、表单、反馈与模态焦点职责。按压动效与Dialog实现应分明确owner，不能继续堆逻辑。
3. `src/v2/app/styles/tokens.css`：dialog surface是rgba(48,73,97,0.98)。`controls.css` 的Dialog有24px backdrop blur，但overlay只做暗化，近乎不透明内容色阻隔了背景可见性。这是配置问题，不等于没有安装玻璃库。
4. `LiquidGlassLens.tsx` 复用已装 @samasante/liquid-glass，live=false、filterResolution=1。源码引用扫描仅发现UI Lab消费；不能声称它已经给产品模态框提供材质。
5. 19个V2 CSS文件中，border-radius值扫描含25种不同写法（包含token、inherit、50%和多角写法），大量为8/9/10/11/12/13/14/15/17/18/19px局部字面量。圆形icon与pill不应机械替换为一个半径；同类表面统一到按角色的短尺度。
6. `PersistentPrimaryOutlet.tsx` 为每个已访问路径重建wrapper/children，`PersistentSurface.tsx`保持隐藏挂载。`usePersistentSearchParams`与多处直接useLocation仍订阅路由；隐藏不意味着零React工作。必须profile后判断是否有显著成本，不能直接断言为卡顿唯一根因。
7. `primaryPages.tsx` 已有缓存loader和后台预取；`main.tsx` 已优先加载初始页。没有“完全不预加载”的问题，不添加第二套loader、拆错入口或取消第三轮首屏就绪契约。
8. `split/split.css`两栏/三栏组合固定leading宽度和vw尺寸，断点目前基于viewport；分栏后的实际内容宽度可能与断点判断不同。需用窄pane与长内容测量证实每个溢出，不能把整个split系统替换成未经测试的新库。
9. 最大TSX入口：models/Page.tsx 1747行、skills/Page.tsx 1285行、mcp/Page.tsx 1011行、memory/Page.tsx 1001行。长度只用于确定审查顺序，不是性能或命名错误的证明。Page.tsx作为路由入口名称本身合理。

## 视觉复核

已查看第三轮留在本机缓存的1232×700模型页和账号Dialog截图（`node_modules/.cache/fyagent-ux/chromium-1232x700-{models,dialog}.jpg`）。可见模型页的嵌套浅蓝表面叠加后辅助白字区分弱，模态框主要呈实色蓝灰块；这是设计观察，不是已完成WCAG测量。截图来自夹具，不是真实账号。第四轮实施需重新采样生产构建并保存before/after，不用JPEG抗锯齿像素当作精确文本前景色。

## 下一步需要的证据

- 首访/回访点击至页面绘制、主线程长任务、模块请求、隐藏组件提交和查询计数；同数据、同构建模式、同屏幕，至少30回访样本。
- 窄分栏、长中英文、不可断邮箱/URL、200%文字缩放和错误状态的布局边界。
- 透明层合成后的文本/控件对比度；modal打开、关闭、正在运动与低能力降级分别测量。
- 动效来源存活、焦点、rapid reopen、减少动态效果、图标清晰度和相邻控件不被覆盖。

上述证据未完成前不声称已找到运行时卡顿根因、已经达到目标或已修复全部响应式问题。
