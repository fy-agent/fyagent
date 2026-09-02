# External Agent Catalog and Runtime Contract

## 1. Scope / Trigger

Read this contract before changing the static Agent catalog, runtime
observation, trusted launch destinations, renderer-visible capability modes,
or the Tauri permission set for those commands.

This contract owns product identity and read/launch projection only. It does
not own installation, update, Auth sessions, Skills, MCP, Models, or vendor
configuration. Use the linked feature contracts for those domains.

Primary evidence:

- `src-tauri/src/services/external_agents/**`
- `src-tauri/src/commands/agent_catalog.rs`
- `src-tauri/capabilities/**`
- `src/v2/shared/features/agents.ts`
- catalog/runtime tests under `tests/v2/**` and `src-tauri/src/services/external_agents/**`

## 2. Signatures

The native command boundary is closed and path-free:

```text
get_agent_catalog() -> AgentCatalogDto

get_external_agent_status({ agentId })
  -> {
       agentId,
       detected: boolean | null,
       running: boolean | null,
       version: string | null,
       installSource: closed enum | null,
       capabilities: runtime capability projection
     }

launch_external_agent({ agentId, destination })
  -> { agentId, destination, state, reasonCode }
```

Closed Agent IDs, in catalog order:

```text
qoderwork | trae-work | workbuddy | grokbuild |
codex | claude-code | opencode
```

`destination` is exactly:

```text
home | skills | hooks | models | mcp
```

The current static catalog contract version is `5`. Every product exposes the
same ordered capability IDs:

```text
product.open app.detect app.launch skills.read skills.write
hooks.read hooks.write models.validate models.write mcp.validate mcp.write
```

Capability mode/reason/evidence enums are closed in Rust and parsed strictly
at `src/v2/shared/features/agents.ts`.

## 3. Contracts

### Static catalog

- The catalog is deterministic and performs no filesystem, process, network,
  registry, database, or credential read.
- IDs, order, display names, variant IDs, official links, capability order and
  contract version are one code-owned table. The renderer must not merge it
  with a second local catalog or silently render an older fallback.
- Pi is not an Agent catalog product. Adding it to types, fixtures, UI or
  lifecycle requires a separately reviewed product decision and contract
  version.
- Official links are exact reviewed HTTPS links. The catalog never accepts a
  renderer URL or derives an install artifact from documentation links.
- Capability declarations describe reviewed FyAgent authority. They are not
  upgraded from local runtime observations or a successful browser/app handoff.

### Runtime observation

- `detected` and `running` are tri-state: `true`, `false`, or `null` when the
  current host/adaptor cannot authoritatively answer.
- Unknown does not become false. A missing settings/config directory is not
  evidence that a desktop app or CLI is absent.
- Version and install-source strings are sanitized and bounded. Absolute
  paths, registry keys, AUMIDs/PFNs, signer fingerprints, command lines and raw
  process details never cross the wire.
- Runtime observation reuses the product-specific trusted identity adapters.
  It does not scan arbitrary executables or let the renderer nominate a path.
- Catalog capability mode remains the static reviewed value even when a local
  status happens to be positive. Local detection is runtime evidence, not a
  new catalog guarantee.

### Trusted launch

- Launch accepts only a canonical Agent ID and closed destination.
- The backend resolves the destination to a reviewed product page, trusted
  application, or feature route. The renderer never supplies a URL,
  executable, bundle, command, argument vector, working directory or
  environment.
- A native application launch is positive only after a trusted bundle/PE/CLI
  identity adapter has admitted the target. Missing or ambiguous identity
  returns a controlled `unverified`/`unavailable` result and starts nothing.
- Windows desktop launch follows the Explorer-user boundary in
  [Windows Shell-user Runtime](./windows-runtime-security.md). macOS launch
  uses the reviewed bundle identity. Do not use generic shell execution.
- Opening an official page or vendor application proves only handoff. It does
  not prove install completion, Auth success, configuration acceptance or
  vendor UI state.

### Tauri permissions

- Catalog, status and launch commands remain separate permission entries.
- The application ACL manifests contain no remote origin and no generic
  filesystem/shell permission.
- Once an application capability manifest exists, the disjoint union of all
  active application permission sets must equal the complete registered
  `generate_handler!` command set. Adding one feature command without retaining
  unrelated compatibility commands silently revokes them.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Unknown/legacy Agent ID | Reject the request; do not map to another product. |
| Catalog version, product order, capability order or enum drifts | Strict Rust/TypeScript parser rejects the whole catalog. |
| Duplicate product/capability/link ID | Reject the catalog; do not deduplicate in the UI. |
| Official link is non-HTTPS, unexpected or malformed | Reject the catalog entry/catalog according to the strict parser. |
| Runtime adapter cannot answer | Return `null`/closed unknown reason; never manufacture `false`. |
| Renderer supplies path, URL, command, executable or extra field | Reject before filesystem/process/network side effect. |
| Launch identity is missing, ambiguous or untrusted | Controlled unavailable/unverified result; start nothing. |
| Browser/app launch completes | Report handoff/launch only; do not report installed, configured or authenticated. |
| Catalog/status/launch permission is omitted from ACL union | Permission contract test fails. |
| Pi appears in catalog/runtime/UI tests | Contract regression. |

## 5. Good / Base / Bad Cases

- **Good:** return catalog v5 in the exact seven-product order, then parse it
  once at the platform adapter before page rendering.
- **Good:** a runtime adapter cannot distinguish absent from inaccessible, so
  it returns `detected: null` and a closed reason instead of “not installed.”
- **Base:** a product can have static `app.detect = unverified` while a local
  trusted adapter reports one installation. Static capability review and local
  evidence answer different questions.
- **Bad:** scan `$PATH`/Program Files from React, infer install from
  `~/.vendor`, accept `{ path }` for launch, append a product only in the UI,
  or treat an opened official page as a completed action.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck:v2
mise run test:v2
```

Required assertion points:

- exact contract version, product order, capability order, link IDs and closed
  enums in Rust and `src/v2/shared/features/agents.ts`;
- unknown/excess fields, duplicate IDs and legacy/future versions fail closed;
- no Pi and no second renderer catalog;
- runtime unknown remains `null`, and sanitized DTOs contain no path,
  registry/process or credential material;
- launch rejects arbitrary URL/path/command fields and starts nothing without
  trusted identity;
- `tests/remainingPlatformSurface.test.ts` and permission-union tests cover
  the complete registered handler set, not only newly added commands;
- browser fixtures prove rendering only and never upgrade native detection or
  launch to verified HIL evidence.

## 7. Wrong vs Correct

Wrong:

```ts
const agents = [...nativeCatalog.agents, localStorageAgent];
await invoke("launch_external_agent", {
  agentId,
  path: form.executable,
});
```

Correct:

```ts
const catalog = parseAgentCatalog(await invoke("get_agent_catalog"));
const result = await invoke("launch_external_agent", {
  agentId: catalog.agents[0].id,
  destination: "home",
});
```

Only the backend resolves the closed ID and destination into trusted runtime
authority. The renderer parses and presents the result; it does not extend the
catalog or nominate execution targets.
