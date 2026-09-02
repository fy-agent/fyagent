# V2 External Agent Auth UI Contract

## 1. Scope / Trigger

Read this contract before changing the Agent Auth status panel, login/logout/
connect-provider controls, Auth session polling, desktop target selection,
Auth wording, or cleanup of sensitive/transient Auth state.

Primary owners:

- `src/v2/pages/agents/AgentAuthStatusPanel.tsx`
- `src/v2/pages/agents/useAgentAuthSession.ts`
- `src/v2/shared/features/agent-auth.ts`
- desktop `AgentAuthPorts` adapter in `src/v2/platform/desktop/ports.ts`

Native authority is [External Agent Auth](../backend/external-agent-auth.md).
Installation/action UI remains in
[V2 Agent Directory](./v2-agent-directory.md).

## 2. Signatures

The renderer uses `AgentAuthPorts` for exactly these behaviors:

```text
observe(agentId)
start(request)
get(sessionId)
getActive(agentId)
stopWaiting(sessionId)
```

All results pass the strict v1 parsers in
`src/v2/shared/features/agent-auth.ts`. The page sends only:

```text
agentId
intent = login | logout | connect_provider
providerId?                         // opaque native capability
inventoryId? + targetId? + expectedTargetRevision? // complete triplet
```

Auth stages remain closed:

```text
preparing | launching | awaiting_user | verifying |
verified | handoff_complete | failed | cancelled | timed_out
```

Observation authority remains tagged rather than one global boolean:

```text
account | provider_connections | handoff_only |
fyagent_managed | unavailable
```

## 3. Contracts

### One shared panel and one hook

- All Agent detail pages use the shared Auth panel and `useAgentAuthSession`;
  do not create product-specific login polling loops in cards/pages.
- The hook owns active-session recovery, bounded polling, terminal cleanup and
  stop-waiting interaction. Query/server state stays in the query layer;
  transient confirmation and selected provider/target stay local.
- A second start while a non-terminal session exists surfaces the native
  conflict/existing session. The UI does not manufacture a parallel session or
  overwrite the current session ID.
- Route unmount stops local polling and clears transient sensitive state. It
  does not claim to stop the external browser/application/CLI flow.

### Evidence-correct product behavior

- Claude login/logout may render confirmed only when the native session reaches
  `verified` with the matching verified outcome.
- OpenCode is provider-level. The UI lists sanitized provider observations and
  passes one opaque provider ID for connect/logout; it never paints a global
  logged-in state from “some provider exists.”
- Grok Build and QoderWork/TRAE Work/WorkBuddy desktop flows remain
  handoff-only. `handoff_complete` is described as “opened/continue in vendor
  UI,” not “logged in/out.”
- Codex `fyagent_managed` routes to the existing Auth Center and does not start
  an external Agent Auth session.
- Unavailable/unsupported authority shows the closed native reason and offers
  only allowed guidance. The renderer does not infer account state from files,
  processes or prior successful handoff.

### Desktop target binding

- Desktop Auth uses the same opaque inventory target contract as lifecycle.
  Multiple candidates require explicit selection; target capabilities are not
  parsed or persisted.
- The complete target triplet is forwarded unchanged. Partial target state
  disables start and prompts a fresh scan.
- A native `refresh_required`, expired target or target drift clears the local
  selection and rereads inventory. It never retries against the stale target.

### Secrets, errors and cleanup

- The panel never asks for or persists vendor passwords/tokens/cookies. An
  OpenCode provider capability is not a credential and must not be expanded
  into one.
- Raw CLI output, paths, commands, registry/bundle identity and environment
  never render. Map closed reasons/stages to localized copy.
- Stop waiting is explicit user action and is worded as stopping FyAgent
  monitoring. Do not label it vendor logout or external cancellation.
- Terminal success, failure, timeout, cancellation, target change, Agent
  change and unmount clear provider/target confirmation and any sensitive
  draft.

## 4. Validation & Error Matrix

| Condition | Required UI result |
| --- | --- |
| v1 parser/version/enum failure | Fail closed; do not spread/render raw Auth DTO. |
| Observation is `fyagent_managed` | Link to Auth Center; do not start external session. |
| Observation is `handoff_only` | Render handoff limitations before user action. |
| OpenCode has multiple providers | Require/select one opaque provider; no global success. |
| Desktop inventory is multiple | Require explicit opaque target. |
| Target triplet is partial/stale | Disable/refresh; do not call start. |
| Existing non-terminal session | Resume/show it or surface conflict; no second local session. |
| Stage is `awaiting_user`/`verifying` | Keep pending state; do not mark logged in/out. |
| Stage is `handoff_complete` | Report handoff only. |
| Stage is `timed_out` | Explain verification timed out; do not state vendor flow failed/cancelled. |
| User chooses stop waiting | Stop native monitoring and local polling; external flow may continue. |
| Agent/route changes | Clear transient target/provider/session presentation and sensitive draft. |
| Raw secret/output/path appears | Security regression. |

## 5. Good / Base / Bad Cases

- **Good:** Claude remains pending until native structured status verifies the
  requested login state, then the panel renders confirmed.
- **Good:** OpenCode logout sends one opaque provider ID and updates only after
  native provider-set verification.
- **Base:** WorkBuddy opens and returns handoff complete; the panel tells the
  user to finish in WorkBuddy without claiming account status.
- **Base:** the user stops waiting; monitoring ends while the browser/app may
  remain open.
- **Bad:** infer auth from `~/.vendor`, accept a token in the form, mark success
  after launch, retry stale desktop target, or use the install-action mutation
  for login/logout.

## 6. Tests Required

```bash
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
```

Required assertions:

- strict v1 observation/session/request parsing, closed stages/outcomes/reasons
  and forbidden URL/path/command/token/env fields;
- one shared panel/hook, active-session recovery and no per-product polling
  implementation;
- Claude verified-only success, OpenCode provider-specific behavior, Grok/
  desktop handoff-only behavior and Codex Auth-Center routing;
- target selection, complete triplet forwarding, stale-target refresh and no
  persisted opaque capability;
- stop-waiting wording and behavior do not claim external cancellation;
- timeout/poll/unmount paths clear sensitive/transient state and do not leave
  stale green status;
- accessibility: focus moves to actionable/error status, controls have names,
  and progress/status is not color-only.

## 7. Wrong vs Correct

Wrong:

```tsx
const login = async () => {
  await ports.agentInstallReadiness.startAction({
    agentId,
    action: "login" as never,
  });
  setLoggedIn(true);
};
```

Correct:

```tsx
const auth = useAgentAuthSession(agentId);

await auth.start({
  agentId,
  intent: "login",
  ...selectedOpaqueTarget,
});

// Render by native stage/authority; only verified is confirmed.
```
