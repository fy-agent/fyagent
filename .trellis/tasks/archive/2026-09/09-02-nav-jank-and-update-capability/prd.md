# 修复配置页切换卡顿并统一软件一键更新能力

## Goal

从「AI 软件配置」切到「配置管理」下任意页面时，用户感知不到卡顿；扫描中选中导航项右侧不再出现亮点溢光。QoderWork CN、TRAE Work CN、WorkBuddy 不再提供 FyAgent 一键更新（前后端同时关闭），其它软件仍按既有安装/更新策略工作。产品能力走统一策略表 + 共享生命周期动作槽，新增软件时只改一处策略，不复制按钮逻辑。

## Background

默认落地 `#/agents`。`AgentsPage` 挂载即 `autoStart` 扫描：七路 `get_agent_install_readiness` 并行、目录卡随结果重绘、进度条出现。当前 `PersistentPrimaryOutlet` 只渲染 `<Outlet />`，主路由 lazy load 且离开即卸载。从 Agents 点「模型管理 / Skills / MCP / 提示词」时，主线程同时承担：卸载扫描树、lazy chunk、目标页首屏 hooks/查询、导航 `SelectionLens` 弹簧。这是「一定会卡一下」的来源。

扫描亮点：用户截图箭头指向「AI软件配置」选中胶囊右上角。主导航已用 `geometry="position"`，但 `.fy-selection-lens` 仍带 `backdrop-filter: blur(16px)` 与 `--fy-highlight` 内高光。透镜若亚像素溢出或采样到内容区扫描蓝条，就会在胶囊右侧出现细亮条。

一键更新回潮：`src-tauri/src/agent_install/lifecycle_policy.rs` 里 Qoder/TRAE/WorkBuddy 的 `update: true`。目录按钮只跟 `allowedActions`，后端一开，前端「一键更新」就回来。权威应仍是 `lifecycle_policy.rs`；目录槽是共享投影，不为每个软件手写 if。

08-29 曾拆掉 keep-alive，是因为 render 期 `setVisited`、隐藏页继续打查询。本次恢复 keep-alive 必须避开这两点：可见性门控查询，不在 render 里 `setState`。

## Confirmed facts

- 六主路由 lazy：`src/v2/app/router.tsx`。`PersistentPrimaryOutlet` 无 visited / `PersistentSurface`。
- `PersistentSurface` + `usePersistentVisibility` 已存在；Dialog 已尊重隐藏祖先。`usePersistentVisibility` 默认 `true`，未包裹时测试行为不变。
- `AgentsPage` 默认 `autoStart: true`；`GenericDirectoryCard` 在 `primaryAction !== null` 时拉 inventory。
- `SelectionLensGroup` 有无依赖 `useLayoutEffect(() => syncBox())`，每次 Group 重渲染都读布局。
- `.fy-side-navigation` 已有 `overflow: auto` 与自身 `backdrop-filter`；透镜额外 blur 会采样邻域。
- `lifecycle_policy.rs` 已是产品/表面/动作唯一所有者；`should_resolve_desktop_source` 在已安装时跟 `policy.update`。关掉这三家的 update 会少打远程 latest，扫描更轻。
- 目录「一键更新」正例测在 WorkBuddy 上（`tests/v2/pages/agents/Page.test.tsx`），需改到仍允许 update 的产品。

## Requirements

### R1 — 主路由切换无感知卡顿

- 已访问（含默认 `/agents`）的主路由树离开后保持挂载，用 `PersistentSurface` 设 `hidden` / `inert` / `aria-hidden`。
- 隐藏页不得发起或继续页面级查询、轮询、扫描 UI dispatch、目录 inventory/auth 拉活。进行中的 native job 仍由后端+Query 缓存持有。
- 首屏后 prefetch 六页 chunk，避免第一次点进配置管理时出现「正在加载页面」。
- 不在 render 阶段 `setState` 维护 visited。可用 ref 在 location 已变的那次渲染里登记路径。
- 离开 Agents 不拆扫描树；隐藏时暂停 reducer 更新，回来时从 Query 缓存灌回。
- NavLink `isPending` 不得在 chunk 已就绪时把选中项做成半透明卡顿感。

