# Catalog UI evidence

## Confirmed implementation

- The production renderer is the isolated V2 shell. The Agent page is
  `src/v2/pages/agents/Page.tsx`; its two-column styles live next to it in
  `Page.css`.
- The list and detail `<img>` elements both resolve through the single local
  asset map `src/v2/shared/assets/agents/index.ts` (`Page.tsx:4, 253-258,
  391-396`).
- The visible pale/white tiles are not baked into the shared JSX. CSS adds
  `rgba(238, 249, 255, 0.86)` to list icons and
  `rgba(238, 249, 255, 0.9)` plus a border/shadow to detail icons
  (`Page.css:68-80, 118-126`). Removing the presentation tile is sufficient for
  assets that already carry their own desired background; the TRAE 48px sizing
  exception must remain (`Page.css:128-132`).
- `.fy-agent-layout` is a two-column CSS Grid with no cross-axis override
  (`Page.css:1-6`). Grid's default `align-items: stretch` makes the catalog
  panel match the taller detail panel. A start-alignment rule at this owning
  layout boundary is the narrow fix; page-level independent scrolling should
  be added only if the maintained viewport tests show inaccessible overflow.
- The Agent and Models routes deliberately share one five-entry native catalog:
  QoderWork, TRAE Work, WorkBuddy, Codex, Claude Code
  (`src/v2/shared/features/types.ts:139-183` and
  `.trellis/spec/frontend/v2-agent-models.md`). Models maps those candidates to
  bounded configuration/guidance flows; Skills/MCP assignments are a different
  six-app domain.
- Skills and MCP share `AssignmentPanel`, which currently renders only text and
  a switch (`src/v2/shared/ui/AssignmentPanel.tsx:1-31`). Their exact assignment
  IDs are Claude, Codex, Gemini, Grok Build, OpenCode, and Hermes
  (`src/v2/shared/features/types.ts:1-22`). These IDs do not match the five-entry
  Agent catalog, so assignment icons need their own typed local asset map rather
  than unsafe casting through `AgentCatalogId`.
- Existing local sources already cover the six assignment brands across the V2
  Agent assets and legacy extracted assets. V2 may copy/review those bytes into
  its own shared asset boundary, but the V2 shell contract forbids importing
  legacy modules directly.

## Required focused evidence

- Unit tests for the typed six-app icon map, fallback behavior, decorative
  semantics, and AssignmentPanel labels/switches.
- Agent component/CSS contract tests for transparent presentation and
  cross-axis start alignment.
- V2 browser geometry at 900x600, 1152x640, 1232x700, and 1440x900 covering the
  Agent layout plus populated Skills/MCP assignment panels.

## Product question

The repository can prove the current five-candidate versus six-assignment
domains, but it cannot infer the missing end of the user's sentence “Agent
目录和模型目录，实际上对应着…”. Any requested semantic remapping remains a user
decision.
