# State motion review

## Scope and ownership

Same-session dialog size/content transitions, bounded perceptible press,
non-replaying selection geometry and shared model disclosure. No native command,
permission, resource authority or credential lifetime changed. The existing
Motion/Radix/WAAPI owners are reused; no copied form trees or animation solver.

## Root causes and corrective decisions

1. The prior dialog ResizeObserver called origin-settle for every content size
   change. Steps now use an explicit presentation key plus intrinsic content
   observation. A separate 320ms actual fixed-window resize leaves text unscaled
   and the action row visible. Window resizing is still immediate. Reverse and
   close freeze current geometry; cancel clears secrets/actions immediately.
2. Initial/revisited lenses grew from zero and a fixed 48-frame loop repeatedly
   restarted movement. Initial/revisit uses actual geometry; true user selection
   travels, reflow observations correct directly. Observe real sibling layout
   and distinguish a newly registered host from an old observer notification.
   The enclosing page/tab now fades without another coordinate transform.
3. Actual WorkBuddy press admission succeeded but icon styles stayed at none.
   Installed motion-dom12.23.23 shared one cleanup array across subjects. A
   two-control executable test failed after disposing the first control. The
   reviewed upstream same-major12.43.0 moves cleanup ownership per effect; the
   exact same test and real positioned-control tests now pass. The lock changes
   three packages only. There is no local dependency patch or second engine.
4. Shared buttons now dip to0.96 and rebound up to1.004 using the existing spring.
   PressableButton owns previously raw business controls. Measured/positioned
   hosts animate their existing inner visual. Fractional bounding boxes are
   essential for measuring a sub-half-percent rebound: integer offsetWidth
   rounding incorrectly reported some WebKit recovery peaks below1.
5. Two model-ID sections duplicated instant conditional rendering. They now
   reuse ModelsExistingSection and declarative Motion auto-height. This also
   removes the manual scrollHeight cache/completion write that left WebKit
   at a fixed pixel height. Readonly collapsed data remains inaccessible/inert;
   private credential dialog bodies still clear immediately.
6. An absent ResizeObserver reached the pane library and broke the route.
   Its capability fallback now renders static stacked panes with no splitter.
   No polyfill or secondary drag implementation was introduced.
7. Final lifecycle review found the sibling-observer exclusion used a stale
   overlay class. Initial mounting passed because the lens did not yet exist;
   re-registering a different selected host incorrectly observed the animated
   lens itself. The same-node host-switch regression failed before the class
   correction. The observer now excludes the actual decorative output, avoiding
   needless measurements driven by its own width/height animation.

## Evidence and review limits

The old dependency failure is retained in
`/tmp/fyagent-round6-motion-isolation-before.log`. Its fixed regression plus
Dialog/Selection/Presentation assertions passed22 tests. Physical12ms pointer,
Enter and Space tests require a visible dip, positive bounded rebound, one
action and unchanged neighbour boxes. These include real data-page control
lifetime, not only a permanent UI Lab button.

WebKit's five state tests now pass: actual intermediate heights, preserved
session/choice, cancel/reopen/reduced-motion, no-observer reversal, readonly
model collapse and non-collapsing page revisits. Full aggregate and production
performance results are recorded at final verification below.

No minimum-version native WebView, real credential, signing, deployment or
physical GPU benchmark is performed. Browser paint/frame evidence is scoped to
the measured engines and fixtures, not all devices or a claim of zero debt.

## Final verification

Final reviewed source includes the observer-output exclusion from finding 7.
The same extended observer test fails before the correction and passes after
it (`/tmp/fyagent-round6-lens-observer-{before,after}.log`). No test threshold
or suppressed diagnostic was changed to make it pass.

- Complete active-task `check:prearchive`: exit 0. Unified strict type/lint/
  format and 174 unit files: 1,550 passed, 1 existing platform-specific skip.
- Rust fmt/check/Clippy and all 19 test suites: 3,495 passed, 0 failed,
  6 existing ignored; desktop fake IPC 7 passed. No native source changes
  against the round baseline `6117a7d9`.
- Release contracts: 611 passed, 1 existing skip; native-fetch 4 passed.
- Final `test:browser`: production boot/timing 2 passed, then all 351 browser
  regressions passed across four Chromium viewports and selected WebKit paths.
- Three serial `test:performance` runs: 10 tests passed per run, production
  build at 1232x700 with the same deterministic fixture and CPU settings.
  Logs: `/tmp/fyagent-round6-reviewed-performance-{1,2,3}.log`.
- Full dependency audit after the three-package Motion update: all severity
  counts zero. No added runtime framework, local patch or permission change.

All timings below are milliseconds. Each navigation condition has 42 revisits;
each presentation/state/theme condition separates one cold and 20 warm cycles.

| Metric                                           | Run 1       | Run 2       | Run 3       |
| ------------------------------------------------ | ----------- | ----------- | ----------- |
| Navigation revisit p95, 1x                       | 28.2        | 27.9        | 28.0        |
| Navigation revisit p95, 4x                       | 50.4        | 48.4        | 50.1        |
| Modal open/close warm frame p95, 1x / 4x         | 33.4 / 33.4 | 33.4 / 33.4 | 33.4 / 33.4 |
| Same-session size warm frame p95, 1x / 4x        | 33.4 / 50.0 | 33.4 / 49.9 | 33.4 / 50.0 |
| Theme capture preparation p95, 1x / 4x           | 34.2 / 34.8 | 17.3 / 34.9 | 34.8 / 33.4 |
| Theme reveal thirds frame p95, both CPU settings | 16.7-16.8   | 16.7-16.8   | 16.7-16.8   |

Normal navigation and normal warm frames meet the unchanged 100ms/33.4ms
targets. A 4x size-animation p95 near 50ms remains an explicit stress cost;
normal size-animation maxima also reach 50.1ms. This is not a 60fps guarantee.
Navigation long tasks in the 4x runs include 95-101ms cold work; modal 4x
sampling includes 51-52ms long tasks. Normal navigation, normal modal and both
state-size samples reported no long tasks. Navigation ends at a frame after
visible destination, not animation completion or operating-system input.

An earlier performance batch was stopped after the observer review finding;
its first run passed but the interrupted batch is not final evidence. The
three completed reviewed runs above include the correction. No compiler or
other browser suite ran concurrently with these measurements.

Five focused SPEC owners were synchronized before the work commit. All five
round-six task context validations pass at their current locations; 166
frontend SPEC relative links resolve. Parent integration owns final archival
relocation, working-commit ancestry and journal/clean-tree verification.
