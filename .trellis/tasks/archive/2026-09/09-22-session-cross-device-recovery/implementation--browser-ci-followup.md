# Browser CI follow-up

At head `6bda82ca1c9cf2b4faa54f3617e5671eda048c87`, [CI run 35657588102](https://github.com/fy-agent/fyagent/actions/runs/35657588102) passed the macOS and Windows backend jobs, repository/desktop contracts, and both Windows architecture contracts. Windows migration integration was 11/11. The frontend job passed production boot 3/3 but ended with browser regressions 666/667.

## Failure and evidence

The single failure was the existing `state-motion.spec.ts` page-revisit test at Chromium 1440×900: its sampled minimum lens width was 0 against the unchanged lower bound of 97% of the initial 70-pixel width. This test and the production SelectionLens/PersistentSurface/route-outlet implementation were unchanged from the PR base. The preceding hosted run passed all 667 browser tests with the same frontend source.

The test clicked away from Auth and immediately clicked back without establishing that the first route had committed. Its initial hidden check could therefore observe the old still-visible page and begin sampling before the legitimate hidden interval. A local diagnostic using real navigation and 6× CPU throttling observed this missing precondition in 4 of 12 rounds: the click had returned and the hash was `#/agents`, but Auth remained active, visible, and 70 pixels wide. The extra diagnostic observations did not reproduce the cloud's entire zero-width trajectory. The cloud run exposed no downloadable artifacts, so the report does not claim trace-level reconstruction.

## Minimal correction

Add explicit assertions that the Agents page is visible and the Auth page is hidden before clicking back. No product source, retry policy, wait duration, width threshold, frame sampling, or real-tab interpolation assertion changes. The correction strengthens the route-roundtrip precondition rather than accepting hidden zero-width samples.

After this change, the canonical state-motion file passed all 25 tests across its five configured browser/viewport projects; production boot passed 3/3. Under 6× CPU throttling, all 12 consecutive real revisits satisfied the departure assertions and original return-width threshold; every sampled return width was 70. This is one bounded diagnostic with 12 rounds, not 12 additional test cases. Formatting and diff checks passed. Coordinator `check:contracts` also exited 0, with 664 passes / 1 existing skip and native Fetch 4/4. A fresh hosted run of the corrected commit remains authoritative for PR completion. The earlier failing run is retained, not relabeled as successful.

Detailed diagnostics and local verification logs are retained outside the repository in `work/executor-logs/ci-browser-fix/`. This new follow-up document is not a transformed original in the historical archive path map.
