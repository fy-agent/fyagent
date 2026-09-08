# Curated Owning-Spec Constraints

Date: 2026-08-31

Purpose: provide a non-truncated execution/check context for the relevant sections of the large owning specs. This is a task-scoped extraction, not a replacement for the full specs. Phase 0 must reread the live owning specs before code changes.

Sources reviewed:

- `.trellis/spec/backend/external-agent-p0.md`
- `.trellis/spec/backend/codex-desktop-installer.md`
- `.trellis/spec/frontend/v2-agent-models.md`

## 1. External Agent closed-contract constraints

### Stable product IDs

The Agent Catalog is a fixed versioned set in this order:

```text
qoderwork
trae-work
workbuddy
grokbuild
codex
claude-code
opencode
```

Do not create an eighth Claude Desktop product. Surface changes happen beneath the existing product ID.

### Renderer request authority

Lifecycle requests contain only:

```text
agentId
closed action
optional complete opaque inventoryId/targetId/expectedTargetRevision triplet
optional opaque expectedReleaseId
closed surface when the contract requires it
```

Forbidden renderer fields include:

```text
URL
raw path
command/script
token/secret
hash
package format
native identity/publisher
bypass/force
```

Serde/parser boundaries reject unknown fields. A partial target triplet fails closed.

### Inventory authority

`agent_install/inventory.rs` is the single owner of:

- candidate normalization and evidence merge;
- opaque inventory/candidate/destination IDs;
- target revisions and expiry;
- ambiguity/multiple-install selection;
- stale target revalidation;
- effective action eligibility.

Platform/source adapters emit evidence and closed policies; they do not mint renderer capabilities or choose the first candidate.

### Action authority

Before any launch/write, the backend re-enumerates and compares the selected opaque target/revision. Missing, expired, scope-changed, owner-changed, identity-changed or revision-changed targets authorize zero side effects.

This task additionally requires product action policy admission before target/network work.

### Existing owner boundaries

- Codex install/update remains on its dedicated desktop installer/job slot.
- Managed desktop Agents reuse shared executable-installer infrastructure.
- Tooling remains the owner of generic CLI install/probe/session behavior outside the Agent lifecycle surface.
- Removing Claude/OpenCode Agent CLI surfaces is not permission to delete unrelated Tooling/Provider/Skills/MCP/model/session behavior.

## 2. Executable installer constraints

### One shared executable-software owner

All one-click executable installers reuse the existing source/download/job/cancel/temp/install/post-install orchestration. Product adapters do not grow a second downloader or package transaction.

### Fixed-source boundary

- Product endpoints are Rust-owned fixed enums/constants.
- Metadata-provided URLs, filenames, redirects or mirrors never become caller capabilities.
- HTTPS/redirect allowlists, retry, timeout, cancellation and body/size caps remain active.
- Renderer/helper APIs accept no arbitrary source or destination.

### Publication/content fields are not admission

The current owning contract prohibits executable admission based on remote/manifest comparisons such as:

- SHA/checksum;
- remote/manifest/Content-Length equality;
- remote package identity/publisher/Team ID;
- remote version/architecture/minimum OS;
- remote signature/notarization/Gatekeeper claims.

`Content-Length` may remain a progress/disk hint. A locally computed digest may bind an existing same-I/O bridge where the current owner already requires it, but no second full-file hash pass is added.

Managed Agent DMGs retain the narrower closed local product-routing gate: after a fixed first-party/reviewed DMG is mounted, exactly one direct top-level app must match the code-owned Bundle ID and reviewed local version source. This routes a product action to the intended local bundle; it does not turn mirror metadata into package admission.

### macOS DMG transaction

The existing owner retains:

- read-only `hdiutil` mount;
- exactly one direct top-level `.app`;
- generated same-volume staging and backup names;
- exact target selection;
- app-running checks;
- atomic/compensating replacement;
- rollback/recovery-required states;
- exact expected-replacement cleanup;
- post-install existence/version/runnable readback;
- bounded detach/temp cleanup.

Claude/OpenCode must delegate to this transaction. Do not copy mount/stage/rename/restore logic or call an upstream updater.

### Helper boundary

`/Applications` system commit remains a separate closed helper owner. This task must not introduce:

- sudo;
- administrator AppleScript;
- arbitrary root XPC/file manager;
- renderer path/URL/command;
- product-specific helper;
- silent `~/Applications` fallback reported as system success.

System acceptance requires the helper task’s signed/notarized HIL.

### Success and launch

- Install/update success requires authoritative post-install reread.
- Install/update does not imply launch.
- “打开软件” is a separate explicit user action against a freshly revalidated backend candidate.

## 3. V2 Agent Directory constraints

### Canonical catalog and local directory

The backend catalog owns product identity, official links and capability declarations. `PRODUCT_DIRECTORY` owns local assets/renderer metadata. A runtime sort may project entries but must not create another product registry or mutate canonical catalog order.

### Scan truth

The directory renders all supported products, including not-installed and unresolved rows. It must not filter to installed products only or manufacture installed state after an action without rereading readiness.

### Installer truth

Agent cards/detail use the authoritative backend readiness/inventory/action facade. Browser preview cannot impersonate desktop installation success.

The frontend:

- parses exact versioned wire shapes;
- renders backend `allowedActions` and reason codes;
- never constructs download URLs/paths/commands;
- does not create product-specific independent mutation paths;
- rereads readiness after terminal action results.

### Product-domain asymmetry

Agent directory/catalog, Models, Skills and MCP share IDs/geometry but retain separate workflows. A lifecycle surface change must not fold Provider/model/Skills/MCP behavior into the installer or delete those owners.

### Reuse and copy

- shared lifecycle/progress/target picker components remain the common UI owner;
- new cross-product UI behavior belongs in shared feature/page projection, not copied card branches;
- primary copy is concise and user-facing; internal diagnostic prose does not leak into visible cards.

## 4. Task-specific implications

1. The product policy matrix must be exhaustive in Rust and mirrored by one strict TypeScript surface parser.
2. Qoder/TRAE/WorkBuddy no-update policy must affect backend actions and inventory, not only React.
3. OpenCode/Claude Agent CLI removal must not delete other product domains.
4. Claude mirror fields select no arbitrary URL and add no remote hash/publisher admission.
5. OpenCode GitHub metadata is fixed-repository release selection; artifact remains the fixed stable endpoint.
6. Directory sort is a pure projection over current scan evidence and existing product metadata.
7. Every install/update ends with the existing authoritative readiness/inventory reread.

## 5. Required live-spec reread

Before implementation and again during final check, inspect the current live sections governing:

```text
Agent Catalog version/order/links
surface and reason enums
opaque request/inventory/action shapes
managed desktop source mappings
executable installer source/download/DMG/helper rules
V2 Agent directory/card/install projections
```

If the full live specs changed after this extraction, the live owning spec wins. Update this task’s design/plan only if the product behavior or acceptance contract materially changes.

