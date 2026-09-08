# Reuse Decision Record

Date: 2026-08-31

## 1. Decision order applied

```text
existing FyAgent owner
  -> already-adopted framework/crate
  -> maintained upstream/open-source evidence
  -> one narrow FyAgent adapter
  -> bespoke implementation only with a proven gap
```

No new Cargo, npm or Swift dependency is selected by this task.

## 2. Capability-by-capability decision

| Need | Existing/reviewed owner | Decision | Why |
| --- | --- | --- | --- |
| Product IDs and canonical order | backend Agent Catalog + frontend `PRODUCT_DIRECTORY` | Reuse | Stable cross-feature identity already exists. |
| Domestic priority metadata | `PRODUCT_DIRECTORY` | Extend once | This is Agent directory presentation metadata; a page-local ID set would duplicate it. |
| Scan truth/current failure/stale result | `agentDirectoryScanProjection` + scan reducer | Extend | Existing state already distinguishes all required evidence classes. |
| Dynamic ordering | none | Add one pure projection | Small missing capability; no package is justified. |
| Lifecycle action policy | existing Agent install domain, currently distributed matches | Consolidate behind one crate-private facade | Removes drift without creating a new framework. |
| Closed request/opaque targets | Agent install types/inventory | Reuse | Already prevents renderer paths and stale targets. |
| HTTP client/TLS/retry/cancel | existing Codex/Agent HTTP owners | Reuse/extract narrowly | A second client stack would duplicate core safety. |
| Claude mirror synchronization | `Wangnov/claude-app-mirror` service/repository | Reuse fixed service endpoints and provenance model only | Its purpose is exactly unchanged official installer mirroring; client scripts are not needed. |
| Claude manifest parsing | Codex fixed-manifest source pattern | Adapt narrowly | Same fixed-endpoint/private-schema problem. |
| OpenCode latest version | existing fixed GitHub latest-version owner used by Tooling | Reuse or minimally extract | Avoid a second GitHub API client. |
| Artifact streaming/progress | Codex shared downloader + Agent job transfer | Reuse | Already supports cancellation, retry, `.part`, limits and progress. |
| DMG mount/single-app/staging/rollback | Codex macOS managed transaction | Reuse | Transaction ordering is a safety property; do not fork. |
| `/Applications` authorization | separate `MacSystemCommitPort` helper task | Delegate | Privilege must have one owner. |
| Launch | existing desktop/process launch owner | Reuse | One explicit “打开软件” path already exists. |
| Percent/bytes/s/terminal state | shared lifecycle projection from predecessor | Reuse | Do not create product-specific progress UI. |

## 3. Open-source review

### 3.1 `Wangnov/claude-app-mirror`

```text
License: MIT
Reviewed use: fixed manifest/artifact endpoints and synchronization provenance
Not adopted: workflow scripts, shell install commands, generic proxy behavior
```

Reasons to use:

- narrow project scope;
- unchanged official installer bytes by documented design;
- current macOS universal artifact;
- current fixed latest endpoint;
- manifest records upstream endpoint/version/provenance;
- same AgentsMirror operational family already used for Codex.

Containment requirements:

- product-specific hard-coded endpoint enums;
- bounded schema parser;
- no renderer URL;
- no automatic trust of remote URL/hash/publisher fields;
- local managed bundle routing and native install/readback remain authoritative;
- re-review on repository/endpoint/behavior drift.

### 3.2 OpenCode upstream Electron updater

```text
Decision: reject as an execution dependency; use as release-structure evidence only.
```

Reasons:

- it owns a different target selection and rollback model;
- invoking it would bypass FyAgent job progress and cancellation;
- it does not integrate with FyAgent’s opaque inventory target;
- it cannot substitute the separate `/Applications` helper contract;
- it would create two update authorities for the same app.

### 3.3 Generic GitHub proxies/Homebrew installers

```text
Decision: reject.
```

- A public proxy adds another unaudited trust/availability dependency.
- Homebrew is not guaranteed to be installed and would change installation ownership.
- Neither is necessary because fixed official/mirror DMGs exist and the shared downloader already solves transport.

### 3.4 Sorting libraries

```text
Decision: no dependency.
```

The order is a four-bucket stable projection over seven entries. A small pure rank function using standard JavaScript `sort` on a copied array is clearer and lower risk than a package. This is not “reinventing a sorting algorithm”; it is expressing product policy while the runtime supplies the sorting primitive.

## 4. Existing hand-written code to replace or converge

The task should remove/converge duplication already in the repository where it directly causes the requested behavior:

### Product/surface/action matches

Current product matches are distributed across:

- Rust legal surfaces/default surface;
- inventory probe routing;
- readiness routing;
- action dispatcher;
- TypeScript legal surface map.

Backend semantics should converge behind one policy owner. TypeScript retains one strict wire-contract mirror because Rust/TypeScript cannot share the same compiled enum, and cross-layer tests keep them equal.

### OpenCode dual-surface projection

Delete the now-obsolete dual-surface aggregation after desktop-only policy is active. Do not leave dead CLI fields as a compatibility fallback.

### Product-specific UI suppression

Do not add `if product in [three IDs] hide update` in React. Remove/update any such local logic and let backend `allowedActions` drive the generic component.

## 5. New local code justified

Only these project-specific adapters are justified:

1. `AgentLifecyclePolicy` facade/table—FyAgent product policy has no external package equivalent.
2. Agent directory bucket projection—seven-product user experience policy has no reusable package semantics.
3. Claude private manifest DTO/product source adapter—maps one reviewed external schema into the existing FyAgent release descriptor.

Each remains narrow, private/crate-scoped and covered by exhaustive tests.

## 6. Dependency decision

```text
New runtime dependencies: none planned
New development dependencies: none planned
External service endpoint: one reviewed Claude product mirror
```

If implementation discovers a capability gap requiring a package, it must pause for a fresh dependency review covering license, maintenance, advisories, macOS 12, toolchain compatibility, transitive footprint and overlap with current crates. The task does not pre-authorize any dependency.

## 7. Negative acceptance scans

Before merge, search for and review:

```text
new reqwest/client construction under Claude/OpenCode adapters
new hdiutil/ditto/copy/rename transaction outside current owner
new app opener
new arbitrary URL/path/command fields
OpenCode/Claude CLI in Agent lifecycle surface tables
Qoder/TRAE/WorkBuddy update allowlists
page-local domestic ID arrays
product-specific progress/speed formatters
electron-updater invocation
claude-app-mirror shell/workflow code copied into runtime
```

