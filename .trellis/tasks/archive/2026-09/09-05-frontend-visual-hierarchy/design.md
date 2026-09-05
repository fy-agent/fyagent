# 最小边界

改变 owner：`app/styles/{tokens,controls,features,shell}.css`、`shared/ui/primitives.tsx` 和 selection-lens 样式。必要的业务 CSS 只替换排版角色，不改栅格/路由骨架。基于现有 Radix Dialog 提供明确 title/description/body/actions 类名、可选 body、取消优先的确认和一致语义。

用 role tokens 对齐页面标题、区段标题、弹窗标题、正文/辅助文案；更圆润的选择标记只改表现，不碰测量算法。原有点击、请求、确认顺序保持。先录浏览器基线，再做样式和交互修复。小窗口、长文案与 reduced motion 必须检查。
