# Technical design

## Architecture and ownership

- Keep the existing router and AppShell ownership. Add an app-level, non-visual V2 feature provider around the router for a dedicated QueryClient, injectable feature ports, the session install target, and a single toast host. It must not alter shell DOM or styling.
- Pages depend only on `src/v2/shared/**`. Domain DTOs, application constants, query keys, parsers, bulk helpers, and ports live in shared V2 modules; direct Tauri invoke implementations live in `shared/platform/tauri` and browser fallbacks in `shared/platform/browser`.
- Shared UI contains behavior-complete primitives only. Skills/MCP layouts, cards, details, dialogs, and controllers remain within their page folders. Add module-specific CSS files imported by the V2 style index, using `.fy-control-*`, `.fy-skills-*`, and `.fy-mcp-*` namespaces.
- The pure MCP preset source moves to a shared V2 domain module that does not import V2 UI. The Legacy preset module re-exports that data and retains its Legacy-only translated-description helper.

## Data flow and concurrency

- Each resource uses stable Query keys. Initial loads may skeleton; refetch retains cached data and emits an inline stale-data notice on failure.
- Mutations use one write coordinator per page. It rejects concurrent conflicting mutations, exposes pending operation/progress, and always invalidates/reloads every resource the backend may have partially changed.
- Bulk operations derive target IDs from the complete authoritative collection, not filtered display rows, and call single-item commands sequentially. Results contain successes and `{id,error}` failures without logging secret-bearing payloads.
- Selection is derived by a pure convergence helper after every collection/filter change: keep current if present, else first, else none.
- The session install Agent is Context state, initialized to Claude, never persisted, and shared by discovery, ZIP, and backup restore. Unmanaged import maps each `foundIn` label to supported IDs and keeps a per-item set.

## Port contracts

- `SkillsPort`, `McpPort`, and `SettingsPort` expose semantic typed methods but pass the exact existing command payloads at the Tauri boundary. Tests assert all command names and camelCase payload keys.
- Browser read methods return empty maps/lists or default settings suitable for empty-state rendering. Browser mutation methods reject with a stable native-only error. External URLs are restricted to HTTP(S).
- MCP server/app types retain index signatures/unknown extensions at the wire boundary. Editing starts with a structural clone of the authoritative object and overlays only user-edited fields.
- Settings sync-method save is read-merge-save. Storage migration never calls settings save because the existing migration command persists its own target after moving files.

## UI behavior

- Master-detail layout uses CSS Grid with list/detail/assignment minimum widths. Below the measured three-column threshold the assignment region renders inside detail; there is one semantic instance at a time, not duplicated interactive controls hidden by CSS.
- Dialogs use Radix focus management and portal. Pending destructive/bulk work prevents accidental close; ordinary dialogs restore focus to their trigger. Confirm dialogs focus Cancel first.
- Inline notices persist for resource errors; Toast is reserved for transient operation results. Error sanitization returns operation/field context only for MCP configuration failures.
- Search text is built by explicit pure functions. MCP search never visits `env` or `headers` and no generic object serialization participates.
- Quick/JSON MCP editing maintains a canonical spec draft plus editable JSON text. Successful JSON parsing updates the draft; quick edits shallow-overlay known fields while preserving unknown keys. Parsing malformed env/header rows yields line numbers and prevents draft submission.

## Compatibility and rollback

- No command, Rust, database, route, or persisted schema changes are required. Existing Legacy screens continue using their current paths and the re-exported preset source.
- Rollback consists solely of reverting V2 feature/provider/port/UI/style/test/spec changes and returning Skills/MCP pages to null. Runtime data remains governed by existing backend backup and synchronization behavior.
