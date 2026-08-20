# Research: large tracked files

- **Query**: Largest tracked git files; binaries, screenshots, lockfile duplicates, vendored blobs; safe cleanup only (`.gitignore` + `git rm --cached`, not history rewrite)
- **Scope**: internal
- **Date**: 2026-08-20

## Findings

### Current tree (HEAD index)

Measured with `git ls-files` + filesystem sizes on 2026-08-20:

| Metric | Value |
|---|---|
| Tracked files | 2141 |
| Tracked bytes | 40 565 985 (~38.7 MiB) |
| `.png` share | 18 272 249 bytes, 143 files (~45% of the tree) |
| `.rs` share | 8 632 288 bytes |
| `.md` share | 4 285 366 bytes, 572 files |
| Test files (`tests/**` + colocated `*.test.*`) | 238 files, 2 250 597 bytes |

There is **no** `package-lock.json` or `yarn.lock`. Four lockfiles exist for four ecosystems: `pnpm-lock.yaml` (234 189), `src-tauri/Cargo.lock` (191 962), `mise.lock`, `uv.lock` (virtual Python env, `requires-python = "==3.14.*"`, no third-party packages). These are not duplicate lockfiles of the same manager.

### Files Found — largest currently tracked

| File Path | Size (bytes) | Kind |
|---|---|---|
| `docs/fyagent/marketing/assets/samples/fyagent-tactile-orchestration-hero-v3.png` | 1 718 652 | marketing concept sample |
| `docs/fyagent/marketing/assets/samples/fyagent-unified-control-hero-v1.png` | 1 710 317 | marketing concept sample (`status: superseded` in sibling md) |
| `docs/fyagent/marketing/assets/samples/fyagent-tactile-orchestration-hero-v2.png` | 1 553 252 | marketing concept sample (`status: superseded`) |
| `assets/screenshots/main-zh-2.png` | 929 682 | public README screenshot |
| `assets/screenshots/main-zh-3.png` | 761 033 | public README screenshot |
| `docs/user-manual/assets/image-20260108010324583.png` | 625 501 | user-manual screenshot (Typora-style timestamp name) |
| `assets/screenshots/main-zh-1.png` | 484 858 | public README screenshot |
| `docs/user-manual/assets/image-20260108004946288.png` | 434 812 | user-manual screenshot |
| `docs/images/codex-claude-routing/03-anthropic-advanced-options.png` | 387 591 | routing how-to screenshot |
| `CHANGELOG.md` | 349 437 | generated/accumulated changelog text |
| `src-tauri/src/services/proxy.rs` | 284 526 | source (not binary) |
| `pnpm-lock.yaml` | 234 189 | frontend lockfile |
| `src-tauri/icons/icon.icns` | 173 043 | desktop bundle icon |
| `assets/brand/github/fyagent-social-preview.png` | 163 545 | GitHub social preview (1280×640, asserted in `currentDocsContract`) |
| `assets/fyagent.png` | 56 130 | canonical 1024 RGBA brand source |

### PNG inventory by directory (currently tracked)

| Prefix | Files | Bytes |
|---|---|---|
| `docs/user-manual/assets` | 40 | 5 921 588 |
| `docs/fyagent/marketing` | 3 PNGs (+ md) | 4 982 221 of 5 042 816 dir total |
| `docs/images/codex-claude-routing` | 4 | 1 303 009 |
| `docs/images/codex-kimi-routing` | 3 | 958 542 |
| `assets/screenshots/main-zh-{1,2,3}.png` | 3 | 2 175 573 |
| `docs/images/claude-codex-routing` | 3 | 853 531 |
| `src/icons/extracted` | 22 PNGs in a larger extracted set | 625 134 |
| `docs/images/codex-deepseek-routing` | 3 | 370 863 |
| `src-tauri/icons/android/` | 15 | 141 040 |
| `src-tauri/icons/ios/` | 18 | 135 361 |

`docs/images/` total: 14 files, 3 675 142 bytes.

### Byte-identical tracked copies (SHA-256 groups)

84 duplicate groups with size ≥ 200 bytes. Classes:

**Product icons copied into V2**

| Bytes | Paths |
|---|---|
| 91 948 | `src/assets/workbuddy-icon-512.png` = `src/v2/shared/assets/agents/workbuddy.png` |
| 39 304 | `src/icons/extracted/hermes.png` = `src/v2/shared/assets/apps/hermes.png` |
| various SVGs | `src/icons/extracted/{meta,gemini,huggingface,ollama}.svg` duplicated under `src/v2/shared/assets/` |

`tests/v2/shared/appAssets.test.ts` asserts some V2 copies are byte-identical to extracted sources. `scripts/tasks/supported-platform-raster-assets.json` lists **both** workbuddy paths with the same digest.

**Tauri icon generator extras**

- Android `ic_launcher.png` = `ic_launcher_round.png` at each mipmap density.
- iOS `AppIcon-20x20@2x.png` = `AppIcon-20x20@2x-1.png` = `AppIcon-40x40@1x.png`; similar `-1` duplicates for 29/40 px.
- About icon `src/assets/icons/app-icon.png` is asserted byte-identical to `src-tauri/icons/32x32.png` in `tests/applicationBrandAssets.test.ts` lines 59–61.

