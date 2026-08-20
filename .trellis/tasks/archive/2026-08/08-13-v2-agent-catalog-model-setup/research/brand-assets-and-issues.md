# Research: QoderWork / TRAE brand assets and FyAgent issues

- Query: Find official QoderWork and TRAE icon/logo sources, record exact retrievable URLs and technical metadata, determine the available brand-use/license evidence, and identify FyAgent Issues/PRs that materially constrain the V2 Agent catalog, Models page, and installer-branding work.
- Scope: mixed (local repository, official vendor sites/repos/docs, FyAgent GitHub Issues/PRs)
- Date: 2026-08-13 (Asia/Shanghai)

## Findings

### 1. Recommended official icon inputs

#### QoderWork CN

Use the Qoder product mark published as the favicon on both the current official global site and the requested CN product URL. It is the highest-quality square official source found and is suitable for a catalog tile after deterministic local processing.

- Product entry: `https://qoder.com.cn/qoderwork`
  - The official Aliyun documentation identifies QoderWork CN as the Qoder CN family desktop work assistant: `https://help.aliyun.com/zh/lingma/qoderwork-cn/product-overview/what-is-qoderwork-cn`.
  - Retrieval caveat: on 2026-08-13, the raw HTML returned for this path had already changed to the broader Qoder CN product-family landing page and did not contain the `QoderWork` string, even though search/document indexes still describe the requested page as QoderWork. Treat the official documentation as product-identity evidence and the shared official Qoder favicon as brand-art evidence.
- Preferred vector asset: `https://img.alicdn.com/imgextra/i3/O1CN01KliT1u1jEq947NlKH_!!6000000004517-55-tps-180-180.svg`
  - Referenced by `<link rel="icon" type="image/svg+xml">` on `https://qoder.com/` and `https://qoder.com.cn/qoderwork` when fetched on 2026-08-13.
  - HTTP `200`; `Content-Type: image/svg+xml`; 39,257 bytes; intrinsic `width="180"`, `height="180"`, `viewBox="0 0 180 180"`; SHA-256 `2924a0fe240e0ca63895e345f65efbb6780b5c8e8b97a3ecf98c610f6e01fc41`.
  - Static-content inspection: no `<script>`, `<foreignObject>`, external href, embedded raster image, data URI, filter, or style element; paths/rectangles only. Primary colors are `#111113`, `#2ADB5C`, and white.
- Exact raster fallback: `https://img.alicdn.com/imgextra/i4/O1CN01OQC0dn1xLcdAaRALo_!!6000000006427-2-tps-180-180.png`
  - Referenced by the same official pages as both `rel="icon"` and `rel="apple-touch-icon"`, declared `sizes="180x180"`.
  - HTTP `200`; `Content-Type: image/png`; decoded PNG `180x180`, 8-bit indexed RGB (PNG color type 3), fully opaque; 1,258 bytes; SHA-256 `5ee51109a6c2195b9e5a2202123787aee2ac5dbae398c64952eda47272647eb7`.
- Do not use the nearby product-card image `https://img.alicdn.com/imgextra/i3/O1CN01fs0oL61vrZ3TMWnlP_!!6000000006226-2-tps-144-144.png` as the canonical source. Despite its `.png` suffix and the official page's `Qoder CN IDE` alt text, the response is `image/webp`, not a QoderWork-specific published brand pack.
- The official pages also expose horizontal wordmarks, for example `https://img.alicdn.com/imgextra/i2/O1CN01js79rH1mt5nkV0kEl_!!6000000005011-55-tps-640-180.svg`, but those are poor fits for the requested square Agent selector.

#### TRAE Work

Use the current official TRAE favicon as the square catalog icon. The CN and international official sites serve byte-identical art, which is useful cross-check evidence that it is the current shared TRAE mark rather than a third-party recreation.

- Product entries:
  - CN: `https://www.trae.cn/` (official page explicitly lists `TraeWork` and links `/work`).
  - International: `https://www.trae.ai/` (official page explicitly describes `TraeWork`).
  - Official product rename explanation: `https://www.trae.ai/blog/trae_work_0609` states that TRAE SOLO became TRAE Work and that TRAE Work is offered on desktop, web, and mobile.
  - Official channel confirmation: `https://github.com/Trae-AI/TRAE/issues/1524` names `https://www.trae.cn/` and `https://www.trae.ai/` as official sites.
