# Research

## Current checkout

- Starting branch: `dev/laiyongjie` at
  `e33d37dd6f9d58c11207f843b5c33750a79dbb4a`, tracking the same upstream
  revision with a clean worktree.
- `src/index.html` already selects `src/v2/main.tsx`; the V2 hash router, six
  empty pages, Router-owned selection, development-only UI Lab, and lifecycle
  bridge already exist and must remain.
- The incorrect native-window contract is isolated to AppShell/TopBar,
  `WindowControls`, V2 platform `windowFrame` modules/types/exports, their three
  dedicated tests, and matching Shell CSS/test assertions.
- `src-tauri/tauri.windows.conf.json` already selects visible system chrome.
  The renderer-side Windows adapter is the remaining source of
  `setDecorations(false)` and can be removed without a native config change.

## Dependency decision

Selected package: `@samasante/liquid-glass@0.1.1`.

- Versioned registry metadata:
  <https://registry.npmjs.org/@samasante%2fliquid-glass/0.1.1>
- Published tarball:
  <https://registry.npmjs.org/@samasante/liquid-glass/-/liquid-glass-0.1.1.tgz>
- Source and browser notes were reviewed at upstream commit
  [`4e7b769e1df7e5a7d3669fef22417fe3d2f79ade`](https://github.com/samasante/liquid-glass/tree/4e7b769e1df7e5a7d3669fef22417fe3d2f79ade),
  including the commit-pinned
  [browser matrix and limitations](https://github.com/samasante/liquid-glass/blob/4e7b769e1df7e5a7d3669fef22417fe3d2f79ade/BROWSERS.md).
- The lock records integrity
  `sha512-i4cQzlwmpYnVl9eBDuBFyPsOsuBMMYDp1ijxul5Z/H+ns5aQ1qiVV5pTEBbO4EQo8ct+eVlS9bVZW8YUfX6nXg==`
  for the exact `0.1.1` package.
- License: MIT; the published package lists its LICENSE file.
- Peer contract: React and React DOM `>=18`.
- Runtime dependencies: none; React is a peer.
- The headless `Glass` accepts `className`, `style`, partial optics,
  `filterResolution`, and `live`, so FyAgent can own appearance and semantics.
- Very wide lenses and many/large simultaneous SVG filters are documented as
  GPU-bound risks. The implementation therefore uses one content-sized active
  navigation lens and keeps broad structural glass in CSS.
- The package is pre-1.0 and its WebView2 behavior is not covered by this
  task's native/manual evidence. Exact pinning, an internal adapter, CSS
  fallback, Chromium Playwright, renderer build, and full local checks are the
  risk controls.

## Rejected expansion

- Do not install a full component system: current Radix behavior, Phosphor
  icons, and React Router already satisfy the interaction contracts.
- Do not source-copy example components or vendor third-party CSS. Only the
  published headless engine is installed, and FyAgent owns its minimal wrapper
  and styles.
- Do not stretch the filter across the navigation track, tools, content plane,
  or ambient background.

## Full-gate baseline findings

The ordered full-project gate exposed four pre-existing contract gaps after
the V2 implementation was already green. Git history confirmed that none was
introduced by the selected lens or this task's production changes:

- Four tracked V2 root toolchain files were absent from the fail-closed change
  classifier added before them. They belong to the existing
  `contracts + frontend` classification.
- The current-host Mise contract still expected three Windows Rust wrapper
  calls even though the established helper-preparation contract requires four.
- The macOS retry-script test selected the obsolete Windows WSL `bash.exe`
  launcher and allowed Git Bash to reorder the mock `PATH`; its test harness,
  not the release script, required a safe argv-only cross-platform runner.
- ESLint 10 had introduced the dev-only metadata path
  `eslint@10.8.1 -> ajv@6.15.0 -> uri-js@4.4.1 -> punycode@2.3.1`, while the
  DEP0040 checker still recognized only the older reviewed jsdom origin. The
  `uri-js` CommonJS main bundles its own conversion implementation and a real
  Node 24 pending/throw probe completed without DEP0040; bare Node core
  `punycode` still failed the same probe. The checker therefore permits only
  the exact contiguous ESLint suffix (and the existing jsdom path), while
  retaining unknown-origin, version-drift, historical-chain, alias, and
  lock/why reconciliation failures.

These repairs touch only acceptance-gate classifiers, tests, the DEP0040
checker, and their existing executable specs. They do not change product,
release-script, Rust, Tauri, or installer behavior.
