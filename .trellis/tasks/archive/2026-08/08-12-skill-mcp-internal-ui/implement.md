# Implementation plan

## 1. Foundation

- Add V2 domain types, six-app constants, parsers/search/selection/pagination/bulk helpers, typed feature ports, Tauri and browser adapters, query keys/hooks, injectable provider, session install-target state, and toast host.
- Move pure MCP preset data to the V2-legal shared domain module and make the Legacy preset path a compatibility re-export.
- Add the minimum shared V2 controls and isolated feature styles without changing shell DOM, tokens, or unrelated pages.
- Add port/helper tests before page integration, including command payloads, native/browser behavior, secret search exclusion, parsing, pagination, selection, and extension preservation.

## 2. Skills

- Implement installed header, search/list/selection, details, update state, assignment, bulk management, empty/error/loading states, uninstall confirmation, and authoritative mutation reconciliation.
- Implement discovery sources, filters/cards, shared install target, skills.sh numeric pagination, install-to-installed handoff, and repository-empty behavior.
- Implement repository, ZIP, unmanaged import, backup, storage migration, and sync-method dialogs with confirmation, progress, accessibility, and resource refresh.
- Add focused component tests for all requirement and failure paths.

## 3. MCP

- Implement header, secret-safe search/list/selection, transport-aware details, assignment, bulk management, empty/error/loading states, import, and deletion.
- Implement the responsive add/edit dialog with presets, Custom quick forms, advanced JSON, validation, extension/hidden-app preservation, and duplicate-submit prevention.
- Add focused component tests for transports, import outcomes, presets/custom forms, validation, editing, deletion, secret isolation, and partial failure.

## 4. Contract and browser acceptance

- Migrate the Phase 1 empty-page contract only for Skills/MCP; keep the other four pages empty and retain every architecture restriction.
- Extend deterministic Playwright coverage at all four configured viewports using init-script Tauri invoke fixtures. Verify shell freeze, responsive columns/grid, dialog bounds/scrolling, keyboard reachability, no overflow, and no console/page errors.
- Confirm the production build contains no fixture entrypoint or development-only feature route.

## 5. Validation and review

Run, in order:

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run format:check
git diff --check
```

- Inspect `git diff --stat`, the complete diff, and Git status; confirm no `src-tauri/**`, shell visual, unrelated page, dependency, or external-reference change.
- Dispatch independent Trellis check for spec, architecture, accessibility, security, state reconciliation, and test completeness. Fix findings and repeat applicable validation.
- Report browser/mock evidence separately from unperformed native Windows/WebView2 and real configuration-write validation.

## Rollback points

- Foundation: V2 provider/ports/controls and preset re-export can be reverted without backend changes.
- Skills and MCP page work is independently revertible while pages remain route-compatible.
- Contract tests must be reverted together with returning the corresponding page to an empty Phase 1 element.
