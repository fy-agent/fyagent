# 技术设计

## 1. 原则

1. 修 owner，不换 owner。
2. install/update 共享 source、download、artifact verification、job、helper/DMG commit 与 post-readback。
3. Renderer 不获得 URL、路径、命令、hash、silent switch 或 bypass。
4. bootstrap 只提示；strict preflight 与可能写缓存的 Clippy 分离。
5. host 保留 Router/ARIA/focus，唯一共享 lens 绘制 frame。

## 2. Registry inventory

```text
TRAVERSE              = fixed intermediate component traversal
READ_VALUES           = enumerated child query-value
INVENTORY_PARENT_READ = leaf query-value + enumerate-subkeys
```

- 所有 capability 均无 create/set/delete/security-write。
- optional parent NotFound 保持 complete；权限、link、enum、bound、Shell drift 保持 incomplete。
- Registry/App Paths 只是证据，可信 executable 仍由 fixed relative path、ProductName、architecture、stable identity、WinVerifyTrust 与 signer subject 决定。

## 3. Managed desktop lifecycle

```text
not_observed + package installable + policy.install
  => Install

single trusted candidate
+ candidate.update_eligible
+ package installable
+ update_state != up_to_date
+ policy.update
  => Update

single trusted candidate + launch_eligible + policy.launch
  => Launch
```

- `lifecycle_policy.rs` 是 product/surface/action 唯一 owner。
- `desktop_allowed_actions` 统一 readiness 投影，UI 不自行推断。
- start 时重查 inventory/target/revision/identity/scope，并重解析 opaque release capability。
- macOS 只替换 exact selected path，保留 same-volume staging、rollback 与 system-scope gate。
- Windows 使用同一 verified vendor EXE/helper 路径；成功要求 exact selected path/scope 权威变化到预期可信版本，额外候选或歧义均失败。

## 4. Windows-MSVC tasks

```text
system:check:windows-msvc-cross:advisory
  read-only; bootstrap child; optional gaps => report + exit 0

system:check:windows-msvc-cross
  read-only; missing/unsupported => exit nonzero

rust:clippy:windows-msvc-cross
  dependency-environment; default-no; strict preflight + fixed cargo-xwin argv
```

- host 固定 darwin x64/arm64，target 固定 `x86_64-pc-windows-msvc`。
- probes：cargo-xwin、Clippy、Rust target、clang-cl、lld-link、llvm-lib、CMake、Ninja。
- 拒绝 caller target/Rust/C/CMake/xwin/Cargo-config 覆盖；无 shell、sudo、包管理器或自动安装。
- advisory 进入 bootstrap；strict/Clippy 不进入 bootstrap；整个 family 不进入 default check。

## 5. SelectionLens

```ts
type SelectionLensGeometry = "size-and-position" | "position";
```

- 默认完整几何 spring 不变。
- position 模式只 spring left/top，width/height 立即同步。
- SideNavigation active leaf/collapsed host 清除重复 frame，focus/ARIA/keyboard/reduced-motion 不变。

## 6. 风险与验证

- source/schema/signer/ProductName 漂移 fail-closed。
- vendor EXE scope 不猜测；post-readback 决定成功。
- cargo-xwin 只提供诊断，不伪造 Windows runtime/HIL。
- 覆盖 Registry fake backend、Agent lifecycle/inventory/transaction、frontend primary action/job/Page、SelectionLens/SideNavigation、Chromium geometry、task contracts 与完整 prearchive gate。