`src-tauri` JSON/TOML in this snapshot has **no** `android` / `ios` product keys; those icon trees are still in the raster digest freeze.

**Trellis platform mirrors (intentional copies, large in aggregate)**

| Prefix | Files | Bytes |
|---|---|---|
| `.agents/skills/` | 46 | 242 545 |
| `.cursor/skills/` | 43 | 233 306 |
| `.codebuddy/skills/` | 43 | 233 306 |
| `.cursor/hooks/` | 3 | 83 215 |
| `.codebuddy/hooks/` | 4 | 102 023 |
| `.codex/hooks/` | 3 | 76 050 |

Many skill/hook files are triple-copied (`.agents` / `.cursor` / `.codebuddy`). Example: `inject-subagent-context.py` 37 724 × 3. This is the Trellis multi-platform install layout, not a random dump, but it is repeated tracked text.

### History-only blobs (still in git objects, not in current index)

`git rev-list --objects --all` + `git cat-file`: 33 paths ≥ 500 KiB that are **not** currently tracked. Largest:

| Size | Path (history only) |
|---|---|
| 4 235 544 | `.tmp/fyagent-visual-review/FyAgent-For-You-Gate-review.zip` |
| 1 679 818 | `.tmp/fyagent-visual-review/05-agent-constellation-hero.png` |
| 1.4–1.7 MiB each | `.trellis/tasks/archive/2026-08/08-12-for-you-gate-workbuddy-visual/research/*.png` (many) |
| 1 459 236 | `icon.png` (repo-root historical) |
| 1 438 227 | `src/icons/extracted/dds.svg` |
| 1.2–1.3 MiB | `assets/partners/banners/*.png`, `assets/partners/logos/etok.png` |

Older `src-tauri/icons/icon.icns` revisions were 1.0–2.1 MiB; current tracked icns is 173 043.

**Safe cleanup cannot remove these objects.** `.gitignore` + `git rm --cached` only affect the next commit's tree. History rewrite is out of scope for this task.

### `.gitignore` vs bloat classes

Root `.gitignore` currently covers `node_modules/`, `dist/`, `.venv/`, `/release/`, `/artifacts/`, `.worktrees/`, logs, env files, editor dirs. It does **not** mention:

- `.tmp/`
- `docs/fyagent/marketing/assets/samples/`
- visual-review zips
- `assets/partners/`

`.gitattributes` already routes `tests/e2e/visual-baselines/**/*.png` to Git LFS; that directory currently contains only `manifest.json` and `README.md` (no PNG baselines tracked).

### Contract coupling (untracking would fail tests as they stand)

These large binaries are pinned by digest or inventory tests:

1. `scripts/tasks/supported-platform-raster-assets.json` — SHA-256 list including the three marketing sample PNGs, all 40 user-manual PNGs, routing how-to PNGs, README screenshots, android/ios/desktop icons, extracted vendor icons, V2 agent/app PNGs.
2. `tests/remainingPlatformSurface.test.ts` — "freezes the decoded and visually reviewed raster inventory by path and digest".
3. `tests/currentDocsContract.test.ts` — README screenshot paths must exist and be referenced; user-manual PNG count 40; `VISUAL_DELIVERABLES` markdown files must exist; marketing sample docs must contain status strings that embed those PNGs.
4. `tests/applicationBrandAssets.test.ts` — `assets/fyagent.png` digest `9e2ceb57…`.

`assets/fyagent.png` is **small** (56 KiB) and is the canonical brand source; READMEs are forbidden from using it as an `<img src>` (`currentDocsContract` line 374).

### Other tracked binaries / unusual files

| Path | Size | Note |
|---|---|---|
| `src-tauri/icons/icon.ico` | 26 931 | desktop |
| `src/icons/extracted/fenno-icon.webp` | 33 198 | vendor icon |
| `scripts/release/apple-developer-id-g2-ca.cer` / `apple-root-ca.cer` | 1.5–1.7 KiB | Apple CA fixtures |
| `tests/fixtures/windows-nsis/*.pem` | ~1.3 KiB | fake signing fixtures |
| `deplink.html` | 104 906 | tracked HTML (filename spelling `deplink`) |
| `.trellis/**/*.jsonl` | 114 files, 98 777 bytes | Trellis task traces; small individually |

### Related Specs

- `.trellis/spec/backend/application-brand-assets.md` — `assets/fyagent.png` as icon source; generated `src-tauri/icons/**`
- `.gitattributes` LFS comment for future visual baselines
- `scripts/audit/repository-governance-scan.mjs` (exercised by `tests/repositoryGovernanceScan.test.ts`) reports large-blob **findings** from git history in a redacted shape; that is a scanner, not a cleanup tool

## Caveats / Not Found

- Current-tree largest files are documentation/marketing/manual screenshots, not vendored `node_modules` or checked-in build artifacts.
- No duplicate pnpm/Cargo lock pair.
- `CHANGELOG.md` at 349 KiB is large text, not a binary.
- Android/iOS icon trees are small (~276 KiB combined) relative to marketing PNGs; they are digest-frozen anyway.
- History still holds a 4.2 MiB zip and many 1.5 MiB research PNGs; clone size is dominated by **history**, which safe cleanup does not shrink.
- Whether any currently tracked PNG is "useless" is coupled to the docs/raster contracts above; those contracts currently require the three largest files to remain.
