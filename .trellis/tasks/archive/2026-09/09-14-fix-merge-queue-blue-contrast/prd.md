# Fix merge queue dark blue contrast regression

## Goal

Restore compliant dark-theme contrast for the Health stop-check control, validate the WebKit regression, and return PR #188 to a green merge queue.

## Requirements

- Treat merge-queue run `34842498230` as a browser-test synchronization defect unless product evidence proves an actual palette defect. Preserve the current dark-theme tokens and the existing 4.5:1 text-contrast threshold.
- Before raster contrast sampling on `/health`, wait for the route-owned initial health read and its conditional batch controls to reach a stable state. Do not use an arbitrary delay.
- Keep all eight route samples, Chromium/WebKit coverage, and the shared contrast sampler unchanged unless broader evidence requires a different owner.
- Record the non-obvious two-phase raster-sampling pitfall in the owning frontend quality SPEC so future dynamic pages do not pair stale text geometry with a later layout.
- Validate the focused WebKit regression repeatedly, then run the repository frontend quality gates before pushing the PR branch.

## Acceptance Criteria

- [ ] The health-page contrast test cannot record the transient `停止检查` text and then sample pixels after that control has unmounted and the primary action has shifted into its coordinates.
- [ ] The dark-blue composited text test passes repeatedly in WebKit at 1232×700 without lowering contrast budgets or changing product colors.
- [ ] Frontend type checking, formatting, lint, unit tests, contract validation, and the relevant browser tests pass locally.
- [ ] PR #188 is pushed, its branch and merge-queue CI are green, and the PR is merged into `main`.

## Notes

- Failure evidence: WebKit reported `停止检查` with foreground RGB(232,245,255) over RGB(179,226,255), exactly the dark primary-action fill, at 1.24:1. The stop action itself uses the dark secondary-control surface.
- `sampleTextContrast` records text/color/coordinates, hides text, and only then captures the raster. The health batch can finish between those phases, unmounting the stop action and moving `检查全部软件` into the old coordinates.
- This is a lightweight, test-and-SPEC-only repair; no product behavior, native contract, or palette redesign is in scope.
- Local evidence: the focused WebKit case passed 10 consecutive runs, the complete blue-theme browser file passed 30/30 across four Chromium viewports plus WebKit, and `check:prearchive` passed the complete frontend, Rust, and repository-contract gate.
- Trellis archival precedes remote execution. PR checks, the replacement merge-group run, and the final `main` readback are post-archive evidence completed by the same session rather than claimed inside the work commit.
