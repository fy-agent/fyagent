# 修复 macOS 顶栏拖拽/全屏与 Windows 最大化位移

## Goal

macOS Overlay 窗口要能像其他 Mac 应用一样：最顶部有一条空白可拖、可双击缩放/全屏，六个菜单在这条空白下面。Windows 点系统最大化后，窗口必须留在当前显示器工作区内，不能瞬移，也不能把一部分界面送到屏幕外。

## Background

生产壳层是 V2：`src/index.html:27` 加载 `src/v2/main.tsx`。顶栏 `src/v2/widgets/app-shell/TopBar.tsx:6-18` 只有 Brand、六个菜单、工具区。布局是三列网格 `src/v2/app/styles/shell.css:11-18`，高度 `--fy-top-bar-height: 68px`（`src/v2/app/styles/tokens.css:40`），控件从窗口顶边开始，没有标题栏空白。

macOS 配置是 Overlay：`src-tauri/tauri.conf.json:16`。红黄绿叠在内容上，拖拽和双击依赖 `data-tauri-drag-region`。V2 契约禁止拖拽区 DOM：`.trellis/spec/frontend/v2-shell.md:22-23`、`:97-101`，测试锁死在 `tests/v2/app/router-shell.test.tsx:73-75`、`tests/v2/app/architecture.test.ts:322-366`、`tests/v2-browser/shell.spec.ts:73-74`。归档任务 `08-12-frontend-v2-native-liquid-glass` 要求恢复系统 chrome 且不改 Tauri 配置，造成 Renderer 删了拖拽区、macOS 仍是 Overlay。旧 V1 用 28px 顶条解决过同一问题：`src/App.tsx:124`、`:1235-1239`、`:1267-1276`。

Windows 配置是可见系统标题栏：`src-tauri/tauri.windows.conf.json:12`。用户说的「全屏」按系统最大化处理。可疑几何路径：`src-tauri/src/lib.rs:369-414` 启动时 `unmaximize` 后用 `inner_size` + `outer_position` clamp 再 `maximize`；`src-tauri/src/window_layout.rs:12` 把正常尺寸上限定为工作区 90%；`src-tauri/src/lib.rs:416-442` 在 `Moved` 后 150ms 调用 `refresh_main_window_layout`，`:357-363` 会 `set_min_size`。Windows 根因以运行时日志为准，本机是 macOS，不能把上述路径写成已证实。

平台开关必须用 `src/v2/shared/platform/runtime.ts` 的 `detectRuntime()`（`isNative && platform === "macos"`）。不能用 UA：Playwright 在 Mac 宿主上会误亮拖拽条。

## Requirements

- R1：macOS 保持 Overlay，不改成 Visible 系统标题栏。
- R2：仅在原生 macOS 窗口最顶部增加 28px 连续空白拖拽条。按住可移动窗口，双击触发系统「双击标题栏」的缩放/全屏。六个菜单、搜索、设置、头像保持可点。
- R3：Brand、六个菜单、工具作为同一行下移到拖拽条下方。水平排列保持左 Brand、中六个菜单、右工具，不横向挪菜单。
- R4：红黄绿落在 28px 拖拽条内，不被 Brand 或六个菜单挡住。
- R5：Windows 与浏览器预览不增加拖拽条，不把控件行下移。
- R6：Windows 系统最大化后，窗口外接矩形留在当前显示器工作区内；可见 UI 不被裁到屏幕外；还原后回到进入前的正常矩形。不实现、不验证独占全屏（F11 / `setFullscreen`）。
- R7：禁止自定义最小化/最大化/关闭按钮，禁止 `setDecorations(false)`。唯一允许的 React chrome 例外是 macOS Overlay 受控 `data-tauri-drag-region`；菜单和工具必须 `no-drag`。
- R8：浏览器预览与 Playwright 走非原生分支，检测不到拖拽条和原生窗口控件；900×600 等视口门禁保持 Brand / 导航 / 工具不重叠、不横向溢出。
- R9：更新 `.trellis/spec/frontend/v2-shell.md` 与负向测试：禁止 caption 按钮保留，原生 macOS 受控拖拽条改为允许。
- R10：不把 Linux 窗口行为带回产品范围。

## Acceptance Criteria

- [ ] AC1：原生 macOS 按住顶部 28px 空白条可拖动窗口；点六个菜单或工具区不会开始拖窗口。（R2、R7）
- [ ] AC2：原生 macOS 双击该空白条触发系统标题栏缩放/全屏；红黄绿可点，Brand 与六个菜单在空白条下方。（R3、R4）
- [ ] AC3：macOS 仍是 Overlay；六个菜单仍水平居中，只是整行下移。（R1、R3）
- [ ] AC4：Windows 系统最大化后窗口留在当前工作区内，UI 不被送到屏幕外；还原后回到进入前的正常尺寸和位置。Windows 顶栏不加拖拽条、菜单不下移。（R5、R6）
- [ ] AC5：没有自定义 caption 按钮，没有 `setDecorations(false)`。仅原生 macOS 渲染受控拖拽条；浏览器预览没有该节点。（R7、R8）
- [ ] AC6：`mise run lint:v2`、`mise run typecheck:v2`、`mise run test:v2`、`mise run test:v2:browser`、`mise run rust:test` 中与窗口布局相关的测试通过；V2 shell 契约已允许原生 macOS 拖拽条。（R9）
- [ ] AC7：macOS 本机手动验证拖拽与双击缩放。Windows 最大化不位移需要 Windows 运行时证据，不能只靠 macOS 编译通过宣称已修好。（R6）

## Out of Scope

- 业务页面、路由、液态玻璃选中态、品牌图形。
- 把六个菜单横向改到左侧或右侧。
- 把 macOS Overlay 改成 Visible 系统标题栏。
- 恢复 V1 `WindowFramePort` 或自定义 caption 按钮。
- Windows 独占全屏（盖住任务栏、隐藏标题栏）。
- 改窗口状态文件格式，除非 Windows 运行时证据证明必须改持久化字段。
- Linux / WSL 窗口支持。
- 用 Playwright 结果替代真实 Tauri 窗口验证。

## Key Decisions

- D1：macOS 保留 Overlay，允许受控拖拽区。改写 V2 shell「完全禁止 drag-region DOM」的旧契约。
- D2：空白留在窗口最顶部 28px（对齐 V1 与常见 Overlay 标题栏）。六个菜单连同 Brand、工具整行下移，不横向挪菜单。
- D3：Windows「全屏」按系统最大化修，不按独占全屏修。

## Risks

- Windows 最大化位移无法在本机 macOS 上证实。修复必须带运行时日志；验收保留 Windows 证据缺口，直到有 Windows 运行记录。
- 拖拽条开关若误用 UA，Mac 上的 Playwright 会假阳性。必须用 `detectRuntime().isNative`。
- 28px 在部分缩放显示器上可能偏紧。先锁 28px，只在本机点红黄绿失败时再调高度。
