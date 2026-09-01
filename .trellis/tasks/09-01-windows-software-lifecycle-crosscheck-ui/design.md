# 技术设计：可枚举 Registry 父键、显式 cargo-xwin 诊断与单层导航 Lens

## 1. 设计结论

1. 修复现有 owner，而不是新建扫描器或安装器。
2. cross Clippy 是显式、非验收诊断，不进入 bootstrap/default check。
3. SelectionLens 继续作为唯一共享动画 owner；SideNavigation 只窄化尺寸动画。
4. 不扩大 QoderWork、TRAE Work、WorkBuddy 的 update policy。

## 2. Windows inventory

调用链：

```text
probe_desktop
  -> discover_windows_inventory
  -> registry_hints
  -> open_*_inventory_parent
  -> enum_keys
  -> complete / incomplete projection
```

权限语义：

```text
TRAVERSE              = query + enumerate
INVENTORY_PARENT_READ = query + enumerate, no create/set
READ_VALUES           = query only
```

- 中间固定组件使用 `TRAVERSE`。
- inventory parent leaf 使用 `INVENTORY_PARENT_READ`。
- caller-controlled child component 经长度/字符验证后使用 `READ_VALUES`。
- NotFound 的可选 parent 不降低完整度；权限、枚举、link、边界或用户漂移保持 fail-closed。

## 3. Windows MSVC 交叉诊断

复用 `cargo-xwin`，由一个薄 Node owner 提供：

```text
system:check:windows-msvc-cross
  -> read-only preflight

rust:clippy:windows-msvc-cross
  -> default-no confirmation
  -> same preflight
  -> fixed cargo xwin clippy argv
```

边界：

- host 仅 `darwin-x64` / `darwin-arm64`；target 固定 Windows x64 MSVC。
- 精确解析 cargo-xwin 版本。
- 完整报告缺失前置，不在第一个失败处停止。
- 复用现有 caller override 与 Cargo-config 审计，并额外拒绝 C/CMake/xwin/native dependency 覆盖。
- 使用 argv 和 `shell:false`；脚本不调用包管理器、安装器或环境持久化。
- 不进入 bootstrap、check、CI、Release。

## 4. SelectionLens

API：

```ts
type SelectionLensGeometry = "size-and-position" | "position";
```

- 默认模式：位置和尺寸均使用现有 `fySpringTransition`。
- position 模式：位置 spring；尺寸直接 set 到目标 host。
- overlay 不因 active host 变化而 key/unmount。
- SideNavigation 通过显式 `data-collapsed-active` 清除重复 host frame。
- expanded group 的 context frame 与 leaf Lens 不重叠；collapsed group 由 Lens 单独绘制 frame。

## 5. 验证

- Rust：Registry rights 事件测试、complete/incomplete inventory projection、既有 lifecycle policy 回归。
- Task runner：host/version/prerequisite/fixed argv/override/DAG/无副作用测试。
- Frontend：SelectionLens geometry、SideNavigation material owner、V2 unit 与 Playwright 扫描稳定性。
- Contract：task metadata、generated docs、CI change classifier、supported-platform structure baseline。

## 6. 风险

- Windows vendor identity 漂移继续 fail-closed，必须通过原生 HIL 更新闭集证据。
- cargo-xwin 可能发现仅原生 Windows 才暴露的 build-script 问题；诊断失败不能通过伪造 cfg 绕过。
- position-only 尺寸变化是直接同步，这是为消除玻璃边缘拉伸而做的局部保守取舍。
