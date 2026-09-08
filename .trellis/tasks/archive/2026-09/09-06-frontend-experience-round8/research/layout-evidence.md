# Round-eight layout evidence

## Baseline and method

Baseline: `5d42639249f5ba2bbe7201cc00a40274e6c3b1d1`, clean
`dev/laiyongjie`, no active task and no linked worktree before planning.
The user supplied a dark-theme Skills screenshot showing malformed assignment
rows after resize and poorly allocated detail space. The reproduction uses
the checked-in rich synthetic IPC fixture, never the user's installed Skills
or credentials. This is browser production evidence, not native WebView HIL.

`mise run build:renderer` passed; the unchanged production assets served via
the checked-in Vite preview configuration have 7 verified route chunks,
658,971 initial JS bytes and 45,123 initial CSS bytes. Chromium and WebKit each
visited Skills and MCP through widths 1564 → 1440 → 1232 → 1182 → 1180 → 900 →
1181 → 1564. Height varied from 991/900 to 700/600. Measurements wait for real
animation frames, not a mocked ResizeObserver or a forced remount.

Diagnostic scripts, numerical samples and screenshots are ignored under
`node_modules/.cache/fyagent-round8/`; execution logs are
`/tmp/fyagent-round8-{baseline-build,baseline-probe,candidates,isolated-tracks,design-study}.log`.
These are investigation artifacts, not product entry points or committed test
configuration. All candidate CSS was injected into a disposable browser page;
no product file has been modified for these experiments.

## R1: assignment rows retain excessive intrinsic grid height

Owners: `src/shared/ui/AssignmentPanel.tsx:75`,
`src/app/styles/features.css:262-285`, `src/shared/ui/split/SplitPanes.tsx:44-145`,
`src/pages/skills/Page.tsx:653-734`, `src/pages/mcp/Page.tsx:516-600`.

At first wide render, both engines produce seven 31px assignment rows. After
crossing the 1181px two/three-pane boundary and returning to wide:

| Engine / routes         | Seven row heights (px)                                 | Assignment list height |
| ----------------------- | ------------------------------------------------------ | ---------------------- |
| Chromium / Skills + MCP | 31, 31, 31, 31, 31, 31, 31                             | 312.89px               |
| WebKit / Skills + MCP   | 215.17, 215.17, 194.38, 194.38, 131.98, 235.97, 194.38 | 1477.31px              |

The label itself remains about 20.8px high and each switch remains 36×20px.
The excess height belongs to the shared grid's implicit auto rows, not an
oversized icon or application data. Enlarging again does not restore normal
row heights. No horizontal overflow or JavaScript error is required for this
failure: a test that checks only `scrollWidth <= clientWidth` passes incorrectly.

One-variable browser experiments, repeated for both routes and engines:

| Candidate                                                          | WebKit final row result     |
| ------------------------------------------------------------------ | --------------------------- |
| Remove row Flex wrapping                                           | Still 104–208px; not a fix. |
| Change row itself to two-column Grid                               | Still 104–208px; not a fix. |
| Give outer assignment grid an explicit `minmax(0,1fr)` column only | Still 132–236px.            |
| Give outer assignment grid `grid-auto-rows:max-content` only       | All seven rows 31px.        |

Use the explicit intrinsic-row rule as the minimal correction, with dynamic
cross-boundary tests. Do not label this as a proven upstream WebKit issue or
replace the existing panel library: the browser difference is reproduced, but
an upstream engine defect has not been independently established.

## R1: bulk rows wrap independently rather than as one responsive group

At a 220px assignment pane, its 190px inner width cannot fit a 91.34px label,
14px gap and 115.63px action pair. Current Flex wrapping creates six 64.8px
rows but one 36px Codex row; the buttons alternately appear beneath or beside
their labels. Removing wrap merely splits the action pair and increases some
rows to 80px. These are deterministic layout faults in both engines.

Skills and MCP duplicate the same bulk markup. Use a shared presentation-only
bulk section with explicit label/action slots. All rows change template based
on the section's content width, not the length of each application name. Keep
the action pair together, preserve the seven-target order and existing native
mutation/source callbacks. An unrelated unmanaged-import checkbox also uses
`.fy-feature-assignment`; do not accidentally apply bulk-specific rules to it.

