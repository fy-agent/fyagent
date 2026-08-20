# 实施计划 — 窗口拖拽条与最大化几何

## 批次 0：运行时证据（先于行为补丁）

1. 在 `src-tauri/src/lib.rs` 的 `restore_hidden_main_window_layout`、`refresh_main_window_layout`、`Moved` 监听里记录：`is_maximized`、inner/outer size、outer position、即将调用的 `set_min_size` / `set_size` / `set_position`。
2. 在 `TopBar` 记录是否渲染拖拽条、`detectRuntime()` 的 `isNative` / `platform`、拖拽条高度。
3. 本机 macOS 复现：拖顶栏、双击顶栏、点红黄绿。Windows 最大化复现用同一套日志（本机若无 Windows，补丁仍按不变量落地，验收标为待 Windows 证据）。
4. 只根据日志修改被证实的调用点；不要在未命中的路径加防御性 `set_position`。

## 批次 1：macOS 28px Overlay 拖拽条

1. `src/v2/widgets/app-shell/TopBar.tsx`：原生 macOS 时在 Brand/Nav/Tools 上方渲染惰性拖拽条；用 `detectRuntime()`，不用 UA。
2. `src/v2/app/styles/shell.css` + `tokens.css`：拖拽条 28px；macOS 原生下 `.fy-app-shell` 第一行变为 96px；Windows/浏览器仍 68px。V2 自写 `drag` / `no-drag` 规则，不引用 `src/index.css`。
3. 控件行水平布局不改。Brand/菜单/工具随顶栏一起在拖拽条下方。
4. 验证：模拟原生 macOS 的 V2 单测能看到 `titlebar-drag-region`；默认 jsdom 看不到。

## 批次 2：V2 契约与负向测试

1. 更新 `.trellis/spec/frontend/v2-shell.md`：允许原生 macOS 这一条拖拽区；仍禁止 caption 按钮和 `setDecorations(false)`。
2. 收窄 `tests/v2/app/architecture.test.ts`、`tests/v2/app/router-shell.test.tsx`、`tests/v2-browser/shell.spec.ts`：浏览器/默认壳层仍无拖拽条；架构测试只允许 TopBar 拖拽条。
3. 验证：`mise run lint:v2`、`mise run typecheck:v2`、`mise run test:v2`、`mise run test:v2:browser`。

## 批次 3：Windows 最大化不变量

1. 按批次 0 日志，让最大化期间的布局刷新不再 `set_min_size` / `set_size` / `set_position`。
2. 启动恢复：对将最大化的窗口，clamp 持久化正常矩形，最后再 `maximize`；不要对最大化客户区做 90% 工作区 clamp。
3. 纯函数/状态测试覆盖「最大化标记保留、正常矩形不被改成 90% 伪全屏」。
4. 验证：`mise run rust:test`。Windows 手工最大化留作 AC7 证据，不在本机 macOS 上宣称已关闭。

## 验证命令

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run rust:test
```

桌面壳层若触发既有门禁，再跑 `mise run test:desktop:mock`。不要把 mock/Playwright 报成真实 Windows 最大化证据。

## 风险文件与回滚点

| 文件 | 风险 | 回滚 |
| --- | --- | --- |
| `src/v2/widgets/app-shell/TopBar.tsx` | 拖拽条误在浏览器出现 | 还原为无拖拽条 header |
| `src/v2/app/styles/shell.css` | 顶栏高度/重叠 | 还原 68px 单行网格 |
| `tests/v2/app/architecture.test.ts` 等 | 负向断言过宽或过窄 | 与 v2-shell 契约一起回滚 |
| `src-tauri/src/lib.rs` | 最大化期间改几何 | 撤销布局监听/恢复序列中的 set_* 变更 |
| `src-tauri/src/window_layout.rs` | 90% clamp 误伤正常窗口 | 只改最大化分支，保留正常窗口 90% 上限 |

批次 1/2 与批次 3 可独立回滚。

## `task.py start` 前检查

- [x] `prd.md` 无阻塞 Open Questions
- [x] `design.md` / `implement.md` 已写
- [ ] 用户已明确批准本轮最终规划摘要
- [x] `implement.jsonl` / `check.jsonl` 有真实 spec 条目
- [ ] 批准后才 `task.py start`；批准前不改产品代码（批次 0 日志也等到 start 之后）
