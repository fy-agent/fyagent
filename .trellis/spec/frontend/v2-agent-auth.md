# V2 External Agent Auth UI Contract

## 1. Scope / Trigger

Read this contract before changing Agent authentication status, login/logout or
provider-connection actions, managed-account routing, active-session recovery, polling, stop-waiting,
desktop-target selection, or authentication copy on the V2 Agents page.

Primary owners are:

- `src/v2/shared/features/agent-auth.ts` for closed DTOs, strict parsers, and
  `AgentAuthPort`;
- `src/v2/shared/platform/tauri/feature-ports/agentAuth.ts` for Tauri IPC
  adaptation;
- `src/v2/shared/platform/tauri/features.ts` for desktop Port composition;
- `src/v2/pages/agents/AgentAuthStatusPanel.tsx` for product-specific Auth UI;
- `src/v2/pages/agents/useAgentAuthSession.ts` for recovery, polling, and
  lifecycle state.

Native observation/session semantics are owned by
[External Agent Auth](../backend/external-agent-auth.md). Agent ordering,
capability admission, lifecycle actions, and return navigation remain in
[V2 Agent Directory](./v2-agent-directory.md) and
[V2 Navigation](./v2-navigation.md).

## 2. Signatures

`AgentAuthPort` is the only V2 access to the native Auth surface:

```ts
interface AgentAuthPort {
  getObservation(agentId: AgentCatalogId): Promise<AgentAuthObservation>;
  getActiveSession(
    agentId: AgentCatalogId,
  ): Promise<AgentAuthSessionSnapshot | null>;
  startSession(
    request: StartAgentAuthSessionRequest,
  ): Promise<AgentAuthSessionSnapshot>;
  getSession(sessionId: string): Promise<AgentAuthSessionSnapshot>;
  stopWaiting(sessionId: string): Promise<AgentAuthSessionSnapshot>;
}
```

The observation union is closed:

```text
account              -> state: logged_in | logged_out | unknown
provider_connections -> state: configured | empty | unknown, providers[]
handoff_only          -> unverified Agent-owned handoff
fyagent_managed       -> verified destination: auth_center
unavailable           -> unavailable authority
```

Every observation also carries:

```text
contractVersion = 1
agentId
ownership: fyagent_managed | agent_owned | provider_owned | unavailable
authority: verified | unverified | unavailable
allowedIntents: login | logout | connect_provider
checkedAt
reasonCodes[]
```

Session requests and snapshots are:

```ts
interface StartAgentAuthSessionRequest {
  agentId: AgentCatalogId;
  intent: "login" | "logout" | "connect_provider";
  providerId?: string;
  inventoryId?: string;
  targetId?: string;
  expectedTargetRevision?: string;
}

interface AgentAuthSessionSnapshot {
  contractVersion: 1;
  sessionId: string;
  agentId: AgentCatalogId;
  intent: AgentAuthIntent;
  stage:
    | "preparing"
    | "launching"
    | "awaiting_user"
    | "verifying"
    | "verified"
    | "handoff_complete"
    | "failed"
    | "cancelled"
    | "timed_out";
  canStopWaiting: boolean;
  outcome: AgentAuthSessionOutcome | null;
  observation: AgentAuthObservation;
  reasonCode: AgentAuthReasonCode | null;
}
```

`useAgentAuthSession({ agentId, port, enabled?, onTerminal? })` returns:

```text
snapshot, error, submitting, recovering, busy,
start(request), stopWaiting(), resetTerminal()
```

The page never invokes native Auth commands directly.

## 3. Contracts

### Strict transport parsing

- The Tauri adapter passes `unknown` responses through the parser in
  `agent-auth.ts`. Contract version, exact keys, closed enums, ISO timestamp,
  and duplicate-free lists are strict for all five commands.
- `getObservation(agentId)` and `getActiveSession(agentId)` additionally bind
  the parsed response `agentId` to the requested Agent and reject a mismatch.
  `startSession(request)` validates the request ID and strictly parses the
  returned snapshot, but the current adapter does not perform that second
  request/response equality check. Do not claim this guard is already present;
  a change touching this path should add the check and a regression test before
  treating cross-Agent start responses as fail-closed.
- `getSession` and `stopWaiting` are addressed by `sessionId`, not by a second
  caller Agent ID. The hook keeps the returned session chain; callers must not
  substitute a snapshot from a different interaction.
