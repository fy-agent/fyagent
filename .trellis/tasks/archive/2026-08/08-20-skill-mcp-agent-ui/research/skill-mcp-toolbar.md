# Research: Skill and MCP page headers (same-row toolbar)

- **Query**: Exact component files, layout CSS, how MCP does same-row vs Skill; which buttons exist on Installed vs Discover; shared chrome; minimal change plan so the two top-right buttons sit on the same row as 已安装/发现.
- **Scope**: internal
- **Date**: 2026-08-20

## Findings

### Files Found

| File Path | Description |
|---|---|
| `src/v2/pages/skills/Page.tsx` | Skills page: page-level header + Installed/Discover `FeatureTabs` |
| `src/v2/pages/skills/page.css` | Skills-only tab/toolbar overrides |
| `src/v2/pages/mcp/Page.tsx` | MCP page: page-level header + Installed/Discover `FeatureTabs` |
| `src/v2/pages/mcp/page.css` | MCP card meta only; no header/tab geometry |
| `src/v2/pages/mcp/Discovery.tsx` | MCP Discover inner search/filter toolbar (not the view tabs) |
| `src/v2/shared/ui/FeatureTabs.tsx` | Shared tablist (`fy-feature-tabs`) |
| `src/v2/shared/ui/primitives.tsx` | Shared `Button` (`fy-control-button`) |
| `src/v2/app/styles/features.css` | Shared `.fy-feature-header` / `.fy-feature-actions` / `.fy-feature-tabs` / `.fy-feature-toolbar` |
| `tests/v2/features/featurePages.test.tsx` | Skills/MCP interaction tests (checks 检查更新 / 更多 / 导入现有) |
| `tests/v2/pages/skills/page.styles.test.ts` | Discovery page-scroll CSS freeze only |
| `.trellis/spec/frontend/v2-skills-mcp.md` | Skills/MCP feature contract |
| `.trellis/spec/frontend/reuse.md` | Shared chrome ownership |

There is no dedicated shared header component. Both pages inline the same class names.

### Two chrome layers (do not mix)

Page-level view switcher (this task):

1. Optional/always `header.fy-feature-header` with `div.fy-feature-actions` (the two top-right buttons).
2. Sibling `FeatureTabs` with labels **已安装** / **发现**.

Inner workspace toolbar (not the view switcher):

- Skills Installed: `fy-feature-toolbar` with `FeatureSearch` only (`Page.tsx` ~600–607).
- Skills Discover: `fy-feature-toolbar` with `FeatureSearch` + category `FeatureTabs` (`aria-label="分类筛选"`, `Page.tsx` ~857–877).
- MCP Installed: `fy-feature-toolbar` with `FeatureSearch` only (`Page.tsx` ~465–472).
- MCP Discover: `fy-feature-toolbar` with `FeatureSearch` + `<select aria-label="分类筛选">` (`Discovery.tsx` ~140–161).

`.fy-skills-page .fy-feature-toolbar > .fy-feature-tabs { flex: 1 1 100%; }` forces **category** tabs onto their own wrapped row under search. That is not the Installed/Discover bar.

### Page-level DOM today

Skills (`src/v2/pages/skills/Page.tsx` 481–542):

```tsx
<div className={`fy-feature-page fy-split-page fy-skills-page${tab === "discovery" ? " fy-skills-page-discovery" : ""}`}>
  {tab === "installed" ? (
    <header className="fy-feature-header">
      <div className="fy-feature-actions">
        <Button>检查更新</Button>
        {/* 更新全部 · N  only when updates.length > 0 */}
        <div className="fy-feature-menu">
          <Button>更多</Button>
          {/* popover: 导入本地 Skill / 从 ZIP 安装 / 备份恢复 / Skill 设置 */}
        </div>
      </div>
    </header>
  ) : null}
  <FeatureTabs
    id="skills-view-tabs"
    label="Skills 视图"
    options={[
      { id: "installed", label: "已安装" },
      { id: "discovery", label: "发现" },
    ]}
  />
```

