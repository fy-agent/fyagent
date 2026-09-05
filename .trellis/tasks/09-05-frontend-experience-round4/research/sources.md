# 官方与开源调研

调研日期：2026-09-05。结论为本轮选型依据，不代表库网页的承诺已经在本项目证明。安装版本由本地 package.json/lock 与 node_modules 核对。

## 动效与组件

| 来源                                                      | 已核实含义                                                          | 项目选择                                                                                  |
| --------------------------------------------------------- | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| https://motion.dev/docs/radix                             | Radix asChild 可接 motion 元素；JS退出必须协同forceMount/presence   | 复用现有Radix语义，不手写modal/focus trap；检查退出清理、隐藏portal和焦点恢复             |
| https://www.radix-ui.com/primitives/docs/guides/animation | CSS动画也可控制原语进入/退出，Radix在退出期间延迟卸载               | 小型浮层允许官方CSS路径；不能在未协调卸载时只添加exit属性                                 |
| https://motion.dev/docs/react-transitions                 | spring有物理参数和duration/bounce两种配置方式；独立属性可用不同过渡 | 保留一个motion owner，按press/navigation/dialog区分角色；不混用冲突参数、不手写弹簧积分器 |
| https://motion.dev/docs/react-gestures                    | whileTap提供按压态，tap支持Enter路径和pointer取消                   | 复用手势机制但保留native button Space/Enter/click语义；不同时在onTap与onClick执行业务     |
| https://carbondesignsystem.com/elements/motion/overview/  | productive与expressive动效分工；距离/尺寸变化更大时持续更长         | 快速输入反馈与较舒缓对话框分开，数值需在本项目实测                                        |
| https://carbondesignsystem.com/elements/motion/resources/ | 评审要求响应及时、运动有目的、不同屏幕验证                          | 不给所有用户动作增加统一等待或全屏stagger                                                 |

已安装并复用：framer-motion 12.23.25（MIT）、@radix-ui/react-dialog 1.1.15（MIT）。当前网页可能展示较新API，实施仅用锁定版本支持的API，不为使用新示例而整体升级依赖树。Motion+付费示例不作为可复制的开源代码来源。

## 玻璃候选比较

| 来源                                                                                  | 适用性                                                                       | 决策                                                                                      |
| ------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| https://github.com/samasante/liquid-glass                                             | 已装0.1.1 MIT，已有LiquidGlassLens适配；作者称对live DOM做滤镜而非截图       | 优先核对已装源码并复用。当前仅在UI Lab使用，不代表Dialog已有此组件；不得折射表单文字      |
| https://reactbits.dev/components/glass-surface                                        | 可调玻璃表面，官方页面参数可见但网页正文不足以核实完整实现/许可证/目标浏览器 | 候选，不据演示直接新增；若现有owner确实缺能力，再检查固定commit源码、license与WebView实测 |
| https://ui.shadcn.com/docs/theming                                                    | 语义CSS变量统一颜色、圆角等                                                  | 借鉴令牌组织；不为换皮并行引入第二套Radix wrapper                                         |
| https://github.com/naughtyduk/liquidGL                                                | 额外玻璃渲染方案，需要独立结构/渲染路径                                      | 当前没有证据证明需要额外引擎，不采用                                                      |
| https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/backdrop-filter | 背景透明或部分透明才可见背后滤镜                                             | alpha0.98是当前失去透感的直接配置证据；先修复表面/遮罩组合，不叠加多层高成本filter        |

## 可访问性、性能、响应

- https://www.w3.org/TR/WCAG22/ ：普通可读文字4.5:1，较大文字3:1，必要非文字界面信息3:1。本轮以普通字4.5:1为底线，辅助信息也不得用低对比度藏起来。禁用例外单独报告，不能借此放宽可用正文。
- https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/%40media/prefers-reduced-transparency ：系统减少透明度偏好应有实色可读降级；无支持时仍要安全默认值。
- https://react.dev/reference/react/Suspense 、https://react.dev/reference/react/useTransition ：transition可避免已显示内容突然被fallback替换并可中断；它不自动消除实际CPU重任务，也不是点击卡顿万能修复。
- https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Containment/Container_queries ：size query依据容器而非viewport，inline-size containment改变布局计算；只做有fallback的渐进增强，不盲目给全部祖先加contain。
- https://www.w3.org/WAI/ARIA/apg/patterns/tabs/ ：自动激活tab的focus有业务影响；保留第三轮取消切换后恢复当前选中tab的修复。

不采用“大家都说舒服”的单一曲线、不把浏览器dev server编译时间当成产品切页性能、不把token色对比当成透明渐变实际像素的完整证明。
