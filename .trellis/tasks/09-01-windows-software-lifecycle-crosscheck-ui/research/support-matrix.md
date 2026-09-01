# Supported lifecycle and evidence matrix

## 产品策略

| Product | Surface | Windows owner | Allowed actions | 本任务 |
| --- | --- | --- | --- | --- |
| QoderWork | Desktop | signed EXE + Registry/App Paths/known roots | install, launch | 修复 inventory 完整度 |
| TRAE Work | Desktop | signed EXE + Registry/App Paths/known roots | install, launch | 修复 inventory 完整度 |
| WorkBuddy | Desktop | signed EXE + Registry/App Paths/known roots | install, launch | 修复 inventory 完整度 |
| Codex | Desktop | PackageManager/MSIX dedicated owner | dedicated owner | 不改路由 |
| Claude Desktop | Desktop | Windows identity 未闭合 | fail-closed | 不猜测身份 |
| OpenCode Desktop | Desktop | Windows identity 未闭合 | fail-closed | 不猜测身份 |
| Grok Build | CLI | Tooling/helper owner | install, update | 不受影响 |

## 状态投影

```text
optional registry parents absent/successfully enumerated
+ known roots absent
+ Shell context stable
=> complete inventory, no candidate
=> not_installed + reviewed fresh destination

access/enumeration/link/bound/context error
=> incomplete inventory
=> unknown + native_projection_unavailable
=> no executable fresh destination

one exact trusted executable
=> single/installed

stale, signer/product mismatch, or multiple candidates
=> visible non-green evidence; never silently pick a target
```

## 证据声明

Portable tests 和 macOS cargo-xwin 可以证明解析、策略、编译合同和 UI 行为；不能证明 Explorer SID、真实 registry、vendor installer/UAC、当前制品 Authenticode、custom path 或桌面启动。这些仍属于 Windows native HIL。
