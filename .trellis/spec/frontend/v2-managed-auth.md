# V2 Managed Account and Authentication Contract

## 1. Scope / Trigger

Read this contract before changing the V2 `/auth` route, managed official
accounts, software connections, model-request source presentation, managed
login sessions, account removal previews, or the Codex/Grok Build/OpenCode
entry points that route users into the central account surface.

Primary renderer owners:

- `src/v2/shared/features/managed-auth.ts`
- `src/v2/shared/platform/tauri/feature-ports/managedAuth.ts`
- `src/v2/shared/features/queries.ts`
- `src/v2/pages/auth/**`
- `src/v2/shared/config/navigation.ts`
- `src/v2/app/primaryPages.tsx`

External Agent-owned authentication remains under
[V2 External Agent Auth UI](./v2-agent-auth.md). Native credential, OAuth,
consumer projection, refresh ownership and recovery semantics are owned by the
backend managed-auth contract once activated; this frontend contract does not
make browser fixtures or an unavailable native façade into authentication
evidence.

## 2. Signatures

`ManagedAuthPort` is the only V2 transport surface for managed accounts:

```ts
interface ManagedAuthPort {
  getOverview(): Promise<ManagedAuthOverview>;
  startLogin(request: StartManagedAuthLoginRequest): Promise<ManagedAuthLoginSessionSnapshot>;
  getLoginSession(sessionId: string): Promise<ManagedAuthLoginSessionSnapshot>;
  cancelLogin(sessionId: string): Promise<ManagedAuthLoginSessionSnapshot>;
  reopenLogin(sessionId: string): Promise<ManagedAuthLoginSessionSnapshot>;
  switchLoginMethod(
    sessionId: string,
    method: ManagedAuthLoginMethod,
  ): Promise<ManagedAuthLoginSessionSnapshot>;
  setDefaultAccount(
    accountId: string,
    expectedRevision: string,
  ): Promise<ManagedAuthMutationResult>;
  previewAccountRemoval(
    accountId: string,
    expectedRevision: string,
  ): Promise<ManagedAuthAccountRemovalPreview>;
  removeAccount(
    previewId: string,
    accountId: string,
    expectedRevision: string,
  ): Promise<ManagedAuthMutationResult>;
  applyConnectionAction(
    request: ManagedAuthConnectionActionRequest,
  ): Promise<ManagedAuthMutationResult>;
}
```

There is no `getActiveLoginSession`, `mutateAccount`, or `mutateConnection`
method. Active login sessions arrive on `overview.activeSessions`. Mutation
`operationId` is a hyphenated UUID v4.

The closed provider set is:

```text
openai | xai | github_copilot
```

The closed consumer set is:

```text
codex | grokbuild | opencode | fyagent_proxy
```

The overview keeps three separate resource families:

```text
accounts      // official account identities and account health
connections   // one software/provider-slot connection to a managed credential
requestMode   // where a consumer currently sends model requests
```

These are not interchangeable. An account can be ready while one connection
needs restart, and Codex can retain an official account while its current
request mode is a third-party API.

## 3. Contracts

### Central route and navigation

- `/auth` is a first-class lazy-loaded primary route. It participates in the
  same visited-route keep-alive, visibility gating, prefetch and selection
  rules as Agents, Models, Skills, MCP, Prompts and Memory.
- The primary navigation label is `账号与认证`. Direct links may carry only the
  closed view, account ID, consumer ID and existing closed Agent-return tuple.
  Free-form return URLs, native paths, commands and provider URLs are invalid.
- Wide layouts use the existing master-detail/split primitives. Narrow layouts
  expose a list view and an explicit detail/back transition; operations must
  not be hidden in horizontal overflow.

### Information architecture

- The first user-level distinction is `账号` versus `软件连接`.
- Account rows show provider identity, login label, health, default state and
  connection count. Quota/profile availability does not redefine login health
  or reorder accounts by short-lived usage.
- Connection rows show the connected official account/provider slot, current
  request source, whether an official session is preserved, credential-renewal
  owner as user-facing copy, pending restart and closed available actions.