MCP (`src/v2/pages/mcp/Page.tsx` 356–385):

```tsx
<div className="fy-feature-page fy-split-page fy-mcp-page">
  <header className="fy-feature-header">
    <div className="fy-feature-actions">
      <Button>导入现有</Button>
      <Button className="fy-control-button-primary">添加 MCP</Button>
    </div>
  </header>
  <FeatureTabs
    id="mcp-view-tabs"
    label="MCP 视图"
    options={[
      { id: "installed", label: "已安装" },
      { id: "discovery", label: "发现" },
    ]}
  />
```

Neither page puts `FeatureTabs` inside `fy-feature-header`. Both are column children of `.fy-feature-page` (`display: flex; flex-direction: column`). Screenshot observation matches this: Skill 检查更新 + 更多 sit **above** the tab bar; MCP 导入现有 + 添加 MCP sit **vertically higher** than the tab switcher.

### Which buttons exist on Installed vs Discover

| Page | Control | Installed | Discover |
|---|---|---|---|
| Skill | **检查更新** | yes (header) | **no** (header unmounted) |
| Skill | **更新全部 · N** | yes, only if `updates.length > 0` | **no** |
| Skill | **更多** → 导入本地 Skill / 从 ZIP 安装 / 备份恢复 / Skill 设置 | yes | **no** |
| Skill | **已安装** / **发现** tabs | yes | yes |
| MCP | **导入现有** | yes | **yes** (same header) |
| MCP | **添加 MCP** | yes | **yes** (same header) |
| MCP | **已安装** / **发现** tabs | yes | yes |

MCP is the closer pattern for *persistence*: the two actions stay mounted when `tab === "discovery"`. It is **not** same-row yet.

Empty-state duplicates (not the header):

- Skill empty Installed: **浏览发现** (`Page.tsx` ~590–595).
- MCP empty Installed: **导入现有**, **添加 MCP**, **浏览发现** (`Page.tsx` ~451–459).

### Layout CSS: why Skill tabs take a full exclusive row

Shared (`src/v2/app/styles/features.css`):

```css
.fy-feature-page {
  display: flex;
  flex-direction: column;
  /* align-items defaults to stretch */
}
.fy-feature-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 16px;
}
.fy-feature-header > .fy-feature-actions {
  margin-left: auto; /* right cluster when a left sibling exists */
}
.fy-feature-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 14px;
  width: max-content; /* MCP keeps this */
}
.fy-feature-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 9px;
  margin-bottom: 14px;
}
.fy-feature-toolbar > .fy-feature-tabs {
  flex: 0 1 auto;
  margin-bottom: 0;
}
```

MCP has **no** page CSS for header/tabs (`src/v2/pages/mcp/page.css` is card-only). View tabs keep `width: max-content`, so in the column flex they shrink-wrap instead of painting a full-width exclusive track. Buttons still occupy a **separate row above** because `header` has no left child—only `fy-feature-actions` with `margin-left: auto`.

Skill overrides (`src/v2/pages/skills/page.css` 1–13):

```css
.fy-skills-page .fy-feature-tabs {
  flex-wrap: wrap;
  width: auto;
  max-width: 100%;
}
.fy-skills-page .fy-feature-toolbar {
  align-items: flex-start;
}
.fy-skills-page .fy-feature-toolbar > .fy-feature-tabs {
  flex: 1 1 100%;
}
```

`.fy-skills-page .fy-feature-tabs` applies to **every** tablist on the page, including `skills-view-tabs`. `width: auto` plus column `align-items: stretch` makes **已安装/发现** span the full page width (the exclusive row in screenshots). The same rule also lets Skill 市场 category tabs wrap.

`fy-feature-toolbar` already knows how to put tabs and buttons on one wrap row (`flex: 0 1 auto` for tabs, `flex: 0 0 auto` for `.fy-control-button`). Page-level chrome does not use that wrapper.

### Shared chrome (what already exists)

