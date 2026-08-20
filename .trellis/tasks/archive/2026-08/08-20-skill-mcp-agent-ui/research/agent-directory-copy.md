# Research: Agent directory copy (empty details except Codex)

- **Query**: Catalog agents; which detail pages have content; where copy lives (constants vs markdown vs i18n); which agents need intros. Do not invent copy; list official sources to fetch.
- **Scope**: mixed (internal catalog + official URLs to fetch later)
- **Date**: 2026-08-20

## Findings

### Files Found

| File Path | Description |
|---|---|
| `src/v2/pages/agents/Page.tsx` | Agent directory UI (`AgentDetail`) |
| `src/v2/pages/agents/Page.css` | Detail identity / Codex installer / leftover unused section styles |
| `src/v2/shared/ui/catalog/CatalogMasterDetail.tsx` | Shared master/detail chassis |
| `src/v2/shared/ui/catalog/CatalogOfficialLinks.tsx` | Official HTTPS buttons; label rewrite for non-`官方` labels |
| `src/v2/shared/codex-desktop/CodexDesktopInstallerPanel.tsx` | Codex-only substantial body copy |
| `src/v2/shared/features/directory.ts` | Display names / IDs (no intros) |
| `src/v2/shared/features/types.ts` | `AgentCatalogEntry.description` on the wire |
| `src-tauri/src/commands/agent_catalog.rs` | Catalog v4: names, short `description`, official links, capabilities |
| `src/v2/shared/assets/agents/` | Brand icons only |
| `tests/v2/pages/agents/Page.test.tsx` | Locks omitted catalog description / usage notes / panels |
| `.trellis/spec/frontend/v2-agent-models.md` | Agent directory presentation contract |
| `src/i18n/locales/*.json` | Leftover V1 strings; V2 must not import |
| `docs/user-manual/zh/1-getting-started/1.1-introduction.md` | User-manual one-liners (not wired to V2) |

No markdown, constants, or i18n table of product intros is rendered on the V2 Agent directory.

### Catalog (order is the UI order)

From `AGENT_CATALOG` in `src-tauri/src/commands/agent_catalog.rs` (contract version 4, `reviewed_at` 2026-08-20) and `PRODUCT_DIRECTORY` in `src/v2/shared/features/directory.ts`:

| `#` | `id` | `displayName` | `variantId` |
|---|---|---|---|
| 1 | `qoderwork` | QoderWork CN | `qoderwork-cn` |
| 2 | `trae-work` | TRAE Work CN | `trae-work-cn` |
| 3 | `workbuddy` | WorkBuddy | `workbuddy` |
| 4 | `grokbuild` | Grok Build | `grokbuild` |
| 5 | `codex` | Codex | `codex` |
| 6 | `claude-code` | Claude Code | `claude-code` |
| 7 | `opencode` | OpenCode | `opencode` |

### What each detail page actually renders

`AgentDetail` (`src/v2/pages/agents/Page.tsx` 43–102) always paints:

- Brand frame + `h2` `entry.displayName` inside `.fy-agent-identity-copy` (no paragraph under the title).
- `CatalogOfficialLinks` when `officialLinks.length > 0`.
- Optional Codex installer.
- Optional **支持的功能** section: only `mode === "direct"` jumps to `/models?target=`, `/skills`, `/mcp`. No Hooks jump even when Hooks is `direct`.

Using **native** catalog modes (not the looser Page.test fixture):

| Agent | Official buttons | Extra body | 支持的功能 |
|---|---|---|---|
| QoderWork CN | 打开 QoderWork 官方页面 | none | 打开 Skills, 打开 MCP (no 配置模型; `models.write` unsupported) |
| TRAE Work CN | 打开 TRAE Work CN 官方页面 | none | 打开 Skills, 打开 MCP (`models.write` assisted → no jump) |
| WorkBuddy | 打开 WorkBuddy 官方页面 | none | 配置模型, 打开 Skills, 打开 MCP |
| Grok Build | 打开 Grok Build 官方页面 | none | 配置模型, 打开 Skills, 打开 MCP |
| Codex | none (`officialLinks: []`) | **Codex Desktop 安装器** | 配置模型, 打开 Skills, 打开 MCP |
| Claude Code | 打开 Claude Code CLI 官网, 打开 Claude Desktop 官网 | none | 配置模型, 打开 Skills, 打开 MCP |
| OpenCode | 打开 OpenCode 官方页面, 打开 OpenCode CLI 官网 | none | 配置模型, 打开 Skills, 打开 MCP |

