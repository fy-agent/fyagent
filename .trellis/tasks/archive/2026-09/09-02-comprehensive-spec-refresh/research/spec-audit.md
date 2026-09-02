# Trellis Spec Audit — 2026-09-02

## 1. Audit basis

This review is a full current-state audit, but not a blind rewrite. It uses the
completed 2026-08-31 audit as a verified baseline, then rechecks every current
spec against the repository at `d5826f37` and the changes made since that
baseline.

Current inventory before this task:

- 43 Markdown files under `.trellis/spec/`;
- approximately 15,500 lines;
- 26 backend files including the backend index;
- 14 frontend files including the frontend index;
- 3 thinking-guide files including the guide index.

Automated structural checks found:

- zero broken relative Markdown links;
- zero unindexed existing specs;
- zero template/TBD placeholders that should have been filled;
- zero substantial paragraphs duplicated verbatim across different specs;
- no dated override blocks that supersede contradictory body text;
- repository paths cited by current specs overwhelmingly still exist.

The structural result is important: the main problem is not widespread link
rot or abandoned templates. The current gaps are semantic ownership drift,
missing domain entry points, and three very large feature contracts whose
independent query domains are costly for an AI to retrieve as one unit.

## 2. Evidence and decision method

Every file was evaluated on six axes:

1. **Fact alignment** — named modules, commands, owners, paths, DTOs and tests
   must still exist or be explicitly described as compatibility surfaces.
2. **Authority** — one semantic owner per rule; orchestration specs link to the
   owner instead of restating its internals.
3. **Retrieval cost** — a reader changing one package or page should not need
   to load unrelated product domains.
4. **Executability** — code specs retain scope, signatures, contracts,
   validation/error behavior, examples, tests, and wrong/correct guidance.
5. **Lifecycle quality** — stable current rules stay in spec; run IDs, commit
   snapshots, investigation notes and transient rollout facts stay in tasks or
   history.
6. **Risk preservation** — security, secret, platform, rollback, signing,
   TOCTOU, readback and native-only negative cases are not shortened merely to
   reduce line count.

Repository evidence used for the semantic pass includes:

- `src-tauri/src/services/mod.rs` and
  `tests/architecture/rustModuleBoundaries.test.ts` for current Rust ownership;
- `src-tauri/src/agent_install/**` and the three Agent command modules for
  catalog/lifecycle/Auth boundaries;
- `src-tauri/src/database/{mod.rs,schema.rs,tests.rs}` for persistence and
  migration behavior;
- `src-tauri/src/proxy/**`, `src-tauri/src/services/proxy.rs`, and
  `src-tauri/src/commands/proxy.rs` for the proxy runtime;
- `src/v2/pages/{agents,models,skills,mcp}/**`, shared feature ports/UI, and
  their tests for the V2 feature split;
- `src/i18n/index.ts`, locale JSON files, and
  `tests/config/localeKeyParity.test.ts` for localization.

## 3. Material findings

### 3.1 Confirmed fact drift

`backend/modular-boundaries.md` still named the old Agent modules `auth` and
`source`, and described all transport as one readiness command module. Current
code uses `auth_actions`, `auth_sessions`, `sources/**`, and three separate
transport owners: catalog, Auth, and readiness/action. The same spec also used
“Proxy subdomains are private” too broadly: the private service helper is
`services/proxy::takeover`, while the top-level Proxy runtime intentionally has
crate-visible router/handler/provider/streaming modules.

### 3.2 Retrieval boundaries that are now justified

The following split candidates have direct one-way ownership evidence rather
than an arbitrary line-count threshold:

- backend Agent catalog/lifecycle, Auth, and configuration extensions map to
  separate command modules and private implementation modules;
- frontend Agent Directory and Models are separate pages with separate ports,
  state, actions and focused tests;
- frontend Skills and MCP are separate pages and mutation domains, while a
  smaller shared contract owns only common target identity and shared UI.

The old filenames remain as compatibility entry points so archived tasks and
existing links continue to resolve. Focused documents become the normal
reading targets; compatibility files retain only cross-domain invariants and
routing.

### 3.3 Missing first-class specs

Three important, independently owned domains lacked a focused entry point:

- SQLite lifecycle and schema migration safety;
- local proxy runtime/routing/takeover boundaries;
- current renderer localization resources and locale-key parity.

These are added as codebase-backed specs. Localization explicitly preserves the
current V2/leftover boundary instead of silently authorizing a V2 import from
the leftover `src/i18n` owner.

### 3.4 Long contracts intentionally retained

The following remain long because their ordered transaction, platform or
failure semantics are cohesive: Change Plan executor, Codex Provider writes,
Codex desktop installer, Windows runtime security, Windows installer, macOS
system commit, CI, Release, Task Runner, SecretRef, and WorkBuddy writes. Their
size alone is not evidence for another split.

## 4. Existing-file disposition

### Backend

