# Bounded layout, native scrolling

The available-height chain is shell -> persistent route -> feature page ->
active tab -> workspace -> split pane -> scrollable content. Every flex boundary
must pass a bounded slot with min-width/min-height:0; content that intentionally
flows must instead belong to an explicitly scrollable owner.

Give workspace-bearing FeatureTabPanel an explicit layout role rather than
depending on the child being the workspace itself. Do not make every semantic
tab a scrolling/flex box: nested auth/model tabs and compact headers have
different responsibilities. Resolve the page-level direct-child nonshrinking
rule and Skills discovery override against this role, including hidden panels.

Reuse native overflow and the installed SplitPanes/react-resizable-panels
adapter. No scroll listeners that cancel wheel input, no new scrollbar engine,
no magic viewport height calculations duplicated per page. Full-height panes
own long list/detail scrolling; flow/error/header content has a reachable
bounded owner rather than being silently clipped when short windows fill up.

Preserve existing stable editor node lifetime, semantic tabs, keep-alive query
ownership, focus and two themes. A breakpoint must not remount a draft or
reintroduce two different data sources. Retain missing-ResizeObserver fallback.

Use existing browser fixtures extended with deterministic 40+ row/long-text
data. Start physical wheel at the rendered list/reading surface and assert
scroll offset changed AND the last item/action is in the visible clipped area.
Keyboard focuses real scroll/interactive owners; scrolling one pane must not
move unrelated pane headers. Test after opening/closing a dialog to detect a
leftover modal scroll lock. No extra package selected: the defect is sizing.

Rollback is a shared-layout child revert with its styles/tests; do not revert
the previous-round split library or silently weaken min-size constraints.