### R2 — 扫描中导航无右侧亮点

- 扫描过程中主导航透镜的 width/height/right 稳定（已有几何采样继续有效）。
- 选中「AI软件配置」右上角不得出现溢出高光/溢光。主导航透镜不得用 `backdrop-filter` 采样内容区。
- 几何对齐到设备像素；禁止每次无关 render 都 `syncBox()`。

### R3 — 三家国产软件关闭 FyAgent 一键更新

- QoderWork CN、TRAE Work CN、WorkBuddy：`lifecycle_policy.update = false`。`allowedActions` 不含 `update`；`start_agent_action(update)` 为 `action_not_supported`；已安装后不解析远程更新源。
- 一键安装、打开/配置、厂商应用内更新不受影响。
- Grok / Claude Desktop / OpenCode Desktop / Codex Desktop 安装器更新策略不变。
- 目录与配置页不得再为这三家画出「一键更新」。

### R4 — 共享生命周期能力与动作槽

- 后端继续以 `lifecycle_policy.rs` 为唯一准入表。新增软件只改该表。
- 前端增加只读投影 `agent-lifecycle-capabilities`（关闭集合：update UI = none | generic | codex_desktop），供目录在 readiness 返回前就知道不该为三家走 update 槽。投影与后端表必须有测试对齐。
- `AgentDirectory` 的 Generic/Codex 忙碌/安装/更新按钮合并为共享 `AgentLifecycleActionSlot`，只吃投影 + `allowedActions`，不为每个 `agentId` 复制 JSX。
- 顺带把 `queries.ts` 的页面查询与 `usePersistentVisibility` 相与，避免每个页面自己写 hidden 门控。

## Acceptance Criteria

- [ ] 从 `#/agents`（含扫描中）点「模型管理」「Skills 管理」「MCP 管理」「提示词管理」，内容区不闪「正在加载页面」，Agents 树仍在 DOM 且 hidden/inert，目标页立即可见。
- [ ] 隐藏的 Agents 在扫描未完成时不再因 settled dispatch 抢前台帧；回到 Agents 时扫描结果从缓存恢复，不无故重开一轮七路 probe。
- [ ] Playwright：扫描采样期间主导航透镜 width/height/right 单一稳定值；透镜 right 不超过「AI软件配置」host；主导航透镜 `backdrop-filter: none`。
- [ ] Qoder/TRAE/WorkBuddy 的 Rust 策略 `update == false`；安装态 readiness 不含 `update`；start update 得 `action_not_supported`。
- [ ] 目录：这三家即使 fixture 误带 `allowedActions: ["update"]` 也不渲染「一键更新」；Grok/Claude/OpenCode 在后端允许且 `update_available` 时仍可显示。
- [ ] Generic 与 Codex 目录卡共用同一个 lifecycle slot 组件。
- [ ] `mise run check`（含本次任务 prearchive exclude）通过。

## Out of scope

- 不改厂商应用自身自动更新。
- 不把 Codex 更新并进 generic `start_agent_action`。
- 不恢复 08-29 那种 render 期 `setVisited`。
- 不在启动瞬间强制挂载全部六页去抢扫描主线程（prefetch chunk + 访问后 keep-alive；空闲可再预热）。
- 不改 Skills/MCP 业务写入，不扩 catalog contractVersion。
- 不做主观「像不像玻璃」的设计重做；只消除溢光与卡顿。

## Technical notes

- 交叉层：update 准入只在 `lifecycle_policy.rs`；前端投影是显示策略，不能单独放行 native update。
- `usePersistentVisibility` 默认 true，页面测试不包 `PersistentSurface` 时查询仍启用。
- 架构测试需改写：允许 outlet 使用 `PersistentSurface`；禁止 render 期 setState；继续要求路由模块 lazy + prefetch。
