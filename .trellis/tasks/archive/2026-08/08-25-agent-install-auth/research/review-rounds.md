# Planning review rounds

Reviewed: 2026-08-25

The task was reviewed after the first complete PRD/design/implementation draft. Findings were applied to the task before delivery.

## Round 1 — Architecture and reuse

Question: does the task accidentally create parallel registries/installers/auth stores, or damage an existing consumer while optimizing the Agent Catalog?

Result: **changes required**.

Finding:

- The first draft correctly reused Agent Catalog, Tooling, Codex Desktop Installer, Auth Center, `CodexOAuthManager` and `ProviderMeta.authBinding`, but did not explicitly protect existing non-Catalog Tooling consumers such as Gemini CLI, OpenClaw and Hermes.

Fix applied:

- Added a non-goal and acceptance/regression gate that the Catalog façade is an incremental Tooling consumer and cannot retire/reroute existing Tooling product surfaces.

Status after fix: **pass**.

## Round 2 — Security, source coherence and privilege boundaries

Question: can renderer input, redirect drift, TOCTOU, Windows elevation or raw helper output turn the planned façade into an arbitrary execution/download channel?

Result: **changes required**.

Findings:

- The first draft's generic `start_agent_action(agentId, action)` did not explicitly preserve the existing Codex installer's checked-release/`expectedReleaseId` coherence.
- Redirect policy said “bounded” but did not require an explicit product-owned host allowlist.
- The optional Windows user helper needed a negative guarantee against returning raw child stdout/stderr, commands or paths to the elevated parent/renderer.

Fixes applied:

- Managed-package readiness now exposes only an opaque backend release/source revision; start force-refreshes and rejects drift.
- Source descriptors now own fixed HTTPS host/redirect allowlists and reject downgrade/unknown host/excess hops.
- Windows helper, if implemented, accepts/returns only closed actions/states and sanitized reason codes; no raw execution artifacts cross the privilege boundary.

Status after fix: **pass**.

## Round 3 — Platform/package compatibility and upstream backport fit

Question: does “reuse Codex Desktop Installer” incorrectly assume all desktop packages use Codex's MSIX/DMG deployment mechanics, or copy CC Switch's auth assumptions blindly?

Result: **changes required**.

Finding:

- The first draft could be read as requiring all Windows desktop Agent packages to reuse Codex's MSIX/PackageBridge deployer. QoderWork/TRAE Work/WorkBuddy may publish EXE/NSIS/MSI or other formats; forcing MSIX semantics would be incorrect.

Fix applied:

- Clarified that the reusable authority is the single managed-package orchestration/security core. Concrete DMG/MSIX adapters are reused only for matching formats; a different positively evidenced package format gets a narrow closed adapter under the same core, never a generic executable/path runner.
- Added explicit platform/architecture/package-format source branches and a “do not infer ARM compatibility from marketing copy” gate.

CC Switch v3.20.0 was also checked against current FyAgent/OpenAI behavior. The task retains only useful state-machine/concurrency ideas and explicitly rejects unconditional `auth.json` token-package writes.

Status after fix: **pass**.

## Round 4 — Evidence precision and testability

Question: do task statements distinguish public vendor contracts from facts visible only in current open-source implementation, and can every risky decision be tested/fail closed?

Result: **changes required**.

Finding:

- OpenAI's public Codex auth documentation currently documents `file/keyring/auto`; current source also contains `ephemeral`. The draft phrasing could incorrectly promote the source-only variant into a public supported-mode promise.

Fix applied:

- Reworded the contract to public `file/keyring/auto` plus defensive handling of source-visible/future/unknown non-file modes. Only `file` receives FyAgent native projection without a separate stable official import API.

Status after fix: **pass**.

## Final review decision

**APPROVED FOR USER REVIEW / NOT APPROVED FOR IMPLEMENTATION YET.**

The task remains in `planning`. Implementation starts only after explicit approval and Trellis `task.py start`.

## Source re-review after concrete package URLs

The user later supplied concrete QoderWork CN, TRAE Work CN and WorkBuddy package URLs. A second focused source review was performed before changing the planning decision.

### Source Round A — Provenance and latest resolution

Result: **changes required, then pass**.

- Qoder's current first-party frontend source independently contains the supplied stable `/releases/latest/` links, so the previous `official_page_only` assumption was too conservative.
- TRAE's current first-party frontend exposes a no-parameter latest API; its `data.solo`/CN branch returns exactly the supplied build `2.3.76922` URLs.
- WorkBuddy's current first-party frontend calls `/v2/update` with three closed platform IDs; current responses identify `5.3.14.36279234` and the supplied packages.

Fix: replace “resolver still unknown” planning with concrete source adapters while retaining first-party/allowlist/schema gates.

### Source Round B — Version semantics and TOCTOU

Result: **changes required, then pass**.

Finding: Qoder's versionless latest aliases solve download selection but do not expose a trustworthy remote semantic version comparable to the installed app. Treating ETag/Last-Modified or the currently stale update-log top entry as a version would manufacture evidence and make the checked-release design dishonest.

Fix: Qoder gets explicit `current latest` semantics with remote version unknown; it revalidates the fixed alias before action but does not show a fabricated `latestVersion` or claim update availability by semver. TRAE/WorkBuddy keep exact opaque release revision checks because their machine resolvers expose current versioned state.

### Source Round C — URL authority and fallback behavior

Result: **changes required, then pass**.

Finding: TRAE and WorkBuddy APIs return URLs. Passing those through unchanged would weaken the earlier “product-owned source” boundary even though the APIs are first-party. Pinning the user-supplied concrete URLs as an error fallback would also silently install stale builds later.

Fix: resolver-returned URLs are accepted only after exact HTTPS host/product-prefix/platform/arch/filename/package-format grammar validation. On resolver/schema/host/package failure all three products fall back to their official download page, never to the versions observed during this research.

Status after focused source re-review: **pass**. The overall task remains **APPROVED FOR USER REVIEW / NOT APPROVED FOR IMPLEMENTATION YET**.
