# Ahead-of-main commit audit — 2026-09-14

## Reviewed baseline

The review baseline is the six commits that were present in
`origin/main..dev/laiyongjie` when this task started:

| Commit | Intent | Review result |
| --- | --- | --- |
| `223fbbc0` | Resolve Claude/Grok npm latest metadata at runtime instead of compiling reviewed manifests. | Implementation uses a concrete semver and root/current-platform integrity before composing executable argv. Registry tags never become install authority. The principal defect was documentation ownership: shared npm source mechanics were repeated across lifecycle, Claude and Windows specs. |
| `b2cdc098` | Apply the live Grok npm plan and npm 12 script policy on development Windows. | LocalProcess consumes the same exact plan, derives policy from the npm executable that actually runs, rejects blocked install scripts and requires PATH-default version readback. The `1.2.3` value remains only a command-shape fixture. No product-code defect was demonstrated. |
| `379bb0d1` | Simplify secondary Renderer pages and close warning/layout/browser validation gaps. | Copy and layout changes preserve destructive-action, uncertainty, secret and recovery boundaries. React warning guards survive restore-mock lifecycle; scroll/density tests exercise native wheel/keyboard behavior, responsive re-entry and draft preservation. |
| `ee475a78` | Stabilize the full-repository supported-platform snapshot test. | The integration test captures one tracked-file snapshot, keeps zero-findings and inspected-file-count assertions, and gives only that repository integration case a bounded 15-second watchdog. It does not loosen a production scanner or product performance budget. |
| `9a6ceab4` | Archive the prior concise-renderer Trellis task. | Archive contents match the implementation commit and retain its check/research artifacts. No active-task path was left behind. |
| `ce6e4916` | Record the prior work journal. | Journal ordering follows the work commit and task archive. No code or SPEC authority is hidden in the journal commit. |

## Code-path trace

### Shared live npm lifecycle

- `services/tooling/grok_npm.rs` parses a live `/latest` document, requires a
  concrete version and current-platform optional package, and admits a registry
  only when root and platform SHA-512 values match the same manifest.
- `GrokNpmInstallPlan::npm_argv_for` composes `package@<resolved-version>`;
  neither `@latest` nor the command-shape fixture is executable authority.
- Claude and Grok use the same compact exact plan while retaining distinct
  package, scoped-registry, script-policy and post-install verification.
- Windows helper execution derives the npm major from the executable that will
  run. npm 12+ receives only the reviewed package-specific allow-scripts flag.
- Development Grok LocalProcess rejects blocked-script output and does not
  report success until the PATH-default Grok version reaches the plan.
- macOS discovery consumes login/process PATH plus product environment, does
  not walk manager internals, and selects the PATH-default install when several
  copies are visible.

Focused Rust evidence: `mise run rust:test -- grok_npm` completed successfully;
the primary library ran 9/9 matching manifest, integrity, concrete-version and
script-policy assertions, and every filtered Rust test binary exited cleanly.

### Concise Renderer surfaces

- Skills and MCP retain assignment switches, trust/overwrite dialogs, source
  visibility and secret redaction while removing repeated explanatory cards.
- Health removes redundant summary prose without removing live status,
  filtering, stop semantics, configuration routes or error evidence.
- Auth removes duplicated labels while preserving account identity, connection
  identity, request source and device-code recovery.
- Prompt/Memory retain truthful native-only empty/error states and reachable
  controls; no seeded or fabricated data was introduced.
- The React warning guard is tested across consecutive cases with the actual
  setup lifecycle, so `restoreMocks` cannot silently disable it.
- Responsive/scroll tests use real wheel and keyboard movement, multiple
  Chromium viewports, WebKit, narrow/wide re-entry, enlarged text and draft
  node preservation.

Focused Renderer evidence:

- `mise run check:frontend`: typecheck, ESLint, formatting, 187 unit files /
  1649 tests and desktop visual preflight passed.
- `mise run test:browser`: production build plus 586/586 browser tests passed
  across Chromium and WebKit. The initial production-route performance boot
  checks also passed.

## SPEC findings and resolution

Four changed owner documents exceeded, or were too close to, Trellis's default
32768-byte injection boundary. They mixed stable public contracts with
platform-, source- or workflow-specific details. Raising the limit was rejected.

The review introduced focused owners for product sources/desktop identity,
prearchive session proof, supported-platform governance, native host task
execution, optional Windows-MSVC diagnostics, Windows Agent execution and
branch-push commit policy. Existing filenames remain valid routing entry points.

After decomposition, the largest affected owner is
`github-ci-workflow.md` at 29922 bytes. All local Markdown links resolve, and
the active task's curated implement/check contexts validate with 18 entries
each and no truncation warning.

## Remaining evidence boundary

Portable/macOS checks do not prove native Windows registry views,
WinVerifyTrust, Explorer-user process identity, Visual Studio environment
loading, installer/UAC behavior or Windows packaging. Those remain explicit
matching-host CI/HIL evidence and are not represented as locally passed mocks.
