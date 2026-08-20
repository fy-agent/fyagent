# Beautify macOS DMG and require changelog on release

## Goal

用户双击正式 macOS DMG 后，看到的 Finder 窗口符合当前 V2 暮蓝玻璃风格：`FyAgent.app` 在左、`Applications` 在右，可从左拖到右完成安装。正式发版时不能再漏改 `CHANGELOG.md`。

## Background

- 当前 `build-macos` 用 `retry-hdiutil.sh` 对暂存目录做一次 `create -volname 'FyAgent' -srcfolder -format UDZO`。暂存里只有 `FyAgent.app` 和指向 `/Applications` 的 symlink。没有背景图、没有窗口尺寸、没有图标坐标。Finder 默认按名字排序，`Applications` 会排在 `FyAgent.app` 左边，和用户要的拖拽方向相反。
- `src-tauri/icons/dmg-background.png` 已存在（1320×800，浅灰底 + 中箭头 + “Drag to Applications to install”），被光栅清单封印，但 **从未拷进 DMG**。视觉也不符合现行 V2 token（`src/v2/app/styles/tokens.css`：`--fy-bg #324d69`、`--fy-bg-mid #567495`、`--fy-bg-air #7b99b8`、`--fy-text #f6fbff`、`--fy-accent #9ddcff`）。
- 发布路径已经强制 `docs/release-notes/${RELEASE_TAG}-en.md`。`CHANGELOG.md` 没有对应门禁；0.3.2–0.4.2 曾整段补写，证明这个缺口真实存在。
- 用户授权本任务直接生成新的 DMG 背景图，不另等设计稿。

## Requirements

- R1. 双击打开的 DMG 窗口使用仓库内 V2 风格背景：暮蓝渐变 + 低对比玻璃光，不画应用图标、不画 Applications 文件夹、不画 Y-gate 商标。Finder 负责那两枚真实图标。
- R2. 图标布局固定为左侧 `FyAgent.app`、右侧名为 `Applications` 且 `readlink` 为 `/Applications` 的 symlink。图标足够大，中间用背景箭头提示从左拖到右。
- R3. 窗口打开时隐藏 Finder 工具栏 / 侧边栏 / 路径栏 / 状态栏，背景铺满内容区；Retina 下背景按 144 DPI 对齐，不缩在一角。
- R4. 容器仍是只读 UDZO、卷名 `FyAgent`、产物名 `FyAgent-X.Y.Z-macOS.dmg`。不恢复 ZIP。签署、公证、装订、Applications symlink 校验、单次 DMG 公证保持不变。
- R5. 美化失败必须让 `build-macos` 失败。禁止 `create-dmg --skip-jenkins` 这类静默跳过 Finder 布局、却仍产出“成功”DMG 的路径。
- R6. 背景图由本仓库确定性生成并检入 `src-tauri/icons/dmg-background.png`。应用图标生成器（`assets:icons`）继续不得改写该文件。
- R7. 正式发版前，`CHANGELOG.md` 必须已有与 Cargo 规范版本一致的 `## [X.Y.Z]` 条目（非空正文）。该要求写入 GitHub Release 契约，并由本地/CI 可执行检查卡住，而不是只写一句提醒。

## Acceptance Criteria

- [ ] 挂载后的 DMG 仍含且仅含顶层 `FyAgent.app` 与 `Applications` → `/Applications` symlink。
- [ ] 挂载后可观察到 V2 背景、固定窗口几何，以及左 App / 右 Applications 的图标坐标；不是默认图标网格。
- [ ] 背景 PNG 为 1320×800、144 DPI；图标落点两侧留空，箭头画在两枚图标之间；像素里没有 App/Applications/Y-gate。
- [ ] `build-macos` 不引入 brew `create-dmg`、不调用 `dmgbuild` CLI、不 `pip install`、不用 `osascript`。使用仓库 uv 锁定的 `ds_store`/`mac_alias` 写 `.DS_Store`。美化失败则该 step 失败。
- [ ] `retry-hdiutil.sh` 仍禁止管道、禁止 `-force` detach、禁止杀 `diskimages`；`Resource busy` 重试语义保持。
- [ ] `mise run version:check` 的写集仍只覆盖 Cargo workspace + 两个 local lock block。Changelog 门禁走 release 检查，不塞进 `version:set`。
- [ ] `CHANGELOG.md` 缺少当前 `X.Y.Z` 标题或正文为空时，release 检查失败。
- [ ] 光栅清单 digest、release workflow 结构测试、hdiutil retry 测试、安装文档中的拖拽说明与代码-spec 同步。

## Out of Scope

- Windows NSIS 外观、捷径、签署策略。
- 改 bundle id、卷名、产物文件名、公证提交次数、或重新引入 macOS ZIP。
- 重跑 `assets:icons` 或改 About/tray 图标。
- 本次打正式 tag / 发布 GitHub Release。
- 改历史 changelog 正文或已发布 GitHub Release 页面。

## Technical Notes

- GitHub `macos-15` 上 Finder AppleScript **有时能跑**，但不能当发版契约（见 `research.md`：2025-06 provisioner 迁移导致 `-1712` 挡发布）。本任务不使用 `osascript`。
- `hdiutil` 仍只通过 `retry-hdiutil.sh`。布局由 uv 锁定的 `ds_store` + `mac_alias` 在已挂载 UDRW 卷上写 `.DS_Store`。`build-macos` 补上与 CI 相同的 `setup-uv`。
