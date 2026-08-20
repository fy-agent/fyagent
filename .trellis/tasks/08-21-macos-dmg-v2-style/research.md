# Research: styled macOS DMG + release changelog gate

## 优秀 DMG 窗口在做什么

对用户而言，双击 DMG 后看到的不是安装向导，而是一个被摆过的 Finder 窗口。Chrome、Slack、VS Code、Figma、Raycast、Linear 的共同模式：

1. 自定义背景图铺满内容区。
2. 应用图标在左，`Applications` 别名在右，箭头画在背景上，引导从左拖到右。
3. 背景 **不绘制** 那两枚图标；Finder 放真实 `.app` 和文件夹图标，深色/浅色模式标签才不会错。
4. 隐藏工具栏、侧边栏、路径栏、状态栏，避免像普通文件夹。
5. 图标约 100–128 pt；窗口常见 600×400 或 660×400 pt。
6. Retina：背景按窗口逻辑尺寸的 2 倍像素绘制，并把 PNG DPI 标成 144，否则背景会缩在角落。

参考：

- [create-dmg](https://github.com/create-dmg/create-dmg) v1.3.0（2026-07-02）：`--icon` 左、`--app-drop-link` 右。`--skip-jenkins` 会 **完全跳过** 美化。
- [hobbyworker 2026-05 系列](https://hobbyworker.me/en/dev/2026-05-20-design-macos-dmg-1-create-dmg-and-background/)：600×400 窗口、1200×800 @2x、图标 (175,190)/(425,190)、`sips` 设 144 DPI。
- 2026 年多个项目从 `create-dmg` 迁到 `dmgbuild`（InputMetrics #239、omi #5991），因为 headless CI 上 AppleScript 不稳定或被跳过。

## AppleScript 在 GitHub `macos-15` 上是否可当发版契约

检索目标：`build-macos` 的 `runs-on: macos-15` 上，用 Finder/`osascript` 摆 DMG 是否稳定到可以挡住公证。

证据：

1. **有时能过。** Cirrus [macos-image-templates#328](https://github.com/cirruslabs/macos-image-templates/issues/328)（2026-03）：同一套 Tauri/`create-dmg` AppleScript 在 GitHub-hosted `macos-15-arm64` 镜像 `20260209.0147` 上约 8 秒成功（`waited 1 seconds for .DS_STORE`）。Cirrus/Tart Sequoia 与 Tahoe 则稳定 `-1712` 超时。说明成功依赖「有 GUI 会话 + TCC 已放行」的托管镜像，不是脚本本身。
2. **同一条路径会整片发布失败。** GitHub [runner-images#12482](https://github.com/actions/runner-images/issues/12482) / [#12489](https://github.com/actions/runner-images/issues/12489)（2025-06-27–07-02）：`macos-14-arm64` 与 `macos-15-arm64` 上 `create-dmg` 在 `Running AppleScript to make Finder stuff pretty` 处 `-1712`。报告里包括正式发版被挡住、重试十几次无效。GitHub 工程师自己的复现 workflow 全部成功，归因为「transient / environment-specific」；随后承认是 **Hosted Compute Agent provisioner 后端迁移**，与 runner image 版本无关。Provisioner `20250620.352` 失败，`20250701.355` 后恢复。窗口大约五天。
3. **上游不把它当 CI 合同。** [create-dmg#72](https://github.com/create-dmg/create-dmg/issues/72) 的官方逃逸是 `--skip-jenkins`（丢掉全部美化）。Tauri [issue #1731](https://github.com/tauri-apps/tauri/issues/1731) 到 2026-06 仍开放：bundler 仍走 AppleScript；维护者写明「本来就不该在 CI 上能跑」，GitHub hosted 能跑更像例外；建议换有完整桌面实例的 runner，并说 *nothing changed so this won't work everywhere*。`CI=true`（Actions 默认）会让 Tauri 跳过布局。
4. **即便 AppleScript 返回 0，Finder 仍异步写 `.DS_Store`。** [dmgbuild 文档](https://dmgbuild.readthedocs.io/en/latest/) 明确这是第二失败模式：需要 GUI 会话，且无法保证卸载前 Finder 已落盘。
5. **业界在 2026 的修复方向是停用 Finder。** omi [#5991](https://github.com/BasedHardware/omi/pull/5991)、InputMetrics [#239](https://github.com/owieth/InputMetrics/pull/239)：从 `create-dmg` 迁到 `dmgbuild`，直接写 `.DS_Store`。Tauri 自己也想 port `ds-store`，五年未落地。

结论：GitHub `macos-15` **现在常常能跑** AppleScript，但这取决于 provisioner/TCC/GUI，不是仓库能锁定的输入。把失败设成 fail-closed 等于把公证绑在 GitHub 桌面会话上——2025-06 已经发生过。`--skip-jenkins` 会发布未排版 DMG，与 R5 相反。因此 AppleScript **不能** 作为本任务方案。

可行替代：在已挂载的 UDRW 卷上用 Python `ds_store` + `mac_alias` 同步写入 `.DS_Store`（dmgbuild 的核心，但不调用 dmgbuild 自己的 `hdiutil`）。`hdiutil` 仍只走 `retry-hdiutil.sh`。Python 必须走仓库 uv/`uv.lock`（[development-environment.md](../../spec/backend/development-environment.md)：禁止系统 Python 回退；`build-macos` 需加上 CI 已有的 `setup-uv` 模式）。不要 `pip3 install` 镜像 Python，不要把 `appdmg` 的 native addon 加进根 `package.json`（Linux CI 会编不过）。

## 对本仓库的约束

| 选项 | 结论 |
|---|---|
| brew `create-dmg` / 仓库 `osascript` | GitHub `macos-15` 上非契约；provisioner 故障会挡住公证。不用。 |
| `create-dmg --skip-jenkins` | 静默丢掉背景与坐标。禁止。 |
| 完整 `dmgbuild` CLI | 会自己调 `hdiutil create/attach/detach`，绕开 `retry-hdiutil.sh`，且历史上有 detach 重试问题。不用 CLI。 |
| 检入 `.DS_Store` 模板 | 别名记录了生成时的卷 CNID；本机 darwin 25 vs CI macos-15 也不一致。不作为主路径。 |
| `appdmg` / `@appdmg/ds-store` | 依赖 `macos-alias` native addon + node-gyp；不能进根 lock。不用。 |
| uv 锁定的 `ds_store` + `mac_alias`，仓库脚本写 `.DS_Store` | Finder-free、可钉版本、`hdiutil` 仍归 `retry-hdiutil.sh`。采用。 |

`retry-hdiutil.sh` 目前只允许 `create` 和 `verify`。美化需要 UDRW `create` + `convert` UDZO + `verify`。`convert` 应对目标文件采用与 `create` 相同的“失败即删除”策略。继续禁止管道、`-force` detach、杀 `diskimages-helper`。

## 现行背景图

`src-tauri/icons/dmg-background.png`：1320×800 RGB，浅灰、中箭头、英文说明。光栅清单 digest `f1864551…`。Release 步骤从未引用它。尺寸适合 660×400 pt @2x 窗口，应 **换内容并真正拷进镜像**，而不是另开路径。

V2 当前不是旧稿里的浅底 `#EAF4FC`，而是 `tokens.css` 的暮蓝玻璃。预览图只定色板与左右结构；落地像素不得把 App / Applications / Y-gate 画进背景。

## Changelog 缺口

- 发布事务已要求 `docs/release-notes/${RELEASE_TAG}-en.md`（见 `.github/workflows/release.yml` 与 `github-release-workflow.md` §8）。
- `CHANGELOG.md` 没有任何检查。commit `81662911` 回填 0.3.2–0.4.2，说明漏改发生过。
- 不能把该检查放进 `scripts/version.mjs`：`version:set` 的写集被钉死为 `Cargo.toml` + 两个 local lock block。应放在 `mise run release` / `release-check.mjs` 能跑到的 release 契约里。