- Internal terms such as SecretRef, credential ID, refresh-token lineage,
  projection generation and native path never appear in product copy or DOM.

### Strict wire boundary

- `managed-auth.ts` parses every native response from `unknown`, requires the
  exact contract version and exact key set, and accepts only closed enums,
  bounded labels, canonical opaque IDs/revisions and valid timestamps.
- The overview parser validates cross-resource references, unique IDs,
  connected-account counts, active-session uniqueness and provider/consumer
  compatibility. A malformed reference rejects the complete snapshot.
- Login snapshots never expose authorization URLs, callback URLs, OAuth code,
  state, verifier, device authorization ID, token, native path or raw error.
  A device-code snapshot may expose only a bounded user code and a validated
  query-free HTTPS verification URI whose host matches the closed provider
  summary.
- Request payloads contain opaque IDs, expected revisions and closed actions.
  The renderer does not send credentials, filesystem locations, commands,
  arguments, environment variables or arbitrary URLs.

### State and mutations

- TanStack Query owns the overview and active-session snapshots. URL state owns
  the selected view/account/consumer. Secret or OAuth material never enters
  Query state, route state or localStorage.
- Login-session polling/recovery is owned by one hook. Remount resumes the
  backend session instead of starting a duplicate. Polling stops while the
  persistent route is hidden and on terminal/unmount.
- Account and connection mutations are revision-bound. Positive UI state comes
  only from the mutation result's freshly parsed overview/readback; the page
  does not patch an optimistic success.
- `reopenLogin` asks the backend to open the official page for the current
  non-terminal session. The renderer never receives an authorization URL.
- Account removal requires a backend impact preview. Failure to disconnect all
  dependents keeps the account visible and recoverable; the page never hides a
  still-referenced account.
- `pendingRestart`, partial completion, external change, unavailable authority
  and recovery-required remain explicit states. Starting a browser, writing a
  credential or launching software is not sufficient to paint success.

### Agent integration

- Codex, Grok Build and OpenCode Agent cards are summary/entry surfaces for
  managed authentication. Their main action routes to `/auth` with the closed
  consumer ID; they do not duplicate account lists, OAuth controls or managed
  connection mutations.
- Claude continues its reviewed Agent-owned login/logout session. QoderWork,
  TRAE Work and WorkBuddy continue trusted desktop handoff. The central page
  does not absorb those flows without a separately reviewed managed adapter.
- Agent install/lifecycle and managed authentication remain separate. The auth
  page never installs software as a side effect.

### Accessibility, responsive behavior and copy

- Tabs, list selection, dialogs, radio groups, code-copy controls and dangerous
  actions use semantic roles, visible focus, keyboard operation and translated
  or stable Chinese accessible names according to the current V2 language
  policy.
- Dialog focus is restored to the invoking control. Copy feedback does not move
  focus. Reduced-motion mode disables non-essential animation. Keep the shared
  `Dialog` mounted while closed so Radix can dismiss on Escape and return
  focus after `aria-hidden` is cleared; do not `return null` when `open` is
  false, and do not wrap login in a second focus owner.
- Copy says what is complete, pending or unknown and gives one safe next step.
  It must not claim login, connection or request routing beyond backend
  readback evidence.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Browser/non-native runtime | Render the controlled desktop-only state; never seed authenticated accounts. |
| Overview has an extra token/path/raw-error field | Reject the complete response. |
| Connection references a missing account/provider | Reject the complete response. |
| Account says two connections but only one references it | Reject the complete response. |
| Device verification URI has query, fragment, wrong host or non-HTTPS scheme | Reject the login snapshot. |
| A second active login session appears | Reject the overview/session chain; never poll both. |
| Page is hidden by persistent routing | Pause automatic queries and polling; retain selected UI state. |
| Mutation returns no authoritative overview/readback | Keep prior state and show uncertainty; do not claim success. |
| Account removal preview fails | Do not expose the destructive confirmation. |
| Connection needs restart | Show saved/pending-restart separately; do not say the consumer is already using it. |
| Managed Agent summary is clicked | Navigate to `/auth?consumer=<closed-id>`; do not start the old Agent Auth session. |
| Token/code/state/verifier/path/command reaches DTO, cache, route or DOM | Security regression. |