- A malformed response, or a mismatch on an adapter path that implements the
  binding check, becomes the generic unavailable error. A component must not
  partially trust fields from an invalid DTO.
- Provider summaries expose only `providerId` and `label`. Credentials, tokens,
  command output, vendor config paths, and raw diagnostic payloads never enter
  the public observation or session snapshot.

### Evidence-correct observation

- Account state and provider-connection state are different authorities.
  `configured` is not `logged_in`; `empty` is not proof of logout.
- `handoff_only` says FyAgent can launch/guide the vendor flow but cannot verify
  the resulting account state. The UI must not convert it to success.
- `fyagent_managed` remains a backend observation compatibility variant. Codex, Grok Build and OpenCode detail panels route to the central `/auth` page instead of duplicating account/connection mutations inside the Agent card.
- `unknown`, `unverified`, and `unavailable` remain visible states with reason
  copy. Absence of evidence is never rendered as logged out or healthy.
- Only an intent present in `allowedIntents` may be offered.

### Managed consumer routing

- Codex, Grok Build and OpenCode map to the closed managed consumers `codex`,
  `grokbuild` and `opencode`. Their detail panels render the current Agent Auth
  observation only as a summary and offer one central management button.
  OpenCode observation is `provider_connections` from official `auth.json`
  metadata; a missing PATH CLI is not `unavailable`. Grok Agent observation
  remains `handoff_only` until Managed Auth helper or file projection is
  HIL-proven; the `/auth` Grok card stays `native_projection_unavailable` and
  must not be shown as connected.
  After an OpenCode Path B write, `/auth` must show `pending_restart` until
  matching-host Desktop HIL proves live pickup.
- The destination is `/auth?consumer=<id>&view=connections` with an optional
  closed `agentReturn`/`agentSection` tuple. The page does not pass a token,
  command, path, arbitrary provider name or free-form return URL.
- These managed-consumer panels disable `useAgentAuthSession` recovery and
  start operations. Clicking their management button must issue zero
  `start_agent_auth_session` calls. Claude and the current desktop handoff
  products retain the Agent-owned session flow until their backend ownership
  changes.
- Account/connection/request-source semantics on the destination are owned by
  [V2 Managed Accounts and Authentication](./v2-managed-auth.md).

### Session lifecycle

- When enabled, the hook first calls `getActiveSession(agentId)` so a remounted
  page resumes a native session instead of launching a duplicate flow.
- A non-terminal snapshot is polled with `getSession(sessionId)` until its stage
  is `verified`, `handoff_complete`, `failed`, `cancelled`, or `timed_out`.
  Polling uses hook-owned timers and stops on unmount or terminal state.
- `start(request)` submits through `AgentAuthPort.startSession`, stores the
  returned snapshot, and calls `onTerminal` immediately only when the returned
  snapshot is already terminal.
- A terminal callback can be reached from immediate start/stop results,
  recovered snapshots, or the polling effect. Consumers must make terminal
  side effects idempotent by `sessionId`; `AgentAuthStatusPanel` keeps the last
  handled terminal session before refetching observation.
- `stopWaiting()` is available only when the current snapshot has
  `canStopWaiting = true`. It calls the native command and preserves the
  terminal result; it is not a generic process kill.
- `resetTerminal()` clears only a terminal local snapshot and error. It does
  not alter native Auth/provider state.
- `busy` covers active recovery, submission, or a non-terminal session. While
  busy, the UI must not start a second incompatible Auth action.

### Target/provider selection and navigation

- Provider selection is required only when the native contract returns the
  matching reason/capability. Pages pass the provider ID, not credentials.
- A desktop action that requires an installed target passes the native
  `inventoryId`, opaque `targetId`, and expected revision supplied by lifecycle
  observation. React never constructs an executable path or application ID.
- Route/section return state uses the closed navigation descriptor; Auth
  requests do not accept a free-form return URL.
- After a terminal result, the page rereads the observation/catalog state used
  for display. A session outcome is evidence for that session, not permission
  to manufacture unrelated Agent readiness or installation claims.

## 4. Validation & Error Matrix

