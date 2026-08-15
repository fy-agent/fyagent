# Issue #35 immutable product rereview

`PRODUCT_REREVIEW=APPROVE`

- Authority commit: `f2f26b8b6b5aa4acf8bbd257cee9ee22713aebaf`
- Branch: `codex/issue-35-secret-backend`
- Evidence: `static_design`
- Blocking findings: `P0=0, P1=0, P2=0`
- Decision: commit D is product-freezable. This receipt does not claim implementation, runtime, UAT, native-platform, integration, merge, or production completion.

## Review scope

The rereview used only `git show D:<path>` blobs from the immutable authority commit. It covered the Issue authority, PRD, product design, technical and detailed design, contract, device-local store, secret-surface inventory, Codex call graph, downstream handoff, execution plan, source audit, OS-keyring options, native evidence plan, and runtime preflight.

The product decision specifically covers:

- the `secretRef` plus device-local OS-keyring MVP, with no secret material in SQLite, public DTOs, IPC, logs, sync, export, or backup;
- the registered-hardware adapter contract as a later capability, including hidden-until-registered behavior, exact confirmation policy, and no fallback to OS keyring or inline settings;
- the Codex-first slice, with Agent and adjacent credential classes explicitly rejected or recorded as follow-up debt rather than included in a repository-global migration claim;
- capture, replace, migrate, rotate, validate, lock/unlock, delete, revoke, missing, denied, unavailable, legacy conflict, cleanup, and crash-resume states and user actions;
- the closed no-value boundary, including the opaque eleven-domain legacy-coverage receipt and fresh coverage checks at startup, summary, capture, and Provider delete preview/confirm;
- truthful evidence boundaries, including Windows native evidence as a blocker for `DONE`, not as evidence already supplied by this design freeze.

## Out of scope

- Code implementation, schema registration, command wiring, runtime behavior, tests, build, dependency resolution, browser or native execution, screenshots, UAT, and production evidence.
- Hardware adapter implementation, non-Codex credential families, historical artifact rewriting, repository-global migration, and automatic live-target apply during secret activation.
- Downstream integration/source freeze before compatible immutable #55 and #41 successor authorities exist and main-integration ownership is resolved.
- macOS and Windows Rust 1.85 `--all-targets` evidence, named CRUD/failure matrices, packaging, and host provisioning required by the later `DONE` gate.

## Key user-flow closure

1. Capture/import/replace creates a backend-verified candidate without exposing a value. The user reviews a token-free #55 plan; #41 then owns pre-confirmation, Provider lease, final baseline, structural backup, writer/readback, and rollback boundaries.
2. Activation binds and scrubs only after fresh exact legacy inventory and CAS checks. Applying the active secret to a live target remains a separate approved plan; activation never silently changes live configuration.
3. Runtime proxy, usage/balance, primary coding-plan, and model-fetch paths resolve only at their final owner-private send boundary. Secret readiness or resolve failure is terminal, network-free, redirect-free, and failover-neutral.
4. Rotation activates the new binding before separately deleting and fresh-confirming absence of the old record. A cleanup failure becomes typed `activatedCleanupPending`; it never rolls the binding back to the old record.
5. User delete, external missing, central/device revoke, logical lock, backend lock, permission denial, and backend unavailable remain distinct states with deterministic actions. Recovery uses typed kind-specific journals and CAS; generic retry or caller-selected recovery is absent.
6. Startup/import resume remains fail-closed. Staged resume accepts exactly `stageId + expectedResumeCas` and returns the independent exact five-field result in every arm; terminal rows carry `issue=null` and recovery rows remain typed.

## Downstream-consumable boundaries

- **#55 Change Plan:** consumes only the closed no-value secret projection and owns admission, digest/comparison policy, role/sink planning, and staged projection. The baseline authority named in D is not treated as compatible implementation evidence; a compatible immutable successor is required before integration/source freeze.
- **#41 configuration apply:** consumes prepared target/rollback capabilities through its pre-confirmation, Provider lease, final-baseline, backup, writer/readback, and rollback boundary. It does not receive secret material, backend locators, or a second ledger. Its compatible successor remains a later integration gate.
- **#63 / main integration:** owns shared Provider/startup/import/proxy registration and the sole complete eleven-domain legacy inventory bridge. It consumes named closed DTOs/receipts and registers the exact fifteen #35 commands plus the separately typed staged-resume handler; it cannot mint backend authority or bypass the startup gate.
- **#35 core:** owns the device-local authority, backend registry/broker, candidate and recovery state machines, and no-value public contract. It adds no SQLite schema version and does not claim downstream wiring as complete.

## Evidence truth

This approval is `static_design` only. The authority consistently leaves implementation, compatible downstream SHA handoffs, source-freeze ownership, native macOS/Windows execution, full command registration, failure-case evidence, UAT, and production as future gates. In particular, absence of the named Windows evidence blocks `DONE` even though it does not block this product design freeze.

## Authority SHA-256 snapshot

| D path | SHA-256 |
| --- | --- |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/issue-35-authority.md` | `365188c6be12092a5e535ba300800eb69918599dbeada824a7a033aecabc8f33` |
| `.trellis/tasks/08-14-issue-35-secret-backend/prd.md` | `1b1c957d414a4506618ba18a998bd9c2f032d529bfb10aca34edff55064da7fc` |
| `.trellis/tasks/08-14-issue-35-secret-backend/design.md` | `2fbcc56cbbbc5a61257c867e7c2dd3502e1518d00273d49fa5fb9fcf5bd71f05` |
| `.trellis/tasks/08-14-issue-35-secret-backend/technical-design-overview.md` | `21bedd66af5a5136125f9d654dabccdbeed8bf6ca6cf269638f836c8e70d6956` |
| `.trellis/tasks/08-14-issue-35-secret-backend/detailed-design-overview.md` | `40681c7b4e0d9522d56e11293275b2a4f309abe28a86483d9c8faa876c04d51c` |
| `.trellis/tasks/08-14-issue-35-secret-backend/secret-contract-v1.md` | `29a64c81554c205196140860a30def14835ac2e54f445ad5d739e214025369bf` |
| `.trellis/tasks/08-14-issue-35-secret-backend/device-local-secret-store.md` | `681947575da8a4a4ccad827a3aa3010bbc4cda828bab6e3c7a6a6124eff2ad7e` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/secret-surface-inventory.md` | `dfa299542460f3aa62fa353f4af575ac7e194c72aab6411a6f868d8c87743ea1` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/codex-secret-call-graph.md` | `3ec8e7b67ff16a1b93e2af79857bd977e4ab3db4fcdcd9079c70fdc7ad8511b4` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/secretRef-contract-handoff.md` | `3c2f72d246d7df20c6167505d41c46e2003f624b00af013d561914e26f79d34f` |
| `.trellis/tasks/08-14-issue-35-secret-backend/execution-plan.md` | `3801eae08742d359a74dc211d011ce73dd8922a765750ec7113766945a647e9b` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/source-audit.md` | `487acfdb858a05f716ec5faa4d4850ea24a38b7cc1452c2b041eee092c79b861` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/os-keyring-options.md` | `baf55f60c30d45cd8f9e83b8bcc06d1d8e5fec33b2ddca428ba308a32372fe1c` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/native-evidence-plan.md` | `dfc4f77fbf3079f7ec089546da3d980825a584d852c01f368ed175e65c5fcec4` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/runtime-preflight.md` | `62848518fafce39ad33040c10192ee0092cd61f7ec7235f7a929c00f472aa39d` |
