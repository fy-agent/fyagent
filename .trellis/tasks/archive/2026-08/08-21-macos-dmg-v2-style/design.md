# Design

## Boundaries

只改 macOS 正式 DMG 的 Finder 外观，以及发版时 `CHANGELOG.md` 必须与 Cargo 版本对齐的契约。不改 Windows 安装器、公证次数、产物文件名、应用图标生成器。

```text
src-tauri/icons/dmg-background.png          检入的 V2 背景（1320×800 @144 DPI）
scripts/release/render-dmg-background.mjs   确定性绘制该 PNG
scripts/release/write-dmg-layout.py         在已挂载卷上写 .DS_Store（无 Finder）
scripts/release/create-macos-dmg.sh         UDRW → 写布局 → UDZO
scripts/release/retry-hdiutil.sh            增加 convert
.github/workflows/release.yml               setup-uv + create-macos-dmg.sh
pyproject.toml / uv.lock                    macos-dmg 组：钉死 ds_store、mac_alias
scripts/release/release-contract.mjs        changelog 标题契约
```

## DMG 几何（点 / 像素一一对应）

窗口坐标原点在内容区左上。图标坐标是图标中心。背景像素 = 点 × 2。

| 项 | 值 |
|---|---|
| 窗口 | 660 × 400 pt |
| 背景 | 1320 × 800 px，DPI 144 |
| 图标 | 128 pt |
| `FyAgent.app` | (180, 188) |
| `Applications` | (480, 188) |
| 卷名 | `FyAgent` |
| 背景文件在镜像内 | `.background/background.png` |

左侧井：(180±64, 188±64) pt → 像素约 (232–488, 248–504)。右侧井对应 (832–1088, 248–504)。箭头只画在两井之间。井内不画符号。

## 背景生成

`scripts/release/render-dmg-background.mjs` 用 Node 内置 `zlib` 写 PNG，无新依赖。

- 垂直渐变 `#324d69` → `#567495` → `#7b99b8`，顶部加低不透明 `#9ddcff` 径向光。
- 左右各一枚极淡玻璃圆盘（提示落点，不是假图标）。
- 中部浅色箭头 `#c4ebff`，其下可选一行 `--fy-text-secondary` 的 “Drag to Applications”。
- 顶部可有字重约 650 的 “FyAgent” 字标。禁止 Y-gate、App 图标、文件夹图标、截图。
- `--apply` 写入 `src-tauri/icons/dmg-background.png`，并用 `sips` 把 DPI 设为 144（仅 macOS 本地/CI 生成机；检入的是最终 PNG）。
- 重复生成必须 byte-stable，digest 写入 `scripts/tasks/supported-platform-raster-assets.json`。
- `assets:icons` 仍不得改写该路径（`application-brand-assets.md` 已有排除；本任务补上“DMG 任务才改它”）。

## 打包数据流

布局写入必须发生在 **已挂载的 UDRW 卷** 上。对暂存目录生成的 alias 会绑到 runner 磁盘的 CNID，用户打开 DMG 时背景解析会失败。

```text
signed FyAgent.app
  → stage/FyAgent.app
  → ln -s /Applications stage/Applications
  → mkdir stage/.background && cp dmg-background.png → background.png（sips 144 DPI）
  → retry-hdiutil create -format UDRW -fs HFS+ -volname FyAgent -srcfolder stage
  → hdiutil attach -nobrowse -noautoopen（固定 mountpoint）
  → uv run --group macos-dmg python scripts/release/write-dmg-layout.py \
        --mount <mount> --app FyAgent.app --applications Applications \
        --background .background/background.png \
        --window 660x400 --icon-size 128 --app-xy 180,188 --apps-xy 480,188
  → 写盘失败则 detach 并失败整个 step（禁止 skip、禁止 osascript 回退）
  → hdiutil detach（无 -force；沿用现有 cleanup；busy 时有限次再 detach，仍无 -force）
  → retry-hdiutil convert -format UDZO -imagekey zlib-level=9 → 正式 dmg 路径
  → retry-hdiutil verify
  → 现有 sign-dmg / notarize-dmg / staple / 挂载校验
```

`write-dmg-layout.py` 只做三件事：用 `mac_alias` 对挂载卷上的背景文件建 alias，用 `ds_store` 写入 icvp（图标视图、尺寸、隐藏工具栏/侧边栏/状态栏/路径栏、窗口 bounds）、图标坐标、背景 alias；不调用 `hdiutil`、`osascript`、`Finder`。

`build-macos` 增加与 CI 相同的 pinned `setup-uv` + 托管 Python 3.14.7，再 `uv sync --locked --group macos-dmg`。不使用镜像 `pip3`，不把 dmg 组打进 Linux/Windows `uv sync --locked` 默认安装集。

`create-macos-dmg.sh` 是 `build-macos` 的唯一美化入口。workflow 不再内联 `hdiutil create -srcfolder … UDZO`。挂载校验继续要求：

- `[ -d "$mount_point/FyAgent.app" ]`
- `[ -L "$mount_point/Applications" ]`
- `readlink` 等于 `/Applications`
- 另增：`.background/background.png` 存在，且挂载根上有 `.DS_Store`。

## Changelog 契约

触发：准备打 `vX.Y.Z` 或跑 `mise run release` / `release-check.mjs`。

签名：Cargo `workspace.package.version` = `X.Y.Z` 时，`CHANGELOG.md` 必须能匹配：

```text
^## \[X.Y.Z\] - 20\d{2}-\d{2}-\d{2}$
```

该标题必须是文件中 **第一个** 版本标题（允许其上保留 `# Changelog` 与 Keep a Changelog 导言）。标题与下一个 `^## [` 之间必须有非空正文（不只空白或 HTML 注释）。

不要求日期等于“今天”，不解析 GitHub Release notes 是否同文。历史条目不得被本检查改写。

`version:set` / `version:bump` 仍只写 Cargo 清单和 lock。漏写 changelog 会在 release-check 失败，而不是在 version 工具里自动插入。

## Compatibility

- 安装器 in-app 更新仍只认挂载后的单个 `.app`；多出来的 `.background` 不是 bundle，`dmg.rs` 的 `discover_single_bundle` 保持通过。
- 不改变公证：仍只提交最终 UDZO DMG 一次。
- `build-macos` 增加 uv + `macos-dmg` 依赖组；不引入 brew、不引入 `dmgbuild` CLI、不引入 `appdmg`。

## Rollback

- 恢复 workflow 内联 UDZO `create`、还原背景 PNG 与 digest、去掉 changelog 检查即可回到 0.4.2 行为。
- 已公证的旧 DMG 不受影响。
