# Design — Auth Observation and Session State Machine

## 1. Domain model

Auth has two durable concepts:

```text
AuthObservation  read-only current authority
AuthSession      one requested interactive/command operation
```

They must not be collapsed. A session can successfully hand off a login UI while the observation remains unknown.

## 2. Observation wire union

Illustrative shape:

```text
AuthObservationDto =
  | { kind: "account", ownership, authority, state, checkedAt, reasons }
  | { kind: "provider_connections", ownership: "provider_owned",
      authority, state, providers[], checkedAt, reasons }
  | { kind: "handoff_only", ownership: "agent_owned",
      authority: "unverified", checkedAt, reasons }
  | { kind: "fyagent_managed", ownership: "fyagent_managed",
      destination: "auth_center", checkedAt, reasons }
  | { kind: "unavailable", ownership: "unavailable", reasons }
```

Provider summaries use bounded identifiers/labels parsed from an official CLI command. They never include credential type/value, account email or backing file location.

## 3. Session wire contract

```text
start_agent_auth_session({ agentId, intent, providerId? })
get_agent_auth_session({ sessionId })
stop_waiting_for_agent_auth({ sessionId })
```

`providerId` is permitted only when it was returned by a prior bounded provider observation or validated against the installed CLI's official provider selector. It is not a command fragment.

Snapshot:

```text
AuthSessionSnapshot {
  contractVersion,
  sessionId,
  agentId,
  intent,
  stage,
  canStopWaiting,
  outcome?,
  observation?,
  reasonCode?
}
```

Terminal outcomes distinguish:

- `verified_logged_in`;
- `verified_logged_out`;
- `verified_provider_change`;
- `handoff_only`;
- `failed/cancelled/timed_out`.

## 4. Coordinator

One `AuthSessionCoordinator` owns:

- session IDs and bounded active/terminal storage;
- per-agent/intent single-flight;
- lifecycle transitions and terminal immutability;
- polling cadence/deadline;
- stop-waiting semantics;
- adapter dispatch and redacted reason mapping;
- optional reload recovery.

It does not know Claude/OpenCode JSON details. Adapters own command and parser policy.

## 5. Adapter interface

Conceptual private interface:

```rust
trait AgentAuthAdapter {
    fn ownership(&self) -> AgentAuthOwnership;
    fn capabilities(&self) -> AuthCapabilities;
    async fn observe(&self) -> AuthObservation;
    async fn launch(&self, intent: AuthIntent) -> AuthLaunchResult;
    async fn verify(&self, before: &AuthObservation) -> AuthVerifyResult;
}
```

Launch and observe use closed Tooling APIs; no adapter builds a renderer-provided shell string. On Windows, commands run in the frozen interactive-user environment.

## 6. Adapter behavior

### Claude

- `observe`: run `claude auth status` with bounded timeout/output; require expected JSON shape and exit semantics.
- `login`: capture before observation, launch official login terminal, poll status.
- `logout`: run official closed command, then require logged-out observation.
- Raw JSON is dropped immediately after parsing; secret-like keys cause unknown.

### OpenCode

- `observe`: run `opencode auth list`; parse only bounded provider rows.
- `connect`: launch `opencode auth login` with closed optional provider selection when documented/validated; verify a provider set change or selected provider presence.
- `logout`: use the official interactive/provider operation and verify set change.
- Never expose or read the backing auth file.

### Grok

- `observe`: handoff-only until an official structured status API is reviewed.
- `login`: launch `grok login`; terminal outcome is handoff-only.
- `logout`: execute `grok logout`; command completion is recorded, but auth observation remains unknown unless a future observer proves otherwise.

### Desktop Agents

- `launch`: use Stage 1 exact candidate launch.
- `verify`: unsupported; user may click reread, which remains handoff-only/unknown.

### Codex

- Adapter returns `fyagent_managed/auth_center`; no session start command in this domain.

## 7. Command output safety

- Output byte limits and timeout are adapter-specific and small.
- Parsers allowlist fields/rows and reject control characters, unbounded arrays and secret-like key names.
- `Debug`, errors and telemetry contain only stable code/agent/intent/stage.
- Tests use synthetic tokens only to prove redaction and never persist real auth output.

## 8. Session transitions

```text
preparing -> launching
launching -> awaiting_user | verifying | handoff_complete | failed
awaiting_user -> verifying | handoff_complete | timed_out | cancelled
verifying -> verified | awaiting_user | failed | timed_out
```

Illegal or duplicate terminal transitions are rejected. Stop-waiting ends FyAgent monitoring but does not claim the external login was cancelled.

## 9. Frontend

`AuthStatusPanel` renders the observation union and session snapshot. Copy is authority-specific:

- verified: “已重新检查：…”；
- awaiting: “已打开登录流程，等待完成”;
- handoff-only: “入口已打开，FyAgent 无法确认最终登录状态”;
- unknown: “当前状态未确认”;
- provider-owned: “已连接 Provider” with bounded provider list/count.

The panel is reused by Agent directory/details. Auth Center gets a navigation action rather than embedded duplicated UI.

## 10. Relationship to install state

Install state, Auth observation and model connectivity remain three independent resources. A product can be installed but Auth unknown, logged in but model request unavailable, or Provider-configured without a global account. No reducer derives one from another.
