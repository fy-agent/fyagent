# Design

## Boundaries

只改 V2 renderer、对应 Vitest，以及 V2 shell 契约中与选中透镜相关的段落。不改 Tauri、遗留 `src/components/**`、依赖版本。

```text
shared/ui/SelectionLens.tsx     # 唯一 framer-motion 适配器
  |- SelectionLensGroup(id)     # 轨道 scope + 单一 overlay 滑块
  |- SelectionLens(active)      # 仅 active 时登记宿主，不绘制滑块
  `- SelectionLensTrack         # Group 的 DOM 包裹，供 tabs / 列表复用

接入：
  PrimaryNav                    # 六菜单（inset=1）
  CatalogList / CatalogListItem # Agent / 模型侧栏
  feature tabs / list items     # Skills, MCP, Prompts, Memory
  UiLabPage tabs
```

## Motion

源项目用 `layoutId` scale 投影。V2 的滑块带 `backdrop-filter`，运行时在大→小切换时测到 `scaleX≈0.29` 且 `scaleY≈1`，胶囊变形，文字被滤镜拉糊。因此 V2 **不**使用 `layoutId` / `LayoutGroup`。

- 每轨一个 overlay `motion.div`，弹簧驱动 `left` / `top` / `width` / `height`
- 默认弹簧：`{ type: "spring", stiffness: 520, damping: 42, mass: 0.62 }`
- `useReducedMotion()` 为真时改为 `{ duration: 0 }`
- 新点击改目标盒，弹簧立即改向，不排队 CSS transition
- 禁止用 `transform: scale` 插值尺寸

`framer-motion` 只允许从 `SelectionLens.tsx` 导入。

## Material

源项目：`bg-(--surface-muted) shadow-(--shadow-control) rounded-(--radius-control)`。

映射到 V2 L3 interactive glass，不引入灰色不透明底：

- 填充：`--fy-glass-interactive` + 现有透镜渐变
- 边：`--fy-border-strong`
- 内高光：`--fy-highlight`
- 外阴影：`--fy-shadow-control`
- 模糊：`blur(16px) saturate(1.3)`（CSS backdrop，不是 SVG filter；几何动画下不缩放该滤镜）
- 圆角：从当前宿主 `border-radius` 复制；顶栏 `inset={1}` 对齐 36px 透镜

选中宿主去掉自己的 `background` / 选中描边 / 选中阴影，只保留文字色，避免滑块还在路上时新项先亮。

## LiquidGlassLens

契约不变：生产实例最多一个，且只在激活 `NavLink` 内。它不再提供滑动填充；导航里的 `.fy-liquid-glass-lens` 去掉与滑块重复的底、边、阴影、backdrop，只留几何给折射。UI Lab 标本仍用完整透镜外观。

禁止把 `Glass` 放到滑动 overlay 上。文字不得进入任何 scale 投影节点。

## Integration

每个互斥选项组一个 `SelectionLensGroup` 作为定位 scope。选项按钮在滑块之上（`z-index: 1`）。`SelectionLens` 只在选中项里登记宿主。tabs 的文本包一层 span。

不把 Switch、Checkbox、`<select>`、分页当成选项轨。
