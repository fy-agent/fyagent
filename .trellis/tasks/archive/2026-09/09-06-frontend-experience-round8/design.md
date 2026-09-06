# Shared responsive layout roles

## Boundary and root causes

One focused task covers the shared layout chain behind both reported symptoms.
No parent/child orchestration or application redesign is needed. The smallest
behavior gap is incorrect intrinsic assignment-row height after re-entry,
inconsistent bulk-row wrapping, fixed middle detail versus unbounded action
rail, and viewport-only info-card admission. Evidence and source anchors are
in research/layout-evidence.md; primary API references are separate.

Keep the current seven routes, list/detail/assignment order, existing two/three
pane presentation boundary, both themes and native operations. Change sizing
policy where it belongs; do not hide a product regression by remounting a route
or adding repaint timers. New abstractions are limited to a shared composition
for the two existing identical bulk action sections and a sizing role for their
existing SplitPanes owner.

## 1. Assignment rows and grouped bulk actions

Use explicit intrinsic rows on `.fy-feature-assignments` (the minimal isolated
candidate is `grid-auto-rows:max-content`). Do not set fixed row heights: real
long labels and text scaling must still increase a row naturally. Keep current
31px minimum and icon/switch dimensions unless measured interaction requires
a separate decision.

Extract the duplicated Skills/MCP bulk block into one pure UI composition,
for example `shared/ui/BulkAssignmentPanel.tsx`. It receives the existing
closed target list, disabled state, `(target, enabled)` callback and optional
DialogOriginRef. It reuses Button, preserves target order and does not own
ports, queries, write locks, progress or success state. AssignmentPanel's
switch/radio contracts stay focused rather than acquiring unrelated flags.

Use explicit label/action classes, not `span:last-child` to guess semantics.
At sufficient section content width, all rows use label + atomic button pair;
below that width, all rows put the label above the pair. The measured current
label/pair requirement is about 221px before section padding, so a shared
230px content-box admission point is an initial candidate. The action pair
must not split arbitrarily. Native wrapping remains a safe fallback; tiny
unsupported widths must not cause hidden actions or horizontal escape.

Other `.fy-feature-assignment` consumers, including import checkboxes and
radio pickers, retain their own semantic layout. Test them explicitly.

## 2. Flexible content pane, bounded auxiliary rails

Extend the existing SplitPanes adapter with a minimal role-neutral flexible
pane selection and optional explicit default widths. Keep last-pane flexible
as the default for all existing two-pane callers. A shared three-pane detail
profile selects index 1, so both outer rails preserve their usable pixel size
and the middle detail receives remaining width.

Initial role values, in CSS pixels at the existing density:

| Role            | Minimum | Default         | Maximum / growth                     |
| --------------- | ------- | --------------- | ------------------------------------ |
| Resource list   | 220     | 268             | 420; user resizing retained          |
| Detail          | 360     | Remaining space | Flexible, no artificial fixed cap    |
| Assignment rail | 220     | 280             | 360; natural compact fallback inside |

Keep these numbers in one `shared/ui/split` sizing-policy owner, not repeated
in Skills and MCP. Use the library's defaultSize/minSize/maxSize and
groupResizeBehavior, preserving at least one relative pane. Do not override
Group CSS or add a second persistent size store/ResizeObserver resize loop.
Use supported Panel callbacks/handles for reset, not property mutation during
render. Retain existing static fallback when ResizeObserver is missing.

Stable pane keys and the first two content trees survive 2↔3 admission. Natural
window resizing must not overwrite a user's drag choice on every frame. Check
group shrink constraints and return to wide against the chosen role, not just
initial defaultSize. The library's nested className placement is part of the
adapter contract. Remove only proven ineffective Skills/MCP `--fy-split-pane-*`
declarations; preserve live catalog reporting aliases and other page roles.

## 3. Inner density by semantic role

The existing `tokens.css` owns CSS layout roles (gap, card floor, label cap,
compact action policy); numeric pane constraints consumed by JS have one
separate split-policy authority. CSS must not declare another unused set of
pane sizes. Different roles may share a spacing token, but a form minimum is
not changed simply because an info card minimum changed.

- Info grid: fill the actual detail content box; a 256px card floor and 12px
  gap are initial candidates, maximum two columns for source/assignment cards,
  with the installation block retaining full-span semantics. Use existing CSS
  Grid minmax/auto-fit or one named container rule, not window-only admission.
  A full-span item must not keep an otherwise blank third column.
- Metadata: label consumes at most 30% and at most 8em; value consumes the
  remaining space after the gap. Short fields retain natural readable text,
  and narrow cards stack name/value when necessary. Test the copy-only path
  case independently: never reveal intentionally hidden paths for density.
- Card height: natural content and top alignment, not uniform tall interiors
  for short secondary cards. Meaningful whitespace is allowed; the criterion
  is usable content width and predictable alignment, not filled pixels.
- Auth definitions already have a 500px stack rule, and Models/Auth forms
  already have 230px auto-fit floors. Audit shared-rule effects and retain
  their role-specific behavior; do not blindly replace those values.
- Long prose, code and secrets keep their existing reading, local scrolling
  and disclosure rules. No global percentage is applied to every child.

## 4. Expected files and non-goals

Expected product changes are limited to shared assignment/bulk presentation,
`split/SplitPanes.tsx` and its sizing owner, `features.css` / `tokens.css`,
Skills/MCP page composition and their ineffective size declarations. Adjust
Auth definitions or other shared consumers only for a demonstrated collision
with the new shared rule, not for unrelated visual redesign.

Tests extend existing renderer/browser suites plus one focused resize-density
file selected for Chromium and WebKit. SPEC updates target surfaces-responsive,
assignments, component-guidelines/reuse and quality-guidelines, with page-owner
links when needed. Align the assignment signature/semantic description to the
actual switch API during that update; no native assignment behavior changes.

No new dependency, version upgrade, native command, query-state rewrite,
window-shell resize animation, credential access, deployment or root-level
scratch/config file is part of this task.

## Verification and rollback

Require a failing baseline test at the 1181px re-entry boundary, real divider
dragging and A→B→A window sequences in both engines, both themes and long data.
Test normalized row heights against their own content, actual action positions,
name/value bounds, no missing targets, native wheel/keyboard reachability and
MCP draft identity. Zero JS errors or no horizontal overflow is insufficient.

Pin tests to adopted public semantics and the project's explicit role data,
not generated Radix IDs or a guessed library class location. Keep the round7
origin, scroll, reduced-motion and focus coverage. Run full correctness gates
and serial production performance with unchanged budgets after integration.

Rollback is a coherent UI/policy/style/test revert; no locks or persisted data
are migrated. Implementation may use one isolated worktree after final plan
approval; merge only reviewed commits back to dev/laiyongjie, verify ancestry
and cleanliness, and remove only the owned merged worktree/branch.
