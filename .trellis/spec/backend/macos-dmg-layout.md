# macOS Styled DMG Layout Contract

## 1. Scope / Trigger

Read this contract before changing the published macOS DMG Finder window,
`src-tauri/icons/dmg-background.png`, `scripts/release/create-macos-dmg.sh`,
`scripts/release/write-dmg-layout.py`, `scripts/release/render-dmg-background.mjs`,
the `dmg-layout` uv group, or `retry-hdiutil.sh` `convert` support.

GitHub-hosted `macos-15` AppleScript/Finder layout is not an allowed publisher
path. Windows NSIS appearance, notarization submit count, installer filenames,
bundle id, and `assets:icons` remain outside this contract.
Orchestration and publication still belong to
[GitHub Release Workflow](./github-release-workflow.md). Changelog heading
alignment belongs to [Application Version](./fyagent-version-contract.md) as a
`release-check` gate, not `version:set`.

## 2. Signatures

```text
create-macos-dmg.sh --app <FyAgent.app> --output <dmg> --background <png>
write-dmg-layout.py --mount <path> --app FyAgent.app --applications Applications \
  --background .background/background.png --window 660x400 --icon-size 128 \
  --app-xy 180,188 --apps-xy 480,188
retry-hdiutil.sh <destination> -- create|convert|verify ...
uv run --locked --group dmg-layout python scripts/release/write-dmg-layout.py ...
node scripts/release/render-dmg-background.mjs [--apply|--check]
```

Layout constants (window origin top-left; icon coordinates are centers):

```text
window          660 x 400 pt
background      1320 x 800 px, pHYs 5669 ppm (144 DPI)
icon size       128 pt
FyAgent.app     (180, 188)
Applications    (480, 188)
volume name     FyAgent
background file .background/background.png
```

## 3. Contracts

- `build-macos` calls `create-macos-dmg.sh` as the only styled-DMG entry after
  Developer ID app signing. The script stages `FyAgent.app`,
  `ln -s /Applications Applications`, and the checked-in background, creates
  UDRW HFS+, attaches at a fixed mountpoint (`-nobrowse -noautoopen`), writes
  `.DS_Store`, converts UDZO, and verifies.
- `write-dmg-layout.py` uses `mac_alias.Alias.for_file` on the **mounted**
  background and `ds_store` for `bwsp` / `icvp` / `Iloc`. It must not call
  `hdiutil`, `osascript`, or Finder. Staging-directory aliases bind the runner
  disk CNID and are invalid.
- AppleScript, `osascript`, `dmgbuild` CLI, `appdmg`, `pip3`, Homebrew
  `create-dmg`, and `--skip-jenkins` are forbidden. Layout failure is
  non-zero; there is no unstyled fallback.
- `retry-hdiutil.sh` allows `create`, `convert`, and `verify`. `create` and
  `convert` delete only their destination on failure. Convert must not delete
  the UDRW source. Attach/detach retries have no `-force`.
- `build-macos` uses the same pinned `astral-sh/setup-uv` and Python 3.14.7 as
  CI, then `uv sync --locked --group dmg-layout`. `dmg-layout` pins
  `ds-store==1.3.3`. `pyproject.toml` `default-groups = ["dev"]`, so default
  `uv sync --locked` does not install that group.
- Final attach of the notarized DMG must see `FyAgent.app`, Applications
  symlink to `/Applications`, `.background/background.png`, and `.DS_Store`.
- Background PNG is generated only by `render-dmg-background.mjs`. Repeat
  generation is byte-stable. `assets:icons` must not rewrite the path.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Missing FyAgent.app, background PNG, or Applications symlink | Fail before `hdiutil create` |
| Layout writer not on Darwin, mount item missing, or alias failure | Non-zero; no osascript fallback |
| `hdiutil create`/`convert` reports `Resource busy` / temporarily unavailable | Retry same argv, at most 5 attempts, 2/4/8/16s |
| Other `hdiutil` diagnostic or retry budget exhausted | Return original status; delete only convert/create destination |
| Final attach lacks `.DS_Store` or background | Fail `build-macos` |
| `ds-store` added to default uv `dev` or project dependencies | Fail the development-environment contract |
| `assets:icons` rewrites `dmg-background.png` | Fail the brand-asset contract |
| Checked-in PNG differs from `render-dmg-background.mjs --check` | Fail release-check / dmg background tests |

## 5. Good / Base / Bad Cases

- Good: mounted-volume `.DS_Store` with picture `icvp` and left/right `Iloc`;
  double-click shows app on the left and Applications on the right.
- Base: one `Resource busy` during convert, then UDZO verifies.
- Bad: `create-dmg --skip-jenkins`; AppleScript `.DS_Store`; `pip3 install
  dmgbuild`; a host-copied `.DS_Store` template; `uv add ds-store` into the
  default group.

## 6. Tests Required

- `tests/releaseWorkflow.test.ts`: script path, coordinates, `setup-uv`,
  `uv sync --locked --group dmg-layout`, Applications symlink, background and
  `.DS_Store` attach checks, no `osascript` / `skip-jenkins` / `dmgbuild` / ZIP.
- `tests/hdiutilRetry.test.ts`: convert destination deletion on busy.
- `tests/dmgBackground.test.ts`: 1320×800, `pHYs` 5669, byte-stable PNG, empty
  wells, `--check` matches the checked-in file.
- `tests/developmentEnvironment.test.ts`: `dmg-layout` pin and
  `default-groups = ["dev"]`.
- Local tests do not mount a real DMG. `build-macos` is packaging proof.

## 7. Wrong vs Correct

Wrong:

```bash
create-dmg --skip-jenkins FyAgent.dmg FyAgent.app
osascript -e 'tell application "Finder" ...'
uv add ds-store
```

Correct:

```bash
uv sync --locked --group dmg-layout
scripts/release/create-macos-dmg.sh \
  --app FyAgent.app \
  --output FyAgent-X.Y.Z-macOS.dmg \
  --background src-tauri/icons/dmg-background.png
node scripts/release/render-dmg-background.mjs --check
```