- Preferred CN asset: `https://lf-cdn.trae.com.cn/obj/trae-com-cn/trae_website_prod_cn/favicon.png`
- Byte-identical international mirror: `https://lf-static.traecdn.us/obj/trae-ai-tx/trae_website/favicon.png`
  - Both are referenced by `<link rel="icon">` on their respective official homepages.
  - HTTP `200`; `Content-Type: image/png`; decoded PNG `48x48`, 8-bit RGBA container (PNG color type 6), fully opaque; 488 bytes; SHA-256 `49d523938a22af5a70dd79923725df38674823026e2f917e76337319969f4af4`; ETag `"98405406dcfc40174dc6115d6df4e2c7"`.
  - Dominant verified colors include `#0A0B0D` and `#32F08C`, matching TRAE's official visual-description page (`https://www.trae.ai/blog/product_new_look`) that calls its core color “Intelligent Green” and describes the pixelated robot mark.
- Higher-resolution official-origin corroboration: the verified official GitHub organization `https://github.com/Trae-AI` (`name: TRAE.AI`) exposes `https://avatars.githubusercontent.com/u/192691831?v=4`, a `95x95` PNG (640 bytes; SHA-256 `f7ebb118ecb95c8d31f6758d2507dd018fed6c996240106a79007f7e6b23e406`). It has the same dominant `#0A0B0D` / `#32F08C` palette, but the website favicon is the stronger product-site source and should be the canonical retrieval URL.
- The official repository `https://github.com/Trae-AI/TRAE` contains only `README.md` and issue templates, has no release/package assets, and GitHub reports no repository license. It is not a logo or source-code license grant.

### 2. Brand-use and licensing caveats

- No Qoder or TRAE press kit, brand guideline, trademark-use license, or downloadable-logo permission was found on the official sites/docs/repos inspected on 2026-08-13.
- Qoder's current official CN user agreement is `https://terms.alicdn.com/legal-agreement/terms/c_platform_service_agreement/20231023213402278/20231023213402278.html` (page reports update date 2026-05-20). Its IP section says the service's trademarks, service marks, URLs, service names, software, and related materials belong to the provider/affiliates; rights not expressly licensed are reserved and require written permission. Therefore the public favicon is official-source evidence, not a general redistribution or modification license.
- TRAE's official CN terms entry is `https://www.trae.cn/terms-of-service` (linked from `https://www.trae.cn/terms-of-service/cn`); the official site identifies the owner as 北京引力弹弓科技有限公司 and displays an all-rights-reserved footer. The official GitHub repo has no license. An official-community support reply at `https://forum.trae.cn/t/topic/19535` explicitly says it could not find an official brand/logo download rule and recommends contacting `feedback@mail.trae.ai` for confirmation. This community page is a risk signal, not a license.
- Practical boundary for this implementation: vendor marks may be bundled only as small, unmodified nominative identifiers for their corresponding catalog entries, paired with the exact product name and official link; do not imply affiliation/endorsement, recolor/redraw, use either mark as FyAgent's app/installer identity, or claim that public availability granted redistribution rights. Preserve source URL and retrieval metadata in the task evidence. If public distribution risk must be eliminated, use a neutral monogram/placeholder until written permission is obtained.
- Issue #25 below independently imposes the same conservative rule for packages: absent caching/redistribution authorization, FyAgent should direct the user to the official source rather than mirror vendor installers. The user-requested QoderWork/TRAE “jump to official site” path is aligned with that boundary.

### 3. Relevant FyAgent Issues (remote state on 2026-08-13)

All listed Issues were open when queried. These are requirements/evidence, not proof that code exists.