Codex installer copy lives in `CodexDesktopInstallerPanel.tsx` as hardcoded Chinese (`stateLabels`, heading **Codex Desktop**, body “在 FyAgent 中安装、更新或启动桌面应用。”, version labels, action labels). That is the only Agent detail with a multi-sentence body.

QoderWork CN is the sparsest non-Codex page: title + one official button + two jumps (Skills/MCP). TRAE is the same shape.

`.fy-agent-identity-copy` is a CSS grid with `gap: 7px` and only an `h2` child—empty slot under the title. `Page.css` still contains unused rules for observation grids, unsupported lists, Hooks editor, MCP validation (`fy-agent-observation`, `fy-agent-unsupported`, `fy-agent-hooks-*`, `fy-agent-mcp-*`). Those nodes are not mounted.

### Where copy lives today

| Source | What it holds | Used on Agent directory? |
|---|---|---|
| Rust `AgentCatalogEntry.description` | One-line **支持 / 不支持** capability matrix (see quotes below) | **No.** Spec forbids rendering catalog `description`. Test: `qoderDetail).not.toHaveTextContent("的目录说明")`. |
| Rust `officialLinks[].label` + URL | Button labels and HTTPS targets | **Yes**, via `CatalogOfficialLinks` / `officialLinkActionLabel` |
| `PRODUCT_DIRECTORY[].displayName` | Names only | List + `h2` |
| `src/v2/pages/agents/Page.tsx` literals | **支持的功能**, **配置模型**, **打开 Skills**, **打开 MCP**, empty/error titles | Yes |
| `CodexDesktopInstallerPanel.tsx` literals | Installer UX copy | Codex only |
| Markdown under `src/v2/pages/agents/` | — | **Not found** |
| `src/i18n/locales/{zh,en,ja,zh-TW}.json` | Leftover V1 (WorkBuddy model panel, Claude/OpenCode settings, etc.) | **Forbidden** for V2 (`v2-skills-mcp.md` / `reuse.md` / frontend index: V2 hardcodes Chinese, must not import `src/i18n`) |
| `docs/user-manual/**` | Manual one-liners for V1 app set (Claude Code, Codex, OpenCode, …). No QoderWork CN / TRAE Work CN / WorkBuddy / Grok Build rows in `1.1-introduction.md` | **Not imported** by V2 |
| Models page | Qoder/TRAE **guidance** (`QoderModelsPanel.tsx`, `TraeModelsPanel.tsx`) | Models route only, not Agent directory |

Rust catalog `description` strings (capability matrix, **not** product intros; must not be treated as the new intro copy):

- QoderWork CN: `支持 Skills 同步、Hooks 配置与 MCP 直接分配；不支持第三方模型配置。本机识别和启动暂无法确认。`
- TRAE Work CN: `支持 Skills 同步与 MCP 直接分配；自定义模型需在 TRAE Work CN 中添加；不支持 Hooks。本机识别和启动暂无法确认。`
- WorkBuddy: `支持 Skills 同步、模型配置与 MCP 直接分配；不支持 Hooks。本机识别和启动暂无法确认。`
- Grok Build: `支持 Skills 同步、模型配置与 MCP 直接分配。本机识别和启动暂无法确认。`
- Codex: `支持桌面安装、Skills、模型配置与 MCP；不支持 Hooks。`
- Claude Code: `支持 Skills、模型配置与 MCP；不支持 Hooks。本机识别和启动暂无法确认。`
- OpenCode: `支持 Skills、模型配置与 MCP；不支持 Hooks。本机识别和启动暂无法确认。`