## 5. Good / Base / Bad Cases

- **Good:** Codex displays an OpenAI account connection, `DeepSeek API` as the
  current request source and `官方登录已保留` as a separate fact.
- **Good:** an OpenAI browser login finishes credential storage but Codex still
  needs restart; the dialog reports partial completion and offers restart or
  later handling.
- **Base:** OpenAI login snapshots come from backend sessions. Browser PKCE and
  Device Code can complete an account after SecretRef readback. Codex file
  projection remains `native_projection_unavailable` until HIL, so connect
  finishes `partial` rather than claiming a live native login.
- **Base:** OpenCode Path B file write + readback is not a live Desktop
  connection. Show `pending_restart` / “等待重启”. Do not show `已连接`
  while `OPENCODE_EXTERNAL_WRITE_HOT_RELOAD_PROVEN` is false.
- **Bad:** display `已登录` because an account record exists, display `已连接`
  because a file write returned, or display `OpenAI Official` while the active
  provider is third-party.
- **Bad:** copy the leftover `AuthCenterPanel` into V2 or let Agent cards keep a
  second managed-account workflow. The leftover Settings auth tab is a
  compatibility shell only: it must not poll, login, or display a second
  account owner. Leftover Provider `CodexOAuthSection` /
  `XaiOAuthSection` / `CopilotAuthSection` may select an existing
  `authBinding` account from `authGetStatus`; they must not start Device
  Code, open a verification URL, remove accounts, or poll leftover login.

## 6. Tests Required

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
```

Required assertions include:

- all valid closed overview/session/mutation variants and strict rejection of
  unknown keys, invalid references, wrong revisions and forbidden fields;
- Tauri command/payload mapping and request/response identity binding;
- `/auth` routing, navigation selection, primary-route keep-alive and browser
  native-only behavior;
- account/connection/request-source separation, login recovery, device-code
  copy, destructive preview, readback-only success and pending-restart states;
- managed Agent cards navigate to the central page while Claude and desktop
  handoff retain their existing owner;
- leftover Provider OAuth sections remain picker-only and leftover
  `authStartLogin` / `authPollForAccount` / `authRemoveAccount` /
  `copilotStartDeviceFlow` / `copilotLogout` throw
  `legacy_auth_mutation_disabled` without invoking Tauri;
- keyboard/focus/ARIA, narrow viewport and reduced-motion behavior;
- overview `reasonCodes` render closed-set recovery copy
  (`secret_unavailable`, `migration_blocked`, `pending_restart`,
  `external_change_detected`) plus a refresh action, never a generic
  “temporarily unavailable” banner;
- login-session polling stops while the persistent `/auth` route is hidden
  and resumes without starting a second session.

Browser and mock tests prove renderer behavior only. Real OAuth, OS keyring,
consumer projection, token renewal and restart evidence require the native/HIL
matrix in the backend owner.

## 7. Wrong vs Correct

Wrong:

```ts
const result = await invoke("login", { url, callback, token });
queryClient.setQueryData(["auth"], { loggedIn: true });
```

Correct:

```ts
const session = await ports.managedAuth.startLogin({
  provider: "openai",
  purpose: "connect_consumer",
  consumer: "codex",
  method: "browser_loopback",
  accountId: null,
});

// One session hook resumes/polls the opaque session ID. A terminal mutation
// result replaces the overview only after strict parsing of native readback.
```

Wrong:

```tsx
if (!open) return null;
return <Dialog open={open} onOpenChange={onOpenChange} />;
```

Correct:

```tsx
return <Dialog open={open} onOpenChange={onOpenChange} />;
```

Unmounting the shared Dialog while closed skips Radix dismiss and focus
return. The primitive records the invoking control and restores it on the
next frame after close.
