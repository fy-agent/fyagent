# Primary sources and decisions

Reviewed 2026-09-06; installed versions were read from package.json/lock and
the actual library source. No dependency upgrade is selected.

1. react-resizable-panels 4.12.3 tagged API:
   https://github.com/bvaughn/react-resizable-panels/tree/4.12.3
   Panel numbers are pixels; default/min/max sizes and preserve-pixel-size /
   preserve-relative-size are supported. At least one pane must remain relative.
   Panel class/style apply to a nested element; Group Flex properties are
   library-owned. Use the existing adapter and role-specific flexible index,
   not a new drag handler or an override of Group layout.
2. MDN container queries:
   https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Containment/Container_queries
   Base local arrangement on the containing box, which is not necessarily the
   viewport. An intrinsic Grid/Flex baseline remains necessary where query
   support is unavailable. Do not require new JS measurement loops for CSS rows.
3. MDN Grid sizing and auto-fit:
   https://developer.mozilla.org/en-US/docs/Learn_web_development/Core/CSS_layout/Grids
   https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/minmax
   https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/grid-auto-rows
   Explicit intrinsic row tracks avoid using leftover height as content height;
   auto-fit with minmax distributes available inline space after honoring a
   meaningful minimum. An item spanning all columns can keep tracks occupied;
   cap this info-grid role at two columns rather than creating an empty third.
4. Carbon 2x Grid usage:
   https://carbondesignsystem.com/elements/2x-grid/usage/
   Layout follows content and user tasks; repeatable local modules and spacing
   roles improve consistency. High-density interfaces and long-form reading
   have different width needs. This supports role-specific policy, not a
   universal 70%, 80% or golden-ratio constant for every child.
5. GOV.UK summary-list pattern:
   https://design-system.service.gov.uk/components/summary-list/
   Keep names, values and actions as explicit semantic slots. Reuse that
   composition principle with FyAgent's existing definitions and Buttons;
   adopting another complete UI framework is unnecessary here.
6. W3C Visual Presentation, SC 1.4.8 (AAA):
   https://www.w3.org/WAI/WCAG22/Understanding/visual-presentation.html
   Reading-line guidance is not a pane occupancy percentage. Avoid treating
   maximum use of width as a universal requirement for paragraphs. This task
   does not claim AAA/full-product accessibility conformance.

## Alternatives rejected

Implementation follow-up: MDN ResizeObserver observation errors
https://developer.mozilla.org/en-US/docs/Web/API/ResizeObserver#observation_errors
explains same-delivery writes to observed geometry and their error event. The
new resize/draft regression exposed an actual 900→868px viewport clamp being
tweened back by the content-resize owner. Prevent that unintended write rather
than suppressing the observer error or adding a repaint polling loop. Explicit
content-height and size-stage transitions retain their existing animation.

- A new split/grid component library: current dependency already supplies the
  required sizing and resize lifecycle. The failures reproduce in our shared
  styles and adapter policy, not a missing library capability.
- More window breakpoints only: independent pane dragging would still break.
- Forced repaint/remount/resize polling: can hide stale intrinsic geometry but
  adds lifecycle risk and can lose drafts, focus or source references.
- Pure Flex nowrap or changing only each row to Grid: controlled WebKit
  experiments remained faulty (see layout-evidence.md).
- One global percentage/minimum for forms, metadata and long prose: conflates
  different content roles and can make narrow controls unreadable.

The 268px list / 280px action-rail defaults, 256px info-card floor and 30%/8em
metadata-label cap are measured local starting points. Validate them in the
final implementation and preserve supported long text/zoom behavior; none of
these sources certifies those exact project-specific values as optimal.
