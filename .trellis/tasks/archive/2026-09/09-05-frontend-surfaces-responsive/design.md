# 设计

`tokens.css`仍为语义视觉单一入口。色彩、surface、overlay、border、文字层级、radius、spacing以角色定义；无需为了令牌加入新runtime。公开可复用角色不能散落在pages。建议调优半径尺度12/16/22/28px加pill/circle，最终按相邻嵌套与现有视觉复核收敛，不机械统一为一个值。

模态使用已用Radix的独立overlay与content，玻璃表面和可读内容分开。先核对现有LiquidGlassLens能力及已装版本，使用backdrop滤镜做背面模糊。单独调整overlay与surface的tint/alpha，避免近不透明surface覆盖模糊、全体opacity降低文字对比或多层filter导致边框/字形失真。需要额外组件必须先过父research的许可证、WebView和GPU成本关。

响应式在实际容器宽度处理：优先fix min-width:0、minmax(0,1fr)、flex wrap、合理overflow-wrap和可滚动body；若现有vw断点确实造成窄pane失败，再使用渐进增强container query，保留当前最低WebView可用布局。共享SplitPanes拖动几何与表单内容自适应分工，不添加第二个ResizeObserver系统去弥补纯CSS问题。

规则先落shared CSS owner再处理具体例外；保持focus outline不被裁剪、按钮大小可点、图标不压扁、状态不可仅靠颜色辨认。材质开启和关闭均测，输出不能宣称全面WCAG认证。