| Existing spec | Decision | Reason |
| --- | --- | --- |
| `backend/index.md` | Update router | Route directly to new focused domains and distinguish compatibility maps from owners. |
| `backend/modular-boundaries.md` | Correct facts | Align Agent and Proxy module ownership with code and architecture tests. |
| `backend/reuse.md` | Retain | Current existing-owner/dependency/adaptor order is concise and stable. |
| `backend/development-environment.md` | Retain | Toolchain authority and host/cross-target gates remain cohesive. |
| `backend/development-hooks.md` | Retain | Optional hook and Trellis ownership scope remains current. |
| `backend/task-runner-contract.md` | Retain | Large but cohesive public task API and side-effect policy. |
| `backend/application-identity.md` | Retain | Stable product/identifier/license owner. |
| `backend/application-brand-assets.md` | Retain | Asset identity and byte-validation owner. |
| `backend/fyagent-version-contract.md` | Retain | Canonical application version and installer naming owner. |
| `backend/main-window-layout.md` | Retain | Native geometry and renderer chrome boundary. |
| `backend/secretref-backend.md` | Retain | Security-sensitive secret reference contract. |
| `backend/deeplink-import-security.md` | Retain | Untrusted input and import capability boundary. |
| `backend/change-plan-executor.md` | Retain | Typed execution/idempotency/compensation state machine is one safety owner. |
| `backend/codex-provider-configuration.md` | Retain | Ordered Provider/auth/live-file write and rollback owner. |
| `backend/codex-desktop-installer.md` | Retain | Installer/helper/bridge/signing transaction remains cohesive. |
| `backend/codex-session-usage.md` | Retain | Focused usage import and retry/log budget contract. |
| `backend/workbuddy-configuration.md` | Retain | Revision, overwrite, backup and reread owner. |
| `backend/external-agent-p0.md` | Convert to compatibility map | Preserve old links while moving independent Agent domains to focused specs. |
| `backend/windows-runtime-security.md` | Retain | Explorer-user/HKU/COM/helper security closure. |
| `backend/windows-installer.md` | Retain | NSIS/signing/uninstall transaction owner. |
| `backend/macos-system-commit.md` | Retain | Closed privileged helper protocol and enablement gate. |
| `backend/macos-dmg-layout.md` | Retain | DMG filesystem/Finder layout and validation owner. |
| `backend/github-ci-workflow.md` | Retain | CI classification, runner evidence and aggregate semantics are cohesive. |
| `backend/github-release-workflow.md` | Retain | Release transaction, signing, assets and recovery are cohesive. |
| `backend/github-merge-governance.md` | Retain | Merge Queue and task/spec lifecycle governance owner. |
| `backend/upstream-sync.md` | Retain | Immutable upstream identity and ancestry-preserving merge owner. |

### Frontend

| Existing spec | Decision | Reason |
| --- | --- | --- |
| `frontend/index.md` | Update router | Route to focused Agent, Models, Skills, MCP and localization contracts. |
| `frontend/directory-structure.md` | Retain | Directory roles and placement remain current. |
| `frontend/modular-boundaries.md` | Retain | Renderer/host, V2/leftover and feature/platform boundaries remain current. |
| `frontend/type-safety.md` | Retain | Unknown-input parsing and exhaustive-union rules remain current. |
| `frontend/state-management.md` | Retain | Server/URL/draft/secret ownership remains current. |
| `frontend/reuse.md` | Retain | Shared owner registry and anti-clone rules remain current. |
| `frontend/component-guidelines.md` | Retain | Component semantics/accessibility/composition contract remains current. |
| `frontend/hook-guidelines.md` | Retain | Hook lifecycle and effect/query ownership remains current. |
| `frontend/quality-guidelines.md` | Retain | Test layering and deterministic evidence rules remain current. |
| `frontend/user-facing-copy.md` | Retain | Evidence-strength and human copy owner remains current. |
| `frontend/v2-shell.md` | Retain | Route/nav/window/motion lifecycle remains one shell owner. |
| `frontend/v2-agent-models.md` | Convert to compatibility/shared map | Preserve cross-domain catalog shell while routing page behavior separately. |
| `frontend/v2-skills-mcp.md` | Convert to compatibility/shared map | Preserve shared target/UI contract while routing mutations separately. |
| `frontend/v2-prompts-memory.md` | Retain | Focused Prompt/Memory ports and Agent delegation contract. |

### Thinking guides

| Existing guide | Decision | Reason |
| --- | --- | --- |
| `guides/index.md` | Retain | Already a compact router. |
| `guides/code-reuse-thinking-guide.md` | Retain | Decision checklist, not a duplicate code contract. |
| `guides/cross-layer-thinking-guide.md` | Retain | Cross-layer authority/data-flow checklist remains compact. |

## 5. New focused documents

Backend:

- `external-agent-catalog-lifecycle.md`;
- `external-agent-auth.md`;
- `external-agent-configuration.md`;
- `persistence-and-migrations.md`;
- `proxy-runtime.md`.

Frontend:

- `v2-agent-directory.md`;
- `v2-models.md`;
- `v2-skills.md`;
- `v2-mcp.md`;
- `localization.md`.

## 6. Validation plan

The implementation is accepted only when all of the following hold:

1. every relative Markdown link resolves;
2. every non-index spec is reachable from its layer index;
3. every focused code spec has the seven executable sections required by the
   project skill;
4. compatibility maps identify the focused semantic owner and do not retain a
   second conflicting contract;
5. repository paths and named source/test owners in changed specs exist;
6. no placeholder, dated override, temporary run evidence, or copied tool
   version is introduced;
7. `git diff --check`, Trellis context validation, focused architecture/i18n
   tests, and `mise run check:contracts` pass;
8. the final diff changes only `.trellis/spec/**` and this Trellis task.
