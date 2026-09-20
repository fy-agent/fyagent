# PR CI follow-up

PR #193 presents the FDE portion of the same delivery as subscription PR #192.
The first hosted run, `35493454323`, was not accepted: Windows backend tests and
the WebKit Agents contrast case failed, although the other native/contract jobs
passed. Final hosted acceptance is recorded on the delivered PR commit.

- Windows test binaries do not enter production `main`. The Agent AppState
  helper and health inventory fixture now initialize the existing real Windows
  Shell-user context, retaining the production admission boundary and temporary
  test configuration/log paths.
- The sample project-file workflow requires the macOS file implementation.
  Windows now tests the actual unavailable result, unchanged project revision
  and generation, and absence of published context or directories. Cross-platform
  verification tests remain enabled. Windows project context/isolated-directory
  preparation is not implemented by this follow-up.
- The guide used detail-brand sizing variables outside their defining containers.
  The reproduced frame was 512 px instead of 64 px. Local 64 px/48 px variables
  restore compact recommendations and keep completion/skip controls reachable.
  The Chromium/WebKit guide regressions retain viewport and keyboard checks and
  additionally check frame/artwork dimensions.
- Raster contrast collection waits for the initial Agent scan to finish before
  recording glyph coordinates. Scan completion removes a progress block and
  commits the card order; a screenshot taken after that transition cannot be
  compared with coordinates from before it. The contrast threshold and the
  deterministic bright-background test are unchanged.
- The shell keyboard contract counts controls in the top bar and navigation;
  a native-only content error may also expose a retry button. The actual 11-step
  keyboard traversal remains required.
- The theme test observes the real native animation and reverses it in the
  browser frame loop. It retains one track, 560 ms duration, easing, clip origin
  and resize/cleanup checks, without depending on a protocol poll catching a
  short-lived animation.

Local verification after these changes: macOS Rust fixture groups 22 + 2 + 1;
Rust formatting and all-targets Clippy; platform inventory; TypeScript, lint and
format checks; 40 staged repository contracts. The integration worktree ran the
identical guide changes through 30 Chromium/WebKit cases and the four affected
CI browser regressions through 19 cases (the existing shell test remains
Chromium-only). These receipts do not replace the new PR's complete hosted run.

No real subscription, customer-system UAT, Windows device acceptance, Apple
notarization, installed application replacement or release is claimed here.
