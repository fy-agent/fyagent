# Implementation plan

## 1. Backend page contract

- Add `DiscoverableSkillsPage`, limit clamp, query/repo/status filter, directory-tail install match, and in-process discovery cache on `SkillService`.
- Add `discover_available_skills_page`; keep `discover_available_skills` returning the full cached scan.
- Invalidate cache from `add_skill_repo` / `remove_skill_repo`.
- Register the command and allow-list it.
- Add Rust unit tests for clamp, filter, install match, and out-of-range offset.

## 2. V2 ports and queries

- Replace `SkillsPort.discover` with `discoverPage`.
- Wire Tauri/browser adapters, query key prefix, `keepPreviousData`.
- Update feature port tests and browser IPC fixture.

## 3. Shared scroll and pagination chrome

- Extract `.fy-feature-discovery-scroll` and switch MCP + Skills discovery to it.
- Add `FeaturePagination` and use it for both Skills discovery sources.

## 4. Skills Discovery page

- Server-backed repo pages; debounce search; repo tabs from enabled repos; reset page on filter change.
- Keep skills.sh paging; do not fetch repo pages on that source.

## 5. Spec

- Update `.trellis/spec/frontend/v2-skills-mcp.md` (and reuse/index pointers if the new chrome needs them).

## Validation

```bash
mise run rust:fmt:check
mise run rust:test
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run format:check
```

If time allows: `mise run test:v2:browser`.

## Rollback

Revert the task branch/files. Do not migrate data; cache is process-local.
