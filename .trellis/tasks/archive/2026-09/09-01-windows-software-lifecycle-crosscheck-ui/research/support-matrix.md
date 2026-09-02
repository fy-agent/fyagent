# Supported lifecycle and evidence matrix

## 产品策略

| Product | Surface | Inventory owner | Allowed lifecycle | Update target |
| --- | --- | --- | --- | --- |
| QoderWork | Desktop | signed bundle/EXE + closed identity + Registry/App Paths/known roots | install, update, launch | one exact existing trusted candidate |
| TRAE Work | Desktop | signed bundle/EXE + closed identity + Registry/App Paths/known roots | install, update, launch | one exact existing trusted candidate |
| WorkBuddy | Desktop | signed bundle/EXE + closed identity + Registry/App Paths/known roots | install, update, launch | one exact existing trusted candidate |
| Codex | Desktop | dedicated Codex PackageManager/Desktop owner | dedicated owner | never uses Agent job slot |
| Claude Desktop | Desktop | closed macOS identity; Windows fail-closed | lifecycle where source/identity supported | exact existing trusted candidate |
| OpenCode Desktop | Desktop | closed macOS identity; Windows fail-closed | lifecycle where source/identity supported | exact existing trusted candidate |
| Grok Build | CLI | existing Tooling/helper owner | install, update | observed distribution owner |

## 状态投影

```text
optional registry parents absent or safely enumerated
+ known roots absent
+ Shell context stable
=> complete/no candidate => not_installed + reviewed fresh destination

access/enum/link/bound/context error
=> incomplete => unknown + native_projection_unavailable

one exact trusted executable/bundle
=> installed/single => launch eligible
=> update eligible only when evidence, scope and policy agree

stale, signer/product mismatch, or multiple candidates
=> visible non-green evidence; never silently pick a target
```

## 更新准入

```text
single trusted candidate
+ candidate update-eligible
+ reviewed source resolves a different comparable version
+ opaque inventory/target/revision and release capability match fresh reads
=> reuse existing platform update transaction

missing/changed/expired/ambiguous target or unsupported source
=> fail before download/write
```

- macOS 只更新 exact selected bundle，保留 rollback/scope gate。
- Windows 复用 verified vendor EXE 与 closed helper selector，不猜 silent switch；成功要求 fresh inventory 证明所选 path/scope 达到预期可信版本且无额外候选。
- cargo-xwin 是诊断证据，不是 Registry/UAC/vendor installer/launch HIL。