## R2: the wrong pane absorbs the remaining width

The existing adapter preserves pixel widths for every pane except the last.
For list/detail/assignment this fixes the actual detail at about 400px while
the action rail absorbs every extra pixel. At 1564px viewport the measured
panel widths are 268 / 400 / 558px. After narrow/wide re-entry they become
about 220 / 400 / 603px. This contributes directly to the empty right rail
and the crowded middle, despite the unused `--fy-split-pane-*` values in
Skills/MCP page CSS (`page.css:66-69` and `page.css:46-49`). Those variables
are declarations without a sizing consumer, not an active source of truth.

Using the real second separator to make the action rail 280px gives
268 / 678 / 280px in Chromium and 268 / 677.98 / 280px in WebKit. However a
subsequent narrow/wide cycle restores the oversized final rail under the
current adapter. A one-off drag or CSS patch is therefore insufficient:
the detail pane must be the library's flexible pane for this layout role.

The adopted `react-resizable-panels@4.12.3` already supports pixel min/default/
max sizes and `groupResizeBehavior`; reuse these through SplitPanes. Do not
override the library Group's Flex layout or install another resize controller.

## R2: inner cards follow viewport width, not their own available space

`src/app/styles/features.css:598-603` forces two columns above 1180px viewport,
even when the detail's content width is only 370px. Result: 178.77px cards,
including padding and border, with much less room left for metadata.

The browser-only candidate uses a 256px minimum info-card width, maximum two
columns, and lets a remaining column fill the available content width. At the
adjusted 678px detail it produces 318 / 318px cards; at 423.5px content width
it produces one 423.5px card. A semantic definition-list label cap of 30%,
limited to 8em, leaves the remaining space after the 12px gap to the value.
Both engines have matching results. These numbers are a project candidate
derived from these labels and controls, not an externally mandated ideal ratio.

The candidate screenshot still demonstrates that equal-height short cards
can have artificial blank interiors. Use natural content height, preserve
meaningful whitespace, and verify short versus long content; do not enlarge
font sizes or invent data to fill a card.

`CopyablePath(revealValue=false)` in installed Skills/MCP is intentional and
documented (`skills.md`, Paths section). A hidden path is not missing content
to reveal for density. Its copy button must remain readable/reachable without
forcing an empty field to look filled.

## Shared-consumer disposition

| Family                                                    | Disposition                                                                                                                                               |
| --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AssignmentPanel switch mode, Skills/MCP full allocation   | Shared row sizing, shared bulk slot composition, all layout states tested.                                                                                |
| AssignmentPanel radio mode and unmanaged-import selection | Preserve semantics and local selected state; regression only unless the same sizing defect is proven.                                                     |
| Skills/MCP installed list-detail-assignment split         | Choose middle flexible pane; shared role dimensions; remove the two ineffective page-local sizing declarations.                                           |
| Skills/MCP info grids and definitions                     | Container-sized card layout, natural heights, shared metadata roles.                                                                                      |
| Auth account/connection definitions                       | Same definition family but has a local stacked rule at 500px; reconcile shared rule only after regression, do not override the established stack blindly. |
| Models/Auth form grids                                    | Already use container queries and auto-fit (230px); retain form-specific role, no blanket 70% width.                                                      |
| Prompts/Memory/CatalogMasterDetail                        | Existing two-pane roles keep their flexible detail; migration must preserve custom min/max, draft identity, scrolling and separator behavior.             |
| Long prose, code/paths and credential fields              | Keep their own reading/overflow/disclosure contracts; not a universal density target.                                                                     |

## Review passes

1. Symptom versus cause: distinguish grid-row inflation, mixed bulk templates,
   outer pane allocation, and inner viewport-only grid admission.
2. Reuse/boundary: no added dependency, DOM copying, renderer generation, native
   writes, or alternative layout engine. Reuse CSS intrinsic sizing, existing
   Button/AssignmentPanel and the installed panel adapter.
3. Acceptance: require A→B→A resizing and known-content geometry, not just a
   fresh screenshot at each width or generic no-overflow assertions.
