# Managed Auth Login Presentation Contract

## 1. Scope / Trigger

Read before changing the login-session parser, the login hook, or the login
dialog. The account/connection/source port is owned by [Managed Auth](./managed-auth.md);
native admission, OAuth and session generations by
[Managed Auth Login](../backend/managed-auth-login.md).

Owners: `shared/features/managed-auth.ts`, `pages/auth/useManagedAuthLoginSession.ts`,
and `pages/auth/LoginDialog.tsx`. The renderer observes native sessions; it does
not implement provider polling or credential writes.

## 2. Signatures

```text
parseManagedAuthLoginSession(value: unknown) -> ManagedAuthLoginSessionSnapshot
ManagedAuthPort.startLogin(request) -> Promise<ManagedAuthLoginSessionSnapshot>
ManagedAuthPort.getLoginSession(sessionId) -> Promise<ManagedAuthLoginSessionSnapshot>
ManagedAuthPort.cancelLogin(sessionId) -> Promise<ManagedAuthLoginSessionSnapshot>
managedAuthCommandErrorCopy(cause: unknown) -> safe user-facing copy
```

Snapshot fields and closed values are defined by the native owner. Parse the
exact contract version/key set, opaque ID, provider/method, stage, capabilities,
terminal state and completion/reason combinations before accepting a response.

## 3. Contracts

- `device_code + preparing` may have null `userCode`, `verificationUri` and
  `expiresAt`. Keep the valid session ID and poll it; do not reject the initial
  native response merely because no device grant has arrived.
- Later nonterminal device stages require all three fields. Validate every
  present device URI as HTTPS, on the closed provider host, without credentials,
  query or fragment. Nullable Preparing does not loosen URL or terminal checks.
- Poll the opaque backend session while the route is active and the session is
  nonterminal. Hidden routes and unmount stop polling. Recovered sessions reuse
  their IDs; opening the dialog must not start a duplicate login.
- Startup, polling and mutation failures use `managedAuthCommandErrorCopy`.
  Render startup errors even while `snapshot` is null, once across all wizard
  steps. Retain valid choices and permit safe retry/exit. Never show raw native
  errors, paths, tokens or callback data.
- Cancel calls the native port. A returned terminal snapshot supersedes an
  older in-flight poll; late responses cannot revive that session. Closing the
  dialog alone does not cancel the backend session or prove worker shutdown.
- Keep focus on the available action during startup failure; normal close
  restores focus only to an available source control. Do not force focus onto
  a disabled action while an independent backend session remains active.

## 4. Validation & Error Matrix

| Condition                                               | Result                                      |
| ------------------------------------------------------- | ------------------------------------------- |
| Valid native Preparing, all device fields null          | Accept session and poll                     |
| Later nonterminal device stage missing any device field | Reject response                             |
| Wrong host, non-HTTPS, userinfo, query or fragment      | Reject response                             |
| Startup fails before a snapshot exists                  | Show safe error, preserve choices/retry     |
| Cancel returns terminal; earlier poll arrives later     | Retain terminal; no duplicate completion    |
| Active session dialog closes                            | Close UI; no manufactured cancel or success |

## 5. Good / Base / Bad Cases

- Good: accept an OpenAI/xAI Preparing snapshot, poll to AwaitingUser, then retain
  the backend Cancelled terminal after a late poll.
- Base: a startup error has no session; show safe feedback and retry the same
  valid selections through the existing port.
- Bad: demand a device code before accepting Preparing, hide error inside the
  session-only branch, relax official URL checks, or treat dialog close as cancel.

## 6. Tests Required

Call the production session and overview parsers with native Preparing shapes,
later complete/missing-field stages, bad official URI and terminal/reason controls.
Exercise the actual AuthPage -> hook -> dialog with a controlled port for
no-session errors/redaction, retained choices, retry and available focus recovery.
Exercise Preparing -> poll -> terminal cancel -> late poll and hidden-route resume.

Focused owners are `tests/renderer/features/iteration_iteration_login.test.ts`,
`tests/renderer/pages/auth/iteration_iteration_login.test.tsx` and
`tests/renderer/pages/auth/useManagedAuthLoginSession.test.tsx`; retain existing
managed-auth port/page tests. These fixtures prove renderer behavior. Native
worker, callback and OS-vault acceptance requires separate matching-host evidence.

## 7. Wrong vs Correct

Wrong: require device fields for every nonterminal snapshot and render startup
errors only when a session exists.

Correct: apply field requirements by stage, retain the Preparing ID for polling,
and show safe command errors independently of snapshot existence.
