# Execution Context

This is the compact stable context for implementation/check agents. Read the full PRD/design for details; do not infer behavior from intermediate predecessor code.

## Product matrix

```text
QoderWork Desktop: install + launch, no FyAgent update
TRAE Work Desktop: install + launch, no FyAgent update
WorkBuddy Desktop: install + launch, no FyAgent update
Grok Build CLI: unchanged
Codex Desktop: unchanged dedicated owner
Claude Code product ID: Agent lifecycle Desktop only; install/update/launch Claude.app
OpenCode product ID: Agent lifecycle Desktop only; install/update/launch OpenCode.app
```

Removing CLI means removing the Agent lifecycle/download surface and CLI official link. Do not delete Provider, Skills, MCP, models, session or user-existing CLI identities without separate evidence.

## Ordering contract

Canonical order remains unchanged globally.

After a complete scan:

```text
installed domestic
installed other
unresolved
confirmed not installed
```

Within each bucket retain canonical order. Initial scan stays canonical; rescan freezes previous committed order. `installed_not_runnable` is installed. Current failure is unresolved for ordering even if stale installed data remains visible/configurable.

Domestic priority must be one field in existing `PRODUCT_DIRECTORY`, not a page-local list.

## Backend policy contract

One crate-private product/surface/action policy owner must drive:

- legal/default surfaces;
- readiness actions/update state;
- inventory action eligibility;
- source-resolution conditions;
- action admission.

Chinese-product update and Claude/OpenCode CLI actions are rejected before target validation, network, helper or filesystem side effects. Use `action_not_supported` for policy-disabled update and `surface_not_supported` for removed CLI surface.

## Claude source contract

```text
manifest: https://claudeapp.agentsmirror.com/latest/manifest
artifact: https://claudeapp.agentsmirror.com/latest/mac
official page: https://claude.com/download
bundle ID: com.anthropic.claudefordesktop
package: universal DMG
version: Info.plist exact
```

Parse only bounded manifest v2 `sources.macos.universal` flow fields. Remote URL/hash/filename is not a capability or executable admission. Reuse Codex fixed-manifest HTTP/retry/cache/cancel patterns and the existing shared downloader/DMG transaction. Do not copy mirror scripts.

Do not claim that the mirror enables Claude service/account access in China mainland.

## OpenCode source contract

Use a fixed official GitHub latest-version owner for `anomalyco/opencode` plus existing fixed stable DMG endpoints. Require mounted app version to match the frozen descriptor. Do not invoke Electron updater.

## Existing owners to preserve

- Agent inventory opaque targets/revisions;
- shared streamed artifact downloader;
- shared lifecycle job/progress/terminal state;
- Codex-tested managed DMG mount/staging/rollback;
- shared explicit desktop launcher;
- separate `MacSystemCommitPort` helper task for `/Applications`.

## Execution order

1. Re-audit after predecessor settles.
2. Backend lifecycle policy.
3. Readiness/inventory install-only behavior.
4. Claude/OpenCode desktop-only surface/catalog.
5. Stable directory ordering.
6. Claude fixed source.
7. OpenCode metadata/update.
8. Frontend integration.
9. Signed helper HIL/specs/review.

## Stop conditions

- predecessor interface still changing;
- Claude mirror identity/provenance/schema drift;
- implementation requires arbitrary source/path/command;
- a second downloader/updater/DMG/helper begins forming;
- `/Applications` claim lacks signed helper HIL;
- non-lifecycle Claude/OpenCode domains would need broad deletion;
- Windows desktop work begins expanding.