- Classes: `fy-feature-header`, `fy-feature-actions`, `fy-feature-tabs`, `fy-feature-menu` / `fy-feature-menu-popover`.
- Components: `FeatureTabs`, `Button`.
- Contract: Skills and MCP share `FeatureTabs` for installed/discovery tracks (`v2-skills-mcp.md` §3 Presentation; `reuse.md`).
- Spec does **not** currently require header actions to share a row with those tabs. Presentation still requires Skills/MCP to own only `.fy-skills-page` / `.fy-mcp-page` wrappers and consume `--fy-*` tokens.
- `reuse.md`: chrome both pages will need goes in `src/v2/shared/ui` on the first commit. Both pages already share the CSS classes; they do not share a React wrapper.

Prompts also uses `header.fy-feature-header` + `fy-feature-actions` (`src/v2/pages/prompts/Page.tsx` ~552–568) with no Installed/Discover tabs. Not a same-row reference.

### Tests that touch this chrome

- `tests/v2/features/featurePages.test.tsx`: clicks **检查更新** and **更多** while on Installed; Discover tests query 发现 / 分类筛选 / 管理仓库 absence, not header persistence.
- `tests/v2/pages/skills/page.styles.test.ts`: discovery scroll (`overflow: auto` on page, `visible` on inner scroll). Unrelated to header row.

### Minimal change plan

Goal state: **已安装/发现** left, the two page actions right, one row, on both tabs.

Smallest alignment with existing shared CSS (no new component required):

1. **JSX (both pages)** — Move the view `FeatureTabs` *inside* `header.fy-feature-header` as the first child; keep `div.fy-feature-actions` as the second child. `.fy-feature-header` is already a wrapping flex row; `.fy-feature-header > .fy-feature-actions { margin-left: auto }` already right-aligns the cluster once a left sibling exists.
2. **JSX (Skills only)** — Remove `{tab === "installed" ? (…header…) : null}` so the same two buttons (**检查更新**, **更多**) stay mounted on Discover. Keep **更新全部 · N** gated on `updates.length > 0` (third button in the same cluster, Installed-or-Discover).
3. **Shared CSS (`features.css`)** — One rule so nested view tabs do not add a second bottom margin:

   ```css
   .fy-feature-header > .fy-feature-tabs {
     margin-bottom: 0;
   }
   ```

4. **Skills page CSS** — Keep `flex-wrap` / `width: auto` / `flex: 1 1 100%` on **toolbar** category tabs only (already `.fy-feature-toolbar > .fy-feature-tabs` for the 100% wrap). Stop applying `width: auto` to *all* `.fy-skills-page .fy-feature-tabs`, or the view tabs would stretch inside the header’s leftover space. Leave discovery page-scroll rules unchanged.
5. **Do not** put page actions into the inner `fy-feature-toolbar` (that row is search/categories and scrolls with discovery on Skills).
6. **Tests** — After the change, Discover should still expose **检查更新** and **更多**; MCP Discover should still expose **导入现有** and **添加 MCP**; both tablists stay `aria-label` **Skills 视图** / **MCP 视图**.

Handlers already work off the current tab: Skill `checkUpdates` / ZIP / import / settings do not require Installed; MCP `importExisting` / `setEditing("new")` already run from Discover.

## Caveats / Not Found

- No screenshot assets in this task dir; layout claims follow current JSX/CSS plus the query’s screenshot notes.
- No shared `FeaturePageHeader` / `ViewTabsHeader` module exists.
- Spec `v2-skills-mcp.md` does not freeze header/tab same-row geometry; changing Skills `page.css` tab `width` is not covered by `page.styles.test.ts`.
- Skill **更多** popover is `position: absolute; right: 0` relative to `.fy-feature-menu`. Same-row layout keeps that menu on the right; no extra CSS found that would break it.
- If the header wraps at narrow widths (`flex-wrap: wrap`), actions can drop under the tabs. That is the existing header wrap behavior, not a second exclusive tab row.
