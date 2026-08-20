# 移植可打断选择滑块到全部切换 UI

## Goal

把 myself-todolist 桌面端「智能视图」那套可打断滑块（运动 + 材质）做成 FyAgent V2 可复用组件，并接到所有同类选项切换：顶栏六菜单、侧栏目录、segmented / tabs、主从列表。用户连续点击时，滑块立即改目标，不排队播完上一段。

## Background

源项目 `SelectionLens`（`myself-todolist/frontend/src/components/app-shell/SelectionLens.tsx`）只在当前选中项渲染一个 `absolute inset-0` 的 pill，并用 Motion `layoutId` 在同一 `LayoutGroup` 里 morph。弹簧是 L1 control：`stiffness 520 / damping 42 / mass 0.62`。材质是 `--surface-muted` 填充、`--shadow-control` 阴影、`--radius-control` 圆角。不测宽、不排队 CSS keyframes。

FyAgent 生产壳是 V2（`src/index.html` 加载 `src/v2/main.tsx`）。顶栏六菜单目前用 `LiquidGlassLens` 包住激活 `NavLink` 的标签：折射是静态挂载，切换时瞬间换位置，不会滑动。V2 契约禁止把 SVG filter 做跨布局动画，且生产实例最多一个。侧栏等价物是 `CatalogListItem`；Skills / MCP / Prompts / Memory 还有 `fy-feature-tabs` 与 `fy-feature-list-item`。

## Confirmed Facts

- 源滑块可打断，是因为共享视觉元素会立刻改目标，而不是 CSS width 动画队列。
- fyagent 已有 `framer-motion`；V2 滑块带 `backdrop-filter`，不能照搬源项目的 `layoutId` scale 投影（大→小会 `scaleX≈0.29` 并拉糊文字）。
- 选中态今天靠元素自身 `background: var(--fy-selected)`。若滑块滑动时仍保留这层填充，新项会先闪出背景。
- 遗留 V1 Settings/Usage Tabs 不在生产入口。

## Requirements

- R1：新增 V2 内部可复用滑块组件，移植源项目的可打断弹簧手感，以及 pill 的填充 / 阴影 / 模糊 / 圆角 / 对比边。用 overlay 几何弹簧，不用 `layoutId`。颜色映射到 `--fy-*` 玻璃 token，不搬源项目业务。
- R2：顶栏六菜单切换使用该滑块。`LiquidGlassLens` 仍只挂在激活 `NavLink` 内，不承担滑动，也不把 SVG filter 放到 overlay 上。
- R7：大项切到小项时滑块不得非等比缩放变形；滑块上的标签文字保持清晰。
- R3：Agent / 模型侧栏目录选择使用同一组件。
- R4：所有 segmented / tabs 与主从列表选择接上，包括 Memory 类型、Skills 视图与发现来源、MCP 编辑模式、各页 `fy-feature-list-item`、UI Lab tabs。
- R5：尊重 `prefers-reduced-motion`：减弱为瞬间换位，保留选中语义。
- R6：不改 Switch / Checkbox / 下拉框 / 分页；不把遗留 V1 tabs 纳入生产范围。

## Acceptance Criteria

- [x] AC1：每组只有一个 `aria-hidden` overlay 滑块；`SelectionLens` 只登记当前宿主。（R1）
- [x] AC2：运动配置与源项目 L1 control 弹簧一致；新点击改目标而不排队。（R1）
- [x] AC3：顶栏六菜单切换时滑块在菜单间滑动；仍恰好一个 `LiquidGlassLens`。（R2）
- [x] AC4：Catalog 侧栏与 feature 列表切换时滑块跟随当前 `aria-current` 项。（R3、R4）
- [x] AC5：feature tabs 与 UI Lab tabs 切换时滑块跟随 `aria-selected` / `data-state`。（R4）
- [x] AC6：选中项不再另铺一层会抢先出现的静态填充；文字仍在滑块之上。（R1）
- [ ] AC7：`mise run typecheck:v2` 与相关 V2 测试通过；新增组件测试覆盖 active/inactive 挂载。（R1–R5）
- [ ] AC8：大→小切换时 overlay 保持 `scaleX/scaleY === 1`，标签 `filter` 为 `none`。（R7）

## Out of Scope

- 窗口拖拽 / 全屏（现有另一任务）
- 遗留 V1 Settings、Usage、Hermes tabs
- 把 `<select>`、开关、复选框、分页改成滑块
- 引入新动画库或改 `@samasante/liquid-glass` 版本
- git commit