| Condition                                                                         | Required result                                                                                                                                                     |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Response contract version or exact keys are wrong                                 | Reject the whole response as unavailable.                                                                                                                           |
| Observation or active-session response `agentId` differs from the requested Agent | Reject in the adapter; do not attach another Agent's Auth state.                                                                                                    |
| `startSession` returns a strict snapshot for another known Agent                  | The current adapter does not rebind it to the request. Do not rely on rejection; add the equality check and regression test before changing/claiming this boundary. |
| Duplicate/unknown intent, stage, outcome, ownership, authority, or reason         | Reject the whole DTO.                                                                                                                                               |
| Observation is `provider_connections/configured`                                  | Say provider configured; do not say account logged in.                                                                                                              |
| Observation is `handoff_only`                                                     | Offer only the admitted handoff and retain unverified wording.                                                                                                      |
| `getActiveSession` fails                                                          | End recovery, expose retry/error state, and do not start automatically.                                                                                             |
| A non-terminal poll fails                                                         | Keep the last snapshot, expose the error, and continue only through the hook-owned retry loop.                                                                      |
| User starts while recovery/submission/session is busy                             | Disable/reject the duplicate action.                                                                                                                                |
| `stopWaiting` when `canStopWaiting` is false                                      | No native call; return no result.                                                                                                                                   |
| Desktop target/revision is stale                                                  | Preserve the native reason and require lifecycle reread/reselection.                                                                                                |
| Terminal session arrives                                                          | Stop polling; deduplicate callback side effects by `sessionId` and reread display authority.                                                                        |
| Secret/raw command output appears in UI or route state                            | Security regression.                                                                                                                                                |

## 5. Good / Base / Bad Cases

- **Good:** remount an Agent card, recover its `awaiting_user` session, continue
  polling by `sessionId`, then render the terminal verified observation returned
  by native code.
- **Good:** a provider-owned Agent shows configured provider labels and offers
  `connect_provider` without claiming an account login.
- **Base:** no active session exists; recovery completes with `snapshot=null`
  and normal observation/actions remain available.
- **Base:** the action is handoff-only; launch it, show `handoff_complete`, and
  retain unverified status until a future observer provides stronger evidence.
- **Base:** a strict `startSession` payload contains another known Agent ID.
  This is a documented current hardening gap, not an accepted semantic success;
  code touching the adapter must close it rather than extending the assumption.
- **Bad:** call `useAgentAuthSession(agentId)` without the Port, poll with a
  component interval, infer logged-in from a provider row, claim every Auth
  command already enforces request/response Agent binding, or put a token/path
  in route/query state.

## 6. Tests Required

Required assertion owners include:

- `tests/v2/platform/agentAuthPort.test.ts`: exact command/payload mapping,
  closed Agent IDs, strict response parsing, contract-version/key mismatch, and
  `agentId` binding for observation/active-session reads. A product change that
  hardens `startSession` must add the corresponding mismatch regression;
- `tests/v2/pages/agents/AgentAuthStatusPanel.test.tsx`: allowed-intent UI,
  provider selection, active-session recovery, terminal polling, stop-waiting,
  terminal-session deduplication, handoff/unknown/unavailable copy, and no
  secret/raw diagnostic display;
- `src-tauri/src/services/external_agents/**` and command tests named by
  [External Agent Auth](../backend/external-agent-auth.md): native state,
  session conflicts, target revision, timeout/cancel, and redacted DTOs;
- architecture tests: the page imports the shared Port/types and never Tauri
  internals or native path/process APIs.

Mock/browser tests prove renderer state transitions only. Real vendor login,
logout, and provider effects require the native/HIL evidence named by the
backend contract.

## 7. Wrong vs Correct

Wrong:

```ts
const auth = useAgentAuthSession(agentId);
setInterval(() => invoke("get_agent_auth_session", { sessionId }), 750);
setLoggedIn(true);
```

Correct:

```ts
const lastTerminalSession = useRef<string | null>(null);
const auth = useAgentAuthSession({
  agentId,
  port: ports.agentAuth,
  onTerminal: (snapshot) => {
    if (lastTerminalSession.current === snapshot.sessionId) return;
    lastTerminalSession.current = snapshot.sessionId;
    void observationQuery.refetch();
  },
});

await auth.start({ agentId, intent: "login" });
// The hook owns recovery/polling; UI renders the parsed snapshot/observation.
```
