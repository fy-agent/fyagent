# Implement

## Checklist

1. 新增 `src/v2/shared/ui/SelectionLens.tsx` 与 `selection-lens.css`：Group overlay 几何弹簧 / Lens 登记宿主 / Track、L1 弹簧、reduced-motion、`data-testid="selection-lens"`。不要 `layoutId`。
2. 顶栏 `PrimaryNav`：Group 包六菜单（`inset={1}`）；每个 `NavLink` 内放 `SelectionLens` 登记点；保留激活项上的 `LiquidGlassLens`；去掉导航选中项静态填充。
3. `CatalogList` 内置 Group；`CatalogListItem` 在选中时渲染 Lens；去掉 catalog 选中静态填充。
4. 接入 feature tabs：Memory 类型、Skills 视图、Skills 发现来源、MCP 编辑模式、UI Lab tabs。
5. 接入 feature lists：Skills 已安装、MCP、Prompts、Memory 长期 / 每日。
6. 更新 `features.css` / `shell.css` / `catalog.css`：宿主 relative、内容在滑块之上、选中填充改由滑块承担。
7. 更新 `.trellis/spec/frontend/v2-shell.md`：记录滑动 CSS 透镜与静态 LiquidGlassLens 的分工。
8. 测试：`tests/v2/shared/SelectionLens.test.tsx`；现有 router-shell / catalog / feature 页测试保持绿色。

## Validation

```bash
mise run typecheck:v2
mise run test:v2
```

若脚本名不同，改用 `package.json` 中的 V2 等价命令。不把 Playwright 浏览器矩阵或真实桌面当成这次必过门。

## Rollback

删除 `SelectionLens*` 并还原各接入点与选中 CSS，即可回到静态选中态。`LiquidGlassLens` 依赖保持不动。