| ID | Status / priority | Direct impact on this task |
| --- | --- | --- |
| [#101](https://github.com/fy-agent/fyagent/issues/101) `PRD：首次目标选择、Agent 目录与既有配置接管` | OPEN, P0, DRI `python-rust`; updated 2026-08-13 | Current group-level decision source. Agent options must derive from one versioned catalog; unsupported/pending entries cannot expose full install/config flows; manual paths must identify the official destination and what FyAgent will not do. Its 2026-08-13 execution comment requires six pages to be reachable and each page to have a real backend action or explicit controlled degradation. |
| [#22](https://github.com/fy-agent/fyagent/issues/22) `[G1-01] 建立 Agent 目录事实合同并驱动可用选项` | OPEN, P0, assignee `python-rust`; updated 2026-08-13 | Catalog SSOT and action-state contract. A 2026-08-12 member research comment proposed QoderWork CN, TRAE Work, WorkBuddy, Codex, Claude Code and 悟空, but the Issue body still names a different first batch; the current user request resolves this task's five visible entries. The 2026-08-13 execution slice says first version may jump to official entry and must not cache/mirror third-party installers without permission. QoderWork/TRAE should therefore be honest `pending_verification` / official-link entries, not “supported” integrations. |
| [#25](https://github.com/fy-agent/fyagent/issues/25) `[G1-04] 展示官方来源、许可和镜像来路` | OPEN, P0; updated 2026-08-13 | Requires official domain/source/version/license links and forbids automatic install when source or license is unclear; no package cache/redistribution without authorization. Reinforces official-link-only QoderWork/TRAE scope. |
| [#34](https://github.com/fy-agent/fyagent/issues/34) `[G2-01] 首次进入先问目标，再按目录给出 Agent 与接入方式` | OPEN, P0, assignee `python-rust`; updated 2026-08-13 | Agent and connection choices must be generated from #22 capability data; unavailable methods must not create dead forms; assisted/manual paths must name the user action and official URL. Config-changing choices route through #41, while non-changing browsing/link jumps do not write. |
| [#41](https://github.com/fy-agent/fyagent/issues/41) `[G2-08] 让配置应用过程可见、可回读、可恢复` | OPEN, P0; updated 2026-08-12 | Defines the future apply-job model (backend snapshots/events, backup/write/readback/recovery) and forbids fake progress or active provider/model probes in the main apply flow. A fast V2 Models page should reuse existing real backend commands and clearly label any temporary bounded configuration path rather than inventing completion evidence. |
| [#43](https://github.com/fy-agent/fyagent/issues/43) `[G2-10] 为官方订阅连接建立逐厂商准入门槛` | OPEN, P1; updated 2026-08-12 | No subscription-token relay or captured login state without official authorization. QoderWork/TRAE should remain official-login/manual-assisted links unless a documented connector exists. |
| [#91](https://github.com/fy-agent/fyagent/issues/91) `[G6-11] 完成 v4 控制面原型评审后再冻结品牌与页面合同` | OPEN, P1; updated 2026-08-12 | Warns that generated six-page prototypes are concepts, not runtime evidence, and says Models/Skills/MCP/Prompts/Memory should consume real existing contracts while future catalog features stay clearly candidate. The current V2 shell spec is executable local authority for this implementation. |

Additional issue searches for `qoder`, `trae`, `Agent 目录`, `models v2`, `frontend v2`, and `icon/logo/branding` found no dedicated implementation PR/Issue that already supplies QoderWork/TRAE assets or a completed V2 Agents/Models implementation.

### 4. Relevant FyAgent PRs

| PR | State | Impact |
| --- | --- | --- |
| [#98](https://github.com/fy-agent/fyagent/pull/98) `Polish GitHub brand and community experience` | MERGED 2026-08-12, merge `e8e578fcfee346947546926fe406a11557f26970` | Added GitHub/social brand assets while explicitly keeping every existing application/installer/tray/About icon byte-identical. It does not implement this task, but proves brand art and application package identity are intentionally separate scopes. |
| [#99](https://github.com/fy-agent/fyagent/pull/99) `Restore the For You Agent identity` | MERGED 2026-08-12, merge `8c8ca4c2eea69889cbdf53d9c983218806e93a4e` | Locks `FyAgent = For You Agent`; reinforces that third-party vendor marks must never replace the FyAgent app identity. |
| [#11](https://github.com/fy-agent/fyagent/pull/11) `docs: 重构公开文档并完善 For You Agent 叙事` | MERGED 2026-08-11 into `dev/laiyongjie`, merge `eec8ccdde9fac6d8b1a3ceda469171b3eb70dcfd` | Introduced current public/manual structure, including WorkBuddy documentation. It changed no `src/`, `src-tauri/`, license, or release behavior, so it is context rather than a V2 implementation predecessor. |

No remote PR was found with `qoder` or `trae` in its searchable metadata; no open PR existed in the repository when `gh pr list --state all` was queried.

### 5. Pre-implementation baseline observed on 2026-08-13

- `src/v2/pages/agents/Page.tsx:1-3` — at baseline commit `cc8553f8`, the V2 Agents page was `return null`; there was no local QoderWork/TRAE implementation to reuse.
- `src/v2/pages/models/Page.tsx:1-3` — at baseline commit `cc8553f8`, the V2 Models page was `return null`.
- `src/v2/shared/config/navigation.ts:1-13` — typed six-route contract and exact order (`agents`, `models`, `skills`, `mcp`, `prompts`, `memory`).
- `src/v2/widgets/app-shell/Brand.tsx:1-18` — the V2 header already imports the project-owned transparent Y-shaped FyAgent mark from `src/v2/shared/assets/fyagent-y-mark-transparent-128.png`; this is distinct from the current Tauri package icon set.
- `src-tauri/tauri.conf.json:35-52` — desktop bundle uses `icons/32x32.png`, `icons/128x128.png`, `icons/128x128@2x.png`, `icons/icon.icns`, and `icons/icon.ico`.
- `src-tauri/tauri.windows.conf.json:21-35` — Windows NSIS package points `installerIcon` at `icons/icon.ico`.
- `docs/fyagent/development/windows/installer.md:117-119` — `icons/icon.ico` is the canonical Windows setup/uninstaller icon.
- `.trellis/spec/backend/application-brand-assets.md:1-114` — app-icon changes must start from one approved `1024x1024` RGBA transparent source, run the repository generator, update all bundle/About/tray consumers, preserve third-party artwork, and pass `mise run assets:icons:check` plus platform-specific evidence. This means “switch installer to the Y mark” is a full FyAgent application-brand asset change, not an `installerIcon` one-line edit.
- `.trellis/spec/frontend/v2-shell.md` (Navigation/content and layer-boundary sections) — current shell contract says Agents and Models were empty Phase-1 pages, pages import only from V2 shared, direct Tauri imports remain below `shared/platform/tauri`, and V2 must not import legacy `src/lib/**` directly. New business pages require a reviewed contract update or explicit task-scoped supersession plus focused tests.
- Remote/local text search found no QoderWork or TRAE brand asset in product code or asset directories before this task.

## Related specs

- `.trellis/spec/backend/application-brand-assets.md` — canonical app/installer icon generation and verification contract.
- `.trellis/spec/backend/application-identity.md` — FyAgent identity and canonical repository/source boundaries.
- `.trellis/spec/backend/windows-installer.md` — Windows setup icon and package validation.
- `.trellis/spec/frontend/v2-shell.md` — six-route V2 shell, page/import boundaries, test matrix, and the prior empty-page baseline.
- `.trellis/spec/frontend/component-guidelines.md` — accessible typed component conventions.
- `.trellis/spec/frontend/directory-structure.md` — legacy layout context; V2-specific placement is governed by `v2-shell.md`.

## Selected local assets

The implementation selected the exact reviewed vendor bytes without runtime
network loading or recoloring:

| Local path | SHA-256 | Bytes | Boundary |
| --- | --- | ---: | --- |
| `src/v2/shared/assets/agents/qoderwork.svg` | `2924a0fe240e0ca63895e345f65efbb6780b5c8e8b97a3ecf98c610f6e01fc41` | 39,257 | Exact passive official Qoder family SVG used only for `QoderWork CN`. |
| `src/v2/shared/assets/agents/trae-work.png` | `49d523938a22af5a70dd79923725df38674823026e2f917e76337319969f4af4` | 488 | Exact official TRAE 48x48 PNG used only for `TRAE Work`. |

The separate FyAgent application identity pipeline now renders the audited Y
geometry into canonical `assets/fyagent.png` (SHA-256
`9e2ceb57c5614a15e73c1812b2013b2b53b34ebbd9289e6c39d5c0f453f77a0f`,
56,130 bytes) and its generated consumers. These file identities and static
icon checks do not by themselves establish a fresh Windows bundle, PE-resource
inspection, native-window HIL, macOS HIL, or trademark approval; those remain
separate acceptance gates.

## Caveats / Not Found

- Neither vendor published an explicit logo/brand-use license in the official sources inspected. Official hosting proves provenance, not permission to redistribute or modify. The safest implementation is unmodified small nominative display with an official link and no endorsement claim; obtain written permission if public distribution policy requires it.
- No QoderWork-specific standalone icon distinct from the Qoder family mark was found. The official requested path uses the family favicon; use the family mark and label it precisely `QoderWork CN` instead of fabricating a sub-brand icon.
- TRAE's website publishes only a `48x48` raster favicon as a stable direct asset URL. It is adequate for a small Agent selector but should be resampled once at build time, not repeatedly at runtime. Do not trace/redraw it to SVG without vendor approval.
- Search-engine snapshots around QoderWork were ahead of/different from the 2026-08-13 raw HTML, which now presents a broader Qoder CN landing page. Preserve the exact source URLs and retrieval date and avoid claiming a stable vendor asset API.
- Issue comments are historical/research context and can conflict with later Issue bodies/PRD. For product decisions, #101 plus the current bodies of #22/#34/#41 override older comments; the user's explicit five-Agent scope governs this task.
- Remote Issues describe broader future catalog/apply contracts than can be safely implemented in a fast iteration. Honest official-link-only degradation for QoderWork/TRAE is supported; automatic installation, credential reuse, token relay, or claims of configuration support are not.
- The selected Y files and generator checks do not validate a fresh packaged
  setup or native visual result. Windows package-resource inspection and native
  visual acceptance remain required by the application-brand spec; macOS and
  legal approval remain explicitly unverified unless separately executed.
