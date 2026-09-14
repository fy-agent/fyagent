# Production timing and trace-recorder isolation

Reviewed 2026-09-14. This is a measurement repair, not a product-motion change.

## Observed failures

The first performance run failed the normal-CPU content-resize budget at 33.5ms
p95. A port-conflicting invocation also truncated its shared log while the first
process continued writing; the mixed log remains in `performance-overlap.log`,
not clean acceptance evidence. A subsequent exclusive complete run still failed:
`performance-final.log` records 34 passes and one failure, content-resize p95
50ms at 1x CPU, with no observed main-thread long tasks. The port collision alone
therefore did not explain the budget failure.

## Discriminating experiment

The performance config used `retain-on-failure`. This records each test and only
discards successful traces afterward; it is not a recorder started after failure.
Playwright documents recording costs and screenshot/DOM snapshot capture:

- https://playwright.dev/docs/trace-viewer#recording-a-trace
- https://playwright.dev/docs/api/class-tracing#tracing-start
- https://github.com/microsoft/playwright/blob/main/docs/src/trace-viewer.md
- https://playwright.dev/docs/api/class-testoptions#test-options-trace — explicitly
  distinguishes recording every run from retaining only failures.
- https://playwright.dev/docs/best-practices#debugging-on-ci — warns that tracing
  every test adds substantial recording cost.

Keep the same production build, real animation, viewport, real clock, twenty warm
cycles and 33.4ms assertion. Change only the trace recorder for diagnostics:

| Evidence log under node_modules/.cache/concise-renderer | Result |
| --- | --- |
| `performance-no-trace-ab.log` | Two serial 1x repetitions pass; both p95 33.4ms |
| `resize-without-trace.cYn7Fw` | Independent 1x and 4x cases pass; both p95 33.4ms |

Neither diagnostic changes product code, sample counts or budgets. These results
support trace instrumentation as the cause of the observed budget failures on
this host, not a claim that every native WebView has identical timing.

## Repair and protection

Use Playwright's existing `trace: off` in the dedicated performance config.
Functional tests retain their normal failure traces. An explicit `--trace on`
run remains available for diagnosis, with its timings labelled instrumented.
CPU profiles, frame/long-task reports, real intermediate-size and cleanup
assertions, one worker and zero retries are unchanged.

`tests/architecture/rootGovernance.test.ts` imports both real configs in native
Node and checks this separation and all four timing suites. The new assertion
failed before the config repair (`performance-config-red.log`) and all five
ownership tests pass afterward (`performance-config-green.log`).

The final gate is the entire canonical `mise run test:performance`, not the
diagnostic CLI override. A distinct log is used for each new run so a repeated
invocation cannot overwrite in-flight evidence. Final results belong in
`verification.md`; failed and diagnostic runs are retained separately.

## Final canonical result

`performance-accepted.gRDHIN` records all 35 production tests passing in 2.7 minutes.
No diagnostic CLI overrides were used. Normal CPU navigation p95 is 31.5ms;
presentation and content-resize frame p95 are 33.4ms, with theme segments at most
16.8ms. At 4x CPU cost navigation p95 is 55.3ms and both animation frame p95
measurements remain 33.4ms. Existing rounding only removes sub-nanosecond
floating-point subtraction noise; all budgets and samples remain unchanged.
This repairs benchmark workload isolation, not the product's animation code.
