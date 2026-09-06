# Current behavior and coverage inventory

Baseline: `4f8973ef1b8acfb580e2e6362920fcee4e455098`, clean checkout before planning.
Measurements use the actual `dist` build, Chromium and WebKit, 1232x700,
repository synthetic IPC fixtures only. No real account mutation, credential,
installer or external service is exercised. Code was not changed by probes.
Ignored evidence: `node_modules/.cache/fyagent-round7/evidence/`.

## A. Skills scrolling — reproduced in both engines

`src/pages/skills/Page.tsx:587-740` wraps its workspace in FeatureTabPanel.
`src/shared/ui/FeatureTabs.tsx:122` adds a real block without a bounded flex slot.
`src/app/styles/features.css:23` gives every non-workspace direct child
`flex:0 0 auto`; the new panel qualifies. Ancestors in `shell.css:350-404`
and `split/split.css` clip overflow. The available height is lost at the tab
wrapper, so pane overflow never exists inside the pane's own box.

With 40 synthetic installed Skills, both engines report:

| Measurement                                   |    Baseline | Browser-only bounded-panel CSS probe |
| --------------------------------------------- | ----------: | -----------------------------------: |
| Page height                                   |       586px |                                586px |
| Active tab height                             |   3068.77px |                              448.5px |
| Split height                                  |   3015.97px |                             395.70px |
| List inner client/scroll height               | 3014/3014px |                           394/3014px |
| Scroll offset after physical wheel delta 1600 |         0px |                               1600px |

The probe only adjusts the visible panel's flex/min-size/overflow role. It is
diagnostic, not a reviewed final stylesheet. No scrollTop setter or locator
auto-scroll is used for the reachability measurement. Before and probe PNGs
plus `initial-probes.json` are retained in ignored evidence.

The fix must assign explicit scroll ownership across the intervening wrappers,
not replace native wheel behavior or remove the split library. Skills discovery
has extra page-local height:auto/overflow:visible rules in `page.css:41-57`.
Their compatibility with a bounded tab must also be reviewed.

## B. Account removal entrance — user-supplied case reproduced

The actual chain already passes a source:
`auth/AccountView.tsx:329-330` -> `auth/Page.tsx:273-288,613-625` ->
`auth/MutationDialogs.tsx:45-46` -> shared Dialog.

Instrumentation records only material geometry/animation timing and command
names. It never presses the destructive confirmation. Controlled preview
delays are fixture conditions, not proposed product delays.

| Engine          | Preview response delay | Entrance geometry result                                             |
| --------------- | ---------------------: | -------------------------------------------------------------------- |
| Chromium        |              immediate | cancelled at about 20.9ms; first sampled width already 480px         |
| WebKit          |              immediate | cancelled at about 13.0ms; first sampled width already 480px         |
| Chromium        |                  120ms | cancelled at about 122ms                                             |
| WebKit          |                  120ms | cancelled at about 117ms                                             |
| Chromium/WebKit |                  600ms | physical entrance completes before result; no premature cancellation |

All cases still report `data-motion-origin=trigger`. That attribute alone is
not animation evidence. `useDialogResize.ts:88-98` handles intrinsic content
changes during !settled by calling originSettler; this cancels the entrance
tracks in Dialog. Fast preview delivery exposes the bug rather than fixing it.
The correction belongs to the shared origin/content lifecycle, not an account
page delay, extra animation or removal of the impact preview.

## C. Origins — static discovery and additional lifecycle gap

TypeScript AST traversal of all src TSX import bindings/JSX found
39 Dialog / dialog-wrapper invocations (wrappers are included, so this is not
39 separate user workflows). 36 pass originRef. Three missing-prop locations
are two callers and the body of the same WorkBuddy trust wrapper:
`agents/AgentAssignmentSections.tsx:328`, `mcp/Page.tsx:606`,
`shared/ui/WorkBuddyTrustDialog.tsx:12`.

Both assignment flows can open after asynchronous success; the real switch or
bulk button has to be captured before the operation, not looked up afterward.

Skills menu experiment, both engines: Skill Settings enters with a trigger, but
after 700ms its originating menu item has unmounted; exit is neutral. This is
not a missing prop. ZIP picking and other asynchronous/transient sources need
click-time geometry plus an explicitly owned return/focus anchor, not a global
last-click cache or a fabricated matching button.

### Required branch matrix (coverage obligations, not claimed passes)

| Owner   | Entrances to cover                                                                                      | Critical edge                                                         |
| ------- | ------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Auth    | add/re-login, remove account, connect/switch/disconnect                                                 | immediate/mid/late/failed preview; cancellation and fresh sessions    |
| Agents  | agent login, WorkBuddy trust, shared file recovery                                                      | real switch -> delayed authoritative result; route hidden during wait |
| Skills  | detail/install, ZIP, unmanaged import, backups, settings, uninstall, backup delete, directory migration | vanished menu item, nested confirmation, local picker delay           |
| MCP     | new/edit/delete, discovery install/target, overwrite, WorkBuddy trust                                   | preview loading, assignment switches and bulk buttons                 |
| Models  | save/overwrite, delete model, connectivity probe                                                        | confirmation preview, chips and multiple model adapters               |
| Prompts | delete and unsaved changes from local selector/navigation                                               | semantic tab capture; cancelled navigation focus                      |
| Memory  | dirty-state and restore/reset confirmations                                                             | nested/contextual source and immediate draft/secret clear             |
| Shared  | Dialog, ConfirmDialog, InstallTargetDialog, FileRecoveryButton and wrappers                             | origin required or explicit neutral rationale; no custom modal bypass |

The initial static scan found no native alert/confirm UI or page-owned raw Radix
Dialog primitives; retain an executable AST guard and runtime branch coverage.
Prop presence, build success and an animated permanent UI Lab specimen do not
prove real asynchronous or conditional callers are correct.

## D. Scroll-state coverage obligations

| Route   | Long-content states to exercise                                             |
| ------- | --------------------------------------------------------------------------- |
| Agents  | catalog and configuration resource lists, tool output, nested details       |
| Auth    | accounts/connections, long impact preview, stacked detail, modal close      |
| Models  | catalog, model chips, long form/error, narrow detail                        |
| Skills  | installed list/detail/assignment, discovery results and footer, all dialogs |
| MCP     | installed/discovery, editor/assignment, long command/header values          |
| Prompts | application/list/editor panes, multiline draft, narrow stack                |
| Memory  | directory/list/content, long text and errors, narrow stack                  |

Test real wheel movement, keyboard reachability and visible final rows/actions,
not only no-horizontal-overflow. Preserve route keep-alive drafts/selection and
local scroll state; background roots stay inactive and modal scroll locks clean
up. Empty/loading/error states must remain usable. Firefox/minimum native
WebView/physical GPU validation is not implied by these two-engine probes.
