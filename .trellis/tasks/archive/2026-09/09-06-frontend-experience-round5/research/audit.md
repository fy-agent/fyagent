# Round-five baseline and evidence

## Confirmed source facts

Product baseline: `53ba8752` on `dev/laiyongjie`; clean before planning.

| Owner                         | Observed implementation                                                           | Relevance                                                                                                                                  |
| ----------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `app/styles/tokens.css`       | dark color-scheme; background #172b40/#223e56/#2e4e68; white foreground hierarchy | Brightening needs paired foreground/status roles, not only a background edit.                                                              |
| `shared/ui/GlassMaterial.tsx` | strength .012, dispersion 0, brightness/glow 0, sheen .12; frost from 20px token  | Existing optics are deliberately weak; a new dependency is not evidence of stronger useful material.                                       |
| `app/styles/controls.css`     | 10px overlay blur plus 20px surface blur, 0.68 dark tint                          | Both backing passes and tint must be compared visually; raising blur alone is not the requested result.                                    |
| `shared/ui/Dialog.tsx`        | geometry 420ms, curve .16/1/.3/1; foreground delay 22%=92.4ms and duration240ms   | Existing tracks overlap early and do not implement the staged shell/content handoff.                                                       |
| same Dialog + GlassMaterial   | plain backing changes to enhanced Glass after settled state                       | A real end-state material replacement exists; whether it causes a visible seam requires frame comparison.                                  |
| same Dialog                   | exiting body/actions removed immediately                                          | Preserve immediate action/secret clearing; bounded inert non-sensitive fade is an explicit new presentation contract, not a form snapshot. |
| `usePressFeedback.ts`         | independent press recovery, scale target .975                                     | Presentation must coordinate with it rather than hide its response.                                                                        |

No source conclusion above establishes FPS, native GPU cost, or subjective approval.

## Production navigation measurement before implementation

Existing `navigation-performance.spec.ts`, production build, one worker,
Chromium, 1232x700, deterministic native fixture, first visits plus42 returns.
Label `round5-baseline-53ba8752`, log `/tmp/fyagent-round5-baseline-performance.log`.
The log reports all3 existing tests passed.

| CPU cost | Return p50 | Return p95 | Observed long tasks (includes first visit) |
| -------- | ---------- | ---------- | ------------------------------------------ |
| 1x       | 40.6ms     | 44.3ms     | none                                       |
| 4x       | 51.5ms     | 57.8ms     | one85ms                                    |

Timer: semantic NavLink activation to frame following visible destination,
not OS input or animation completion. This one run is an initial reference,
not the three-run comparative gate or a modal animation benchmark.

## Required additional measurements

The existing material/response suite was rerun before product changes: all12
tests across four desktop projects passed, exit0 recorded in
`/tmp/fyagent-round5-baseline-materials.log`. Its inherited fixture screenshot
location is `node_modules/.cache/fyagent-ux-round4/`; the directory name is a
historical test convention, not a claim that this is an old unrepeated run.
The fresh1232x700 dialog screenshot was inspected: exterior blur is strong,
while the central material is still dark and relatively flat. Existing contrast
passing does not refute the user's brightness or material feedback.

Before changing product styling, collect fixed-fixture screenshots and raw
modal frame samples against this code revision. Review lightness, selection,
warnings/errors, focus and semantic buttons across all7 pages. Add warm/cold
modal frame/layout/painters evidence; navigation results alone cannot justify
an optical implementation. Keep full background/foreground contrast sampling.

## Review passes already performed

1. Visual architecture: paired palette roles, backing topology and optical ownership.
2. Temporal architecture: source/geometry/material/content handoff, comparison of
   actual curves and independent source feedback rather than duration alone.
3. Safety/compatibility: React18 and existing Motion/Radix, no source/form clone,
   no return to removed controls, no fake reduced-motion or jsdom shortcut in production.
4. Delivery: two separate work units, parent combination gate, final archive links
   and clean tree; public research and runtime observations remain separate.
