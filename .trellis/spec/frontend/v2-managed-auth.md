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
refresh ownership and recovery semantics are owned by
[Managed Auth Core](../backend/managed-auth.md); provider login sessions by
[Managed Auth Login](../backend/managed-auth-login.md); and software projection
by [Managed Auth Consumers](../backend/managed-auth-consumers.md). This
frontend contract does not make browser fixtures or mock IPC into native
authentication evidence.

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
- Account detail lists already-linked software and matching unlinked software
  for the same provider. Linked cards expose the closed connection actions
  that switch official usage (`switch_to_official`, `switch_account`). Unlinked
  matching slots expose `connect_account` when the backend advertises it. A
  ready account may also start a `connect_consumer` login for a matching
  consumer that is not yet connectable from the saved credential purpose. The
  page does not auto-connect on login and does not install software.
- Connection rows show the connected official account/provider slot, current
  request source, whether an official session is preserved, credential-renewal
  owner as user-facing copy, pending restart and closed available actions.
- A connection `targetId` of `null` means the slot is not bound to a lifecycle
  install target. It is not evidence that the software is missing. Do not show
  “未检测到可管理的安装实例” from that null; the auth page still never
  installs software as a side effect.
- Internal terms such as SecretRef, credential ID, refresh-token lineage,
  projection generation and native path never appear in product copy or DOM.

### Strict wire boundary

- `managed-auth.ts` parses every native response from `unknown`, requires the
  exact contract version and exact key set, and accepts only closed enums,
  bounded labels, canonical opaque IDs/revisions and valid timestamps.
- The overview parser validates cross-resource references, unique IDs,
  connected-account counts, at most eight uniquely identified active sessions,
  and provider/consumer compatibility. The backend admits at most one
  non-terminal session per provider, while one OpenAI and one xAI session may
  coexist. A malformed reference rejects the complete snapshot.
- Login snapshots never expose authorization URLs, callback URLs, OAuth code,
  state, verifier, device authorization ID, token, native path or raw error.
  A device-code snapshot may expose only a bounded user code and a validated
  query-free HTTPS verification URI whose host matches the closed provider
  summary.
- Request payloads contain opaque IDs, expected revisions and closed actions.
  The renderer does not send credentials, filesystem locations, commands,
  arguments, environment variables or arbitrary URLs.
- A completed login stage or completed mutation outcome accepts only
  `reasonCode=null` or `reasonCode=pending_restart`. `pending_restart` is a
  successful save/readback that still awaits consumer pickup, not a failed
  operation. Any other non-null completed reason rejects the whole response.
  The freshly parsed overview, not `pendingRestartConsumers` alone, decides
  which connection is pending.

### State and mutations

- TanStack Query owns the overview and active-session snapshots. URL state owns
  the selected view/account/consumer. Secret or OAuth material never enters
  Query state, route state or localStorage.
- Login-session polling/recovery is owned by one hook. Remount resumes the
  backend session instead of starting a duplicate. Polling stops while the
  persistent route is hidden and on terminal/unmount.
- Account default/removal and OpenCode file mutations are revision-enforced by
  their backend owners. Every connection request still carries the displayed
  revision, but Codex/Grok metadata-only paths do not yet provide independent
  stale-revision CAS. Positive UI state therefore comes only from the mutation
  result's freshly parsed overview/readback; the page does not patch an
  optimistic success or claim stronger concurrency protection.
- `reopenLogin` asks the backend to open the official page for the current
  non-terminal session. The renderer never receives an authorization URL.
- Account removal requires a backend impact preview. Failure to disconnect all
  dependents keeps the account visible and recoverable; the page never hides a
  still-referenced account.
- `pendingRestart`, partial completion, external change, unavailable authority
  and recovery-required remain explicit states. Starting a browser, writing a
  credential or launching software is not sufficient to paint success.
- `completed + pending_restart` keeps the positive result and presents a
  restart-specific next step. It must not become “请稍后重试”, an optimistic
  connected badge, or a generic failure banner.
- The current backend reports Codex `connected` only when live ChatGPT
  identity matches the connection-bound credential. A ready SecretRef with a
  different or missing live identity is saved-not-projected / disconnected.
  Explicit non-file stores still surface `native_projection_unavailable`.
  The renderer must not use credential presence alone for HIL or stronger
  success claims.

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
| More than eight active sessions or a duplicate session ID appears | Reject the overview/session chain. |
| OpenAI and xAI each have one active session | Accept both; the UI follows the selected opaque session ID and must not merge them. |
| A second start is attempted for a provider with a non-terminal session | Preserve `operation_conflict`; recover the existing provider session. |
| Page is hidden by persistent routing | Pause automatic queries and polling; retain selected UI state. |
| Mutation returns no authoritative overview/readback | Keep prior state and show uncertainty; do not claim success. |
| Account/default/removal or OpenCode file mutation has a stale revision | Preserve stale error, reread, and require an explicit retry. |
| Codex/Grok metadata action completes from an older displayed revision | Render only the returned overview; do not infer that the backend performed stale-write rejection. |
| Codex is `disconnected` with a saved account but live identity is absent or different | Present “账号已保存”, not “已连接”; never count it as proven native pickup. |
| Completed login/mutation has `reasonCode=pending_restart` | Accept the response, render the returned overview, and offer restart-specific guidance; do not show a generic retry. |
| Completed login/mutation has another non-null reason | Reject the response as invalid managed-auth data. |
| Account removal preview fails | Do not expose the destructive confirmation. |
| Connection needs restart | Show saved/pending-restart separately; do not say the consumer is already using it. |
| Managed Agent summary is clicked | Navigate to `/auth?consumer=<closed-id>`; do not start the old Agent Auth session. |
| Token/code/state/verifier/path/command reaches DTO, cache, route or DOM | Security regression. |

## 5. Good / Base / Bad Cases

- **Good:** Codex displays selected OpenAI account metadata, `DeepSeek API` as
  the current request source and `官方登录已保留` as separate facts; it does not
  use credential presence alone as proof of native Codex pickup.
- **Good:** a Codex or OpenCode login finishes credential storage plus file
  readback, then reports `completed + pending_restart`; the dialog
  distinguishes “saved” from live consumer pickup and offers restart or later
  handling.
- **Base:** OpenAI login snapshots come from backend sessions. Browser PKCE and
  Device Code can complete an account after SecretRef readback. Codex file
  projection is capability-gated by effective store, complete material, and
  live identity readback. A ready credential with a different or missing live
  identity is saved-not-projected / disconnected, never connected.
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
  unknown keys, invalid references, malformed revisions and forbidden fields;
- maximum-eight/unique session parsing plus backend per-provider single-flight
  and cross-provider coexistence;
- Tauri command/payload mapping and request/response identity binding;
- `/auth` routing, navigation selection, primary-route keep-alive and browser
  native-only behavior;
- account/connection/request-source separation, login recovery, device-code
  copy, destructive preview, readback-only success and pending-restart states;
- stale account/OpenCode mutations reread before retry; Codex/Grok metadata
  actions render only the returned overview and tests do not claim full CAS;
- Codex `disconnected` with a saved account is presented as “账号已保存”, not
  “已连接”, and is never counted as native pickup/HIL;
- strict parsers accept completed login/mutation responses with only null or
  `pending_restart`, reject every other completed/non-null reason, and keep
  pending restart distinct from generic retry;
- account detail lists matching unlinked slots and exposes connect/switch from
  that page; purpose-mismatch Codex slots start `connect_consumer` login with
  no `accountId`;
- a connection `targetId` of `null` does not render “未检测到可管理的安装实例”;
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