Spec (`v2-agent-models.md` §1 / §3): catalog descriptions use 支持/不支持 wording and must not contain `可在 FyAgent` / `可通过 FyAgent`. Agent details omit the capability-item grid, catalog `description`, application status, configuration overviews, unsupported lists, support counts, **usage notes**, Hooks editors, and MCP validation panels.

Existing tests also forbid **使用说明**, **应用状态**, **配置概览**, **不适用的功能**, **项支持**, Hooks/MCP panels (`tests/v2/pages/agents/Page.test.tsx` 386–432).

Shared paragraph class already used by Skills/MCP details: `.fy-feature-intro` in `src/v2/app/styles/features.css` (16px, line-height 1.7). Agent directory does not use it.

### Agents that need intros

Task statement: directory pages **except Codex** feel empty.

Need substantial intros (no copy in-repo today that the page may render):

1. QoderWork CN
2. TRAE Work CN
3. WorkBuddy
4. Grok Build
5. Claude Code
6. OpenCode

Codex already has installer heading + status + versions. This research does not draft a Codex product intro.

Do **not** reuse Rust `description` as the intro: it is a capability matrix, and current spec + tests forbid showing it.

### Official sources to fetch (do not invent copy)

Fetch vendor-owned pages. Catalog URLs are the V2-authoritative HTTPS set (`agent_catalog.rs` 78–125). Secondary docs are listed so implementers can pull product definition, not FyAgent capability text.

| Agent | Catalog URL(s) already in Rust | Additional official docs to fetch |
|---|---|---|
| QoderWork CN | https://qoder.com.cn/qoderwork | Same host product/docs pages linked from that landing (QoderWork CN, not generic Qoder IDE). |
| TRAE Work CN | https://www.trae.cn/sem-work | TRAE CN Work/product pages linked from that landing. Spec product URL is exactly this value. |
| WorkBuddy | https://www.workbuddy.cn/ | Same site product/help pages. |
| Grok Build | https://x.ai/grok | Spec: product URL is exactly `https://x.ai/grok`. Also xAI docs for Grok if the landing distinguishes Grok (chat) vs Grok Build (coding agent). |
| Codex | *(empty list by contract)* | OpenAI Codex / Codex Desktop official docs. Do not add a catalog product button. Installer copy stays in `CodexDesktopInstallerPanel`. |
| Claude Code | https://docs.anthropic.com/en/docs/claude-code/getting-started ; https://claude.com/download | Anthropic Claude Code overview/docs; Claude Desktop download/about. Renderer labels become `打开 {catalog label} 官网` when the Rust label lacks `官方`. |
| OpenCode | https://opencode.ai ; https://opencode.ai/docs/cli | OpenCode product + CLI docs. |

Fetch rules implied by existing contracts:

- Third-party marks identify their own products only; presence is not vendor endorsement (`v2-agent-models.md` §3).
- Do not write copy that claims FyAgent can detect/launch unverified agents (`本机识别和启动暂无法确认` is catalog matrix wording, not marketing).
- Qoder Models already states 官方不支持第三方模型配置; TRAE Models states custom models must be added in TRAE Work CN. Agent intros should not contradict those Models-page facts.
- V2 user-visible strings are hardcoded Chinese in TSX, not i18n keys.

User-manual table (`docs/user-manual/zh/1-getting-started/1.1-introduction.md` 49–57) is **not** an official vendor source and covers the leftover V1 app set (includes Gemini CLI / OpenClaw / Hermes; omits QoderWork CN, TRAE Work CN, WorkBuddy, Grok Build). Do not treat it as Agent directory copy.

## Caveats / Not Found

- This file does **not** contain drafted intro paragraphs.
- Official pages were listed, not fetched, in this pass; wording on those sites can change after `reviewed_at` 2026-08-20.
- Page.test fixture marks almost every capability `direct` (except Codex `product.open` and TRAE models), so test details show more jumps than production Qoder/TRAE.
- Adding a visible intro is a new Agent-directory surface. Current spec names “usage notes” and catalog `description` as omitted; implementers need a spec decision on a third copy field vs. page-local Chinese blocks vs. extending the catalog payload.
- No Agent intro markdown, JSON, or i18n namespace exists under `src/v2`.
