# 窗口拖拽条与最大化几何 — 技术设计

## 1. 边界

本任务改三处，互不把窗口控件所有权交回 React：

```text
原生 macOS Overlay
  AppShell / TopBar
    28px 受控拖拽条          ← 仅 isNative && macos
    68px Brand | Nav | Tools
  V2 shell 契约 + 负向测试   ← 允许这一条，仍禁止 caption 按钮

原生 Windows Visible 标题栏
  src-tauri 窗口布局
    最大化期间禁止改 size/position/min_size
    启动恢复先钳制正常矩形，最后再 maximize
```

不改 `titleBarStyle`。不引入 `WindowFramePort`。不调用 `setDecorations(false)`。不实现独占全屏。

## 2. macOS Overlay 拖拽条

### 2.1 何时渲染

只用现有 `detectRuntime()`（`src/v2/shared/platform/runtime.ts`）：

```ts
const runtime = detectRuntime();
const showMacOverlayDragStrip =
  runtime.isNative && runtime.platform === "macos";
```

`AppShell` 或 `TopBar` 可以读这个值；widgets 允许依赖 `shared`。禁止用 `navigator.userAgent` 判断，否则 Mac 宿主上的 Playwright 会画出拖拽条。

### 2.2 DOM 与拖拽契约

`TopBar` 结构变为：

```text
header.fy-top-bar
  [native macOS] div.fy-titlebar-drag-strip
                  data-testid="titlebar-drag-region"
                  data-tauri-drag-region
  div.fy-top-bar-chrome
    Brand | PrimaryNav | ToolCluster
```

- 拖拽条高度 28px，不进 Tab 顺序，没有按钮或可访问名称。
- 六个菜单和工具保持可点：拖拽属性只打在空白条上；若祖先带 `drag`，子控件必须 `data-tauri-no-drag` / `.no-drag`（已有 `src/index.css` 规则；V2 不引用 legacy CSS，必须在 `src/v2/app/styles/shell.css` 写等价规则）。
- 水平排列不变。`.fy-app-shell` 的第一行高度在 macOS 原生下改为 `28 + 68`，Windows / 浏览器仍是 68。

### 2.3 测试与契约收窄

`.trellis/spec/frontend/v2-shell.md` 改为：

- 仍禁止自定义 caption 按钮和 `setDecorations(false)`。
- 允许且仅允许原生 macOS Overlay 在 TopBar 顶部放一条惰性 `data-tauri-drag-region`。
- 浏览器预览不得出现该节点。

测试：

- 默认（jsdom / Playwright，非 Tauri）：继续断言没有拖拽条、没有窗口按钮。
- 模拟 `detectRuntime()` 为原生 macOS：必须出现 `titlebar-drag-region`，Brand/Nav/Tools 在它下面且互不重叠。
- `architecture.test.ts` 把「任何 `data-tauri-drag-region` 都失败」收窄为：只允许 TopBar 拖拽条这一处；caption 标识符和 `setDecorations(false)` 仍失败。

## 3. Windows 系统最大化

用户要修的是系统最大化，不是 `setFullscreen`。`window_state_flags()` 本来就不持久化 fullscreen，本任务保持这一点。

### 3.1 当前危险序列

启动：`windows_window_state::restore` 已经按保存的正常矩形 `set_size` / `set_position`，必要时 `maximize`。随后 `restore_hidden_main_window_layout` 再 `unmaximize`，用现场 `inner_size` + `outer_position` clamp 到工作区 90%，再 `set_size` / `set_position` / `maximize`。若 `unmaximize` 尚未生效，`set_position` 会作用在仍最大化的窗口上，表现为瞬移、一部分 UI 到屏幕外。

运行中：最大化会触发 `Moved`（Windows 常见坐标如 `-8,-8`）。150ms 后 `refresh_main_window_layout` 调用 `set_min_size`。最大化期间改 min size / size / position 都可能把窗口打出工作区。

### 3.2 目标行为

不依赖尚未发生的运行时日志做最终补丁选择，但补丁必须满足这些不变量：

1. `is_maximized() == true` 时，不得 `set_size`、`set_position`、`set_min_size`。
2. `Moved` 触发的布局刷新在最大化期间只允许 `emit_main_window_layout_mode`，不得改窗口几何。
3. 启动恢复若窗口将最大化：用持久化的**正常矩形**（Windows 状态里的 `width`/`height`/`x`/`y` 或 `prev_*`）做 clamp，最后一步才 `maximize`。不要在最大化几何上做 90% 工作区 clamp。
4. 还原最大化后，正常矩形必须是进入最大化前的那一份，而不是被 90% clamp 改写过的最大化客户区。

实现前先打日志（见 `implement.md` 批次 0），用 `is_maximized`、inner/outer size、position、是否调用了 set_* 证实哪一条路径在最大化瞬间改了几何。只修被日志证实的调用点。

### 3.3 测试

在 `window_layout` 纯函数层补充：最大化标记不得把正常矩形 clamp 成「工作区 90% 的伪最大化尺寸」。Rust 侧对「最大化期间跳过 set_min_size / set_position」用可测的辅助函数或现有 `windows_window_state` 记录逻辑覆盖。本机 macOS 的 `rust:test` 不能替代 Windows 手工最大化。

## 4. 兼容与回滚

- 不改窗口状态 JSON 字段，除非日志证明缺字段无法恢复正常矩形。现有 `prev_x` / `prev_y` / `maximized_x` / `maximized_y` 已能表达正常矩形与最大化目标。
- macOS 拖拽条是纯 UI + 契约测试，回滚即还原 TopBar / shell.css / v2-shell 负向断言。
- Windows 几何补丁与 macOS 拖拽条可独立回滚。

## 5. 权衡

| 方案 | 不采用的原因 |
| --- | --- |
| macOS 改 Visible 标题栏 | 产品已否决；会在玻璃顶栏上再加一条系统栏 |
| 六个菜单左移或右移 | 产品已否决；其他 Mac 应用留的是顶部一条，不是横向空洞 |
| React 自定义红黄绿/最大化按钮 | 违反 V2 系统 chrome 边界 |
| 用 UA 切换拖拽条 | 会污染 Mac 上的浏览器预览和 Playwright |
| 把最大化 clamp 成工作区 90% | 最大化就应该填满工作区；90% 只约束正常窗口 |
