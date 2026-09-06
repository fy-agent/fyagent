# Consolidation Review

## Scope and preservation

Baseline: `ec1393c1`. The adopted renderer moves to role-based directories;
unreachable predecessor UI, its exclusive dependencies/tests and offline HTML
delivery are retired. Portable configuration/Codex/security owners and their
tests remain. The durable owner/retirement map is
`docs/fyagent/development/renderer-migration.md`.

No `src-tauri` source, Cargo, command registration or ACL change is included.
The merged Claude CLI, reversible writes and hidden-window startup behavior
remain intact. Persisted provider IDs, native DTO versions and third-party
model versions are not renamed. No account operation or user configuration
write was executed during browser/native mock checks.

## Review findings resolved

Unified test/lint coverage exposed unvalidated dynamic configuration types;
use unknown and existing structural guards, not suppression. Literal bearer
token replacement now uses a callback so `$&`, `$1` and `$$` remain literal.
Query readers and daily-search invalidations share the renderer namespace/key
factory. Tests assert actual current source coverage, not a vacuous old tree.

The Memory test now waits for the captured confirmation portal to detach before
typing: removal from the accessibility tree happens earlier during exit. The
business draft lifetime was not changed to satisfy that test.

CI retains collect-all diagnostic behavior and aggregates new lint/browser
steps. Browser download failures cannot turn a skipped regression into success.
Canonical local host checks use mise rather than a package shim injecting
NODE_PATH; the native runtime injection guard was not weakened.

## Asset and reference evidence

Raster identities were compared to both the baseline Git blobs and the sealed
digests: 5 product rasters moved byte-for-byte; 27 retired-only rasters removed;
121 remain. Structure admission drops 7 retired candidates (112 to 105), with
only the three reviewed checker/test digests changed. No visual baseline was
recaptured or accepted automatically.

276 task JSONL manifests checked with archive self-reference resolution:
zero newly broken references, zero active broken references. There are 168
pre-existing historical unresolved references outside this migration; those
are not claimed fixed. Changed retired-owner references resolve to the migration
record, retaining their original historical reasons rather than inventing
current test evidence. Maintained code-spec links resolve. Six multilingual
manual sections no longer direct users to a downloaded HTML generator.

## Verification

- Unified type, lint, formatting and unit gates: 169 files, 1,528 passed,
  1 existing platform-specific skip.
- Production boot/build and full browser regression: 272 passed across four
  Chromium viewport projects and selected WebKit scenarios. Existing route
  chunk budgets remain unchanged; seven product route entries verified.
- Native fmt/check/Clippy/test: 3,495 passed, 0 failed, 6 existing ignored.
- Desktop fake-IPC: 7 passed; visual manifest preflight is candidate readiness,
  not a claimed native visual run.
- Contract-only prearchive after the final documentation correction: passed,
  including release contracts (611 passed, 1 skip) and native-fetch (4 passed).
- Final complete prearchive rerun passed (exit 0), including the unified
  1,528 tests, native 3,495 tests and release contracts. No React act warning
  occurred in this final run. The generated preview is absent; its obsolete
  ignore entry and only Finder metadata left inside the retired tree are removed.

Browser/native mocks are not Windows/macOS release-candidate GUI acceptance.
No push, release or deployment is part of this task.
