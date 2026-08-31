# Planning Review Rounds

Date: 2026-08-31

## Round 1 — Architecture and reuse

### Questions

- Is a helper actually necessary after the Apple-native authorization spike?
- Which existing FyAgent owners must remain authoritative?
- Which open-source components remove real wheel-reinvention?
- Should the task use SMJobBless, SMAppService, or both?

### Findings

1. The sibling task's signed/API spike found no sufficient public authorized-FileManager path for fresh absent-target app copy; automatic system commit remains unavailable without a helper.
2. FyAgent already owns download, DMG mount, product identity, target capability, job, rollback semantics and readback. Rewriting those would be the largest architecture regression.
3. Blessed removes direct SMJobBless/requirement diagnosis work; SecureXPC removes custom RPC/audit-token/FD-transfer work; Authorized removes raw Authorization Services wrapper work.
4. `SMAppService` is macOS 13+, while FyAgent supports 12. Dual production registrars would double state/layout/HIL.
5. Swift packages cannot be consumed as Cargo crates; a narrow in-process Swift client bridge is needed.

### Decisions

- helper confirmed as a separate task;
- one `MacSystemCommitPort` seam;
- Blessed/SMJobBless single registrar for initial macOS 12+ implementation;
- SecureXPC + Authorized;
- main-process Swift library bridge, not executable client sidecar;
- future SMAppService migration isolated behind registrar interface.

### Result

Pass after updating design to forbid a second installer and runtime registrar fallback.

## Round 2 — Adversarial privilege review

### Questions

- What can a same-user attacker do with a public Mach service?
- What if a user-writable source path changes after validation?
- What if the FyAgent process itself is partially exploited?
- Is helper installation authorization enough for later mutations?
- Can sample/Mist command routes be reused safely?

### Findings

1. Mach service name and PID are not identity; actual peer code-signing validation is required.
2. Passing a source path to root creates a classic TOCTOU opportunity. SecureXPC already supports FD transfer, so a path protocol is unnecessary.
3. A signed app exploit could call every private route; helper operations must remain independently finite and require fresh user authorization.
4. SMJobBless authorization installs/updates the daemon. It does not prove user intent for every later app replacement.
5. Mist and sample helper business APIs are intentionally broad (paths/commands). Copying them would create a general root primitive.
6. Multi-call root file operations create crash/interleaving gaps; one request must own the full transaction.

### Decisions

- source directory FD + fd-relative revalidation;
- target slot enum mapped inside helper;
- per-mutation Authorization Services custom right, rechecked in helper;
- mutual SecureXPC requirements; PID diagnostic only;
- no path/URL/command/process/network fields;
- single root transaction with receipt/recovery;
- sample projects reference only.

### Result

Pass after adding threat model, forbidden-capability matrix and crash recovery gate.

## Round 3 — Dependency and maintenance review

### Questions

- Are selected packages maintained enough for privileged code?
- Which exact versions should be pinned?
- Do transitive dependencies and licenses fit the project?
- Is there a better modern drop-in helper implementation?

### Findings

1. Blessed/SecureXPC/Authorized have MIT licenses and small auditable source, but core repositories are stable/lightly maintained rather than rapidly active.
2. Active Mist commit from 2026 still uses Blessed >=0.6.0 and SecureXPC >=0.8.0 on macOS 12+, providing current integration evidence.
3. SecureXPC 0.8.0 already has FD wrappers, SMJobBless/SMAppService criteria and message-based code-sign validation. Unreleased later commits add hardened-runtime defaults and reliability/path fixes.
4. Modern SMAppService samples are 13+ and often expose root shell commands. They do not solve this task's compatibility or business boundary.
5. No reviewed project provides a reusable known-FyAgent-app transaction; that policy must remain local and small.

### Decisions

- Blessed exact `0.6.0` candidate;
- Authorized exact `1.0.0` candidate;
- SecureXPC exact reviewed revision candidate, upgraded to a newer exact tag if upstream has released equivalent fixes at implementation time;
- commit `Package.resolved`, official-repo allowlist, license/NOTICE and source-only builds;
- no additional helper framework/dependency.

### Result

Pass with a mandatory implementation-time dependency refresh and local source audit.

## Round 4 — Release, update and operations review

### Questions

- Can the current Tauri release pipeline correctly embed/sign/notarize a helper?
- How are helper versions upgraded and downgrades blocked?
- What happens if helper/app is removed, disabled or crashes?
- When may production system actions be enabled?

### Findings

1. Current scripts sign/verify only the top-level app; nested Swift client/helper need explicit inside-out signing and verification.
2. Helper is copied outside the app bundle, so universal architecture and runtime linkage must work from `/Library/PrivilegedHelperTools`.
3. SMJobBless refuses equal/lower CFBundleVersion replacement. Source-hash auto-increment is not reproducible and would dirty the workspace.
4. App release version can be the monotonic helper version; helper security changes require an app version bump.
5. Apple has no simple legacy uninstall API; explicit helper self-removal is required if product wants cleanup.
6. Portable tests cannot prove Developer ID requirements, launchd root execution, admin auth, APFS transaction or notarization.

### Decisions

- universal Swift build integrated into existing formal release flow;
- exact nested sign order and explicit verifier;
- helper CFBundleVersion/minimum client version tied to formal app release version;
- explicit helper health/update/removal states;
- root-private recovery receipts and fault injection;
- formal signed/notarized HIL is the only production enablement gate;
- system target stays disabled on unsigned/debug/incompatible builds.

### Result

Pass after adding release/HIL plan and honest blocked-state semantics.

## Round 5 — Scope and implementation readiness review

### Checks

- Does the new task touch the sibling task or product code? No.
- Does it create a Trellis parent link that mutates existing artifacts? No.
- Does PRD state user-visible behavior, non-goals and evidence gates? Yes.
- Does design choose a primary architecture without over-freezing current code paths? Yes.
- Are uncertain implementation details isolated behind explicit spikes? Yes: source copy API and final Swift build/FFI mechanics.
- Are `implement.jsonl` and `check.jsonl` curated from specs/research rather than code files? Required before validation.
- Is task still `planning` and inactive? Must remain so.
- Is Windows explicitly excluded? Yes.

### Remaining blockers before implementation start

1. sibling macOS lifecycle task must settle helper-facing product/transaction contracts;
2. implementation-time upstream dependency refresh;
3. user approval to start;
4. formal signing/HIL environment for final enablement.

### Final planning verdict

**Ready as a planning task after manifests and Trellis validation pass.**

The design is intentionally conservative: one legacy-compatible registrar, mature typed/authenticated communication, no generic privileged operations, one existing lifecycle owner and formal HIL before enablement.
