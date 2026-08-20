# Implement

## Ordered checklist

1. 写 `scripts/release/render-dmg-background.mjs`（几何常量与 design.md 表一致），生成 1320×800 PNG，本地用 `sips` 设 144 DPI，`--apply` 覆盖 `src-tauri/icons/dmg-background.png`。目视确认井内无假图标。
2. 更新 `scripts/tasks/supported-platform-raster-assets.json` 中该路径 digest。
3. 扩展 `scripts/release/retry-hdiutil.sh`：允许 `convert`，失败删除目标输出；`create`/`verify` 行为不变。补 `tests/hdiutilRetry.test.ts`。
4. 在 `pyproject.toml` 增加可选组 `macos-dmg`（钉死 `ds_store`、`mac_alias`），更新 `uv.lock`。Linux/Windows 默认 `uv sync --locked` 不得安装该组。
5. 新增 `scripts/release/write-dmg-layout.py`：只对已挂载路径写 `.DS_Store`。新增 `scripts/release/create-macos-dmg.sh`：暂存 app + Applications symlink + 背景、UDRW、调用 layout 脚本、convert UDZO。美化失败非 0。禁止 `osascript`。
6. `build-macos`：加入与 CI 相同的 pinned `setup-uv` + 托管 Python，然后调用 `create-macos-dmg.sh`。保留 sign/notarize/staple/attach 校验，并断言 `.background/background.png` 与 `.DS_Store`。步骤名若仍写 ZIP 则改掉。
7. 更新 `tests/releaseWorkflow.test.ts`：要求 `setup-uv`、`create-macos-dmg.sh`、`write-dmg-layout.py`、背景路径、左/右坐标；禁止 `osascript`、`skip-jenkins`、`dmgbuild` CLI、ZIP；仍校验 Applications symlink。
8. Changelog 检查：在 `scripts/release/release-contract.mjs`（或紧邻的纯函数）实现标题契约，从 `release-check.mjs` 调用；单测覆盖缺标题、空正文、版本不匹配。
9. 更新 `.trellis/spec/backend/github-release-workflow.md`（Finder-free DMG 布局 + changelog 门禁 + build-macos uv）、`application-brand-assets.md`、必要时 `development-environment.md` / `fyagent-version-contract.md`。
10. 安装说明保持拖拽安装语义。跑 `mise run release-check` 能跑的测试。不打 tag。

## Validation

```bash
# 背景 + 光栅清单
node scripts/release/render-dmg-background.mjs   # preview; --apply 仅在采纳后
mise run supported-platform:check                # 或 release-check 内嵌的那步

# 契约
pnpm exec vitest run tests/hdiutilRetry.test.ts tests/releaseWorkflow.test.ts
node scripts/tasks/release-check.mjs --ci
```

本机完整“做成 DMG 并打开 Finder”需要已有 `FyAgent.app`；CI `build-macos` 才是正式证据。规划阶段不声称已目视验收。

## Risky files

- `.github/workflows/release.yml`：结构测试对具体行敏感。
- `scripts/release/retry-hdiutil.sh`：convert 不得削弱 create/verify 的 busy 重试。
- `scripts/tasks/supported-platform-raster-assets.json`：digest 必须与新 PNG 字节一致。
- `CHANGELOG.md`：检查只读当前版本标题，不要重排历史。

## Rollback points

- 脚本未调用前：只改背景生成也不影响已发布 0.4.2。
- workflow 切换后若 layout 脚本失败：job 失败，不会发布未排版 DMG。不回退到 `osascript` 或 `--skip-jenkins`。

## Follow-up before `task.py start`

- 规划摘要已给用户，并得到明确同意。
- `implement.jsonl` / `check.jsonl` 已有真实 spec 条目。
- 不在本机打 tag、不跑正式 Release。
