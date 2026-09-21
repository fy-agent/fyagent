> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Session build integration

## Changes

- `config/vite.config.ts`: retained dependency-aware named vendor groups. Removed route-only Radix Dialog, Popover, Select and Tabs entries from the startup Radix group so Rollup follows their actual lazy consumers. No catch-all dependency partition, dependency or lockfile change; all feature code remains available.
- `tests/renderer/widgets/app-shell/SideNavigation.test.tsx`: asserts Session among the nine primary route leaves, Memory as the tenth auxiliary route, and keyboard order through both; retained Memory active-state/lens checks.
- `tests/renderer/scripts/verify-route-chunks.test.ts`: ten separate lazy routes, positive Session route and deferred port presence, unchanged negative and budget checks.
- Did not change feature composition, DTOs, Session page, budget constants, or the coordinator's ten-entry verifier lists.

## Cause and evidence

Session and its native port were already outside the static initial graph. Shared Zod/mini code gained by the new schemas was allocated to the existing initial shared module. The old manual Radix group independently forced route-only controls into startup: all eight Radix package entry points shared one eager chunk. Narrowing that group removed this existing eager-loading overhead without changing schema validation or business behavior.

The recorded failing build had 670259 initial JS bytes against the unchanged 665600-byte budget. The new production build has 637815 initial JS bytes (32444 bytes less, 27785 bytes below budget), 46775 initial CSS bytes, ten route chunks and ten bootstrap-deferred ports plus the existing nested subscription port. Radix startup chunk decreased from 85302 to 52858 bytes. The generated Dialog and FeatureTabs chunks are not in the initial static closure.

## Validation

- `rtk proxy mise run build:renderer`: PASS, Vite 7.3.6; route chunk graph and unchanged budgets passed.
- `rtk proxy mise run test:unit tests/renderer/widgets/app-shell/SideNavigation.test.tsx tests/renderer/scripts/verify-route-chunks.test.ts`: PASS, 2 files / 33 tests after the final test edit.
- Focused files formatted through `mise run format:files`; no global formatting while other writers are active.

## Integration follow-up

The coordinator/QA must still run production browser boot on the integrated tree: a successful build and manifest do not establish browser initialization. This worker deliberately did not run concurrent browser builds or repeat the entire test suite. Update frontend security/quality/navigation specs' historical nine-route wording when documenting the final Session route scope. No commit or push was made.
