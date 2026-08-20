# Official links and Codex installer reuse

## Current link path

- The native catalog owns one `officialUrl` per entry in
  `src-tauri/src/commands/agent_catalog.rs:40-47, 62-190`.
- Agent details currently expose one generic official button for every entry
  and pass that URL to `ports.settings.openExternal`
  (`src/v2/pages/agents/Page.tsx:231-299, 313-329`).
- The V2 Tauri adapter validates the URL and invokes `open_external`
  (`src/v2/shared/platform/tauri/features.ts:123-129`). Rust registers the
  command and launches HTTP(S) through the interactive user's shell
  (`src-tauri/src/lib.rs:1843`, `src-tauri/src/commands/misc.rs:19-24`,
  `src-tauri/src/platform/process_launch.rs:264-274`).
- Mock/unit/browser tests prove the IPC payload but do not prove that Windows
  Explorer or macOS opener actually launched a browser. The reported failure
  therefore needs native reproduction/log evidence after the UI actions are
  corrected.

## Required product changes

- QoderWork, TRAE Work, and WorkBuddy retain vendor-owned HTTP(S) actions.
- Claude needs two distinct actions: official Claude Code CLI setup
  documentation and the official Claude Desktop download surface. The existing
  single-URL catalog wire cannot express both without a contract revision or a
  reviewed renderer-owned secondary link. Because these are durable catalog
  facts, revising the versioned native catalog is the safer single-source
  design.
- Codex must suppress the official-link action and instead expose the existing
  trusted desktop installer behavior.

## Existing Codex installer boundary

- The backend already registers the exact seven ordinary installer commands at
  `src-tauri/src/lib.rs:2160-2170`; its security contract is
  `.trellis/spec/backend/codex-desktop-installer.md`.
- The established renderer state machine lives in
  `src/hooks/useCodexDesktopInstaller.ts`; the existing presentation is
  `src/components/codex/CodexDesktopInstallerCard.tsx`, with focused tests under
  `tests/hooks/useCodexDesktopInstaller.test.tsx` and
  `tests/components/CodexDesktopInstallerCard.test.tsx`.
- V2 cannot import those legacy hook/component modules directly because
  `.trellis/spec/frontend/v2-shell.md` forbids V2-to-legacy dependencies and
  direct Tauri imports outside `src/v2/shared/platform/tauri/**`.
- Safe reuse means preserving the current backend commands, DTO validation,
  query/event snapshot ordering, start/cancel/launch/log actions, redacted
  errors, and installer state semantics while exposing them through a V2-owned
  port/query/view. It does not mean duplicating the backend installer or
  accepting a URL/path from the renderer.

## Validation boundary

- Component and fake-Tauri browser tests must cover all distinct official
  actions, Codex no-link behavior, installer states, exact IPC payloads, and
  error handling.
- A real native run must separately prove the system browser opens and the
  Codex installer card reaches the expected current-host status. A real install
  should be exercised only with a safe, explicit test state and remains
  separate from static or mock success.
