# Delivery contract

## #42 — reusable account source

- The central account page offers login-backed connection only for a ready account,
  a native-advertised provider/consumer pair and a readable disconnected slot.
- Native provider summaries remove blocked local stores from consumer-purpose
  choices. xAI offers OpenCode and FyAgent Proxy; Grok native projection remains
  unavailable. GitHub Copilot login remains unavailable with no advertised target.
- An unreadable OpenCode auth file offers refresh only even if an independent
  credential exists. Repair plus fresh read restores available actions without
  the observation writing any bytes.
- Existing native per-target preview/save/readback remains the only write owner.
  UI stores each result by connection ID. A successful A survives failed B and
  B's read-only refresh. Retry B takes its current revision and a fresh preview;
  it never replays A or the consumed preview.
- Recovery uses existing closed file controls scoped to Codex auth/config or
  OpenCode auth. UI results are local history for this mounted page; durable
  file recovery remains owned by the native receipt implementation.

## #43 — official login lifecycle

- Agent cards continue routing Grok to `/auth`; they do not start duplicate flows.
- Grok connection detail reaches the existing AgentAuthPort official CLI login
  and confirmed logout. Static terminal recovery steps remain available. Handoff
  is explicitly unverified; no automatic login/expiry/logout claim is invented.
- Shared session hook is now `src/shared/features/useAgentAuthSession.ts`; old
  page-local implementation removed. Active recovery, hidden visibility, terminal
  dedup, generation fencing and request/session identity binding are shared.
- Start response must match requested Agent and intent. Poll/stop response must
  match session ID; hook also checks Agent and intent. Failed recovery blocks new
  starts until explicit recovery succeeds. The existing Claude flow still works.
- xAI Device Code is distinct from Grok official CLI authentication and API Key
  models. Narrow Models copy change and corresponding test labels are included.
- Gated Grok summaries no longer claim official request mode, preserved native
  session, restart, or executable disconnect. They expose unknown/read-only facts.

## Integration notes

Native changes are confined to managed_auth/service.rs, consumers/{grok,opencode}.rs
and providers/xai.rs. No database, credential owner, export, model save handler,
installer, version or proxy recovery changes. The previous #35 worktree/commit
remains frozen; this package is based on the original shared base and should be
cherry-picked separately. Small test/spec overlap with #35 is expected; no runtime
contract or schema version changes.

Remaining external evidence: real host CLI handoff, official account login/logout,
Grok helper/file projection HIL and OpenCode live pickup. No code gate was weakened
to substitute mock evidence for those facts.
