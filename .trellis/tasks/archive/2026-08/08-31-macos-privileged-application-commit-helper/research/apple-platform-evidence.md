# Apple Platform Evidence

Date reviewed: 2026-08-31
Source policy: Apple Developer documentation and Apple Developer Technical Support responses are authoritative. Archived Apple guides are used where modern symbol docs do not explain Authorization Services flow.

## 1. Service Management

### SMJobBless

Apple documents `SMJobBless` as a system-domain helper submission API and marks it deprecated in favor of `SMAppService`.

Required facts:

- the executable label must be a key in the app's `SMPrivilegedExecutables` dictionary;
- the authorization reference must contain `kSMRightBlessPrivilegedHelper`;
- the helper is a launchd system job;
- legacy helper packaging uses an executable in `Contents/Library/LaunchServices/<label>` plus embedded info/launchd property lists.

Sources:

- https://developer.apple.com/documentation/servicemanagement/smjobbless(_:_:_:_:)
- https://developer.apple.com/documentation/servicemanagement/
- https://github.com/trilemma-dev/SwiftAuthorizationSample

### SMAppService

Apple documents `SMAppService` for macOS 13 and later. A LaunchDaemon registered through it is not bootstrapped until an administrator approves it in System Settings; `register()` can return launch-denied status while approval is absent.

Its modern bundle layout keeps daemon executables and launchd plists inside the app bundle, commonly using `Contents/Library/LaunchDaemons` and a bundle-relative `BundleProgram`.

Sources:

- https://developer.apple.com/documentation/servicemanagement/smappservice
- https://developer.apple.com/documentation/servicemanagement/smappservice/register()
- https://developer.apple.com/documentation/servicemanagement/updating-helper-executables-from-earlier-versions-of-macos

### Selection implication

FyAgent's current minimum is macOS 12. A pure SMAppService solution would drop a supported platform. Shipping both SMJobBless and SMAppService now would create two production installation/approval/state/layout paths. The task therefore selects one compatible initial path—SMJobBless behind Blessed—and isolates it for a later SMAppService migration.

This is a compatibility tradeoff, not a claim that deprecated API is preferable in a macOS-13-only product. Apple DTS explicitly describes SMAppService as nicer when the deployment target can be raised:

- https://developer.apple.com/forums/thread/757739

## 2. Authorization Services

Apple's factored-application guidance requires the app to request/preauthorize a named right, create an external form, transfer it securely to the helper, and have the helper re-request/check the right immediately before the privileged operation.

Important facts:

- Authorization references are session/process/time-bound;
- `AuthorizationExternalForm` must be protected because another process obtaining it may use it;
- the helper should perform the actual authorization check immediately before mutation;
- custom rights use a reverse-DNS namespace and should represent individual actions;
- the system Security Server controls authentication UI and credential caching;
- Authorization Services does not replace the helper's own BSD/root safety checks.

Sources:

- https://developer.apple.com/documentation/security/authorizationexternalform
- https://developer.apple.com/documentation/security/authorizationmakeexternalform(_:_:)
- https://developer.apple.com/library/archive/documentation/Security/Conceptual/authorization_concepts/02authconcepts/authconcepts.html
- https://developer.apple.com/library/archive/documentation/Security/Conceptual/authorization_concepts/03authtasks/authtasks.html

### Design implication

Blessing the helper once is not sufficient user intent for every later app replacement. The task adds operation-specific rights and transfers the authorization only inside the authenticated XPC call. FyAgent promises fresh request/recheck and no long-lived blanket capability, but does not promise that macOS always shows a password prompt because Security Server owns credential reuse and authentication method.

## 3. XPC peer identity

A Mach service name is not an authentication boundary. Apple provides code-signing requirement APIs for XPC peers, including team, signing identifier, entitlement and explicit requirement checks. The operating system can enforce a peer requirement on every message.

Sources:

- https://developer.apple.com/documentation/xpc/xpc_connection_set_peer_code_signing_requirement(_:_:)
- https://developer.apple.com/documentation/xpc/xpc_connection_set_peer_team_identity_requirement(_:_:)
- https://developer.apple.com/documentation/xpc/xpc_connection_set_peer_entitlement_matches_value_requirement(_:_:_:)

Apple DTS and SecureXPC guidance warn against treating PID as identity because PIDs are reusable and race-prone. The selected transport instead uses the audit-token/code-signing identity of the actual peer.

Design implication:

- helper validates FyAgent app signature/version requirement;
- app validates helper signature/version requirement;
- PID is diagnostic only;
- installation requirements (`SMPrivilegedExecutables` / `SMAuthorizedClients`) and runtime XPC requirements are both required.

## 4. Code signing requirements

SMJobBless requires mutually compatible signing requirements in the app and helper property lists. The requirements must use the actual signed identities and must be checked for every architecture.

The task follows these principles:

- generate/verify requirements from the formal signing policy, not display names;
- include app/helper signing identifiers and Team ID;
- include a minimum safe app version in the helper's authorized clients;
- sign nested code before signing the main app;
- do not accept ad-hoc or Sign to Run Locally as production helper evidence;
- bump helper version with every security-sensitive app release that changes client requirements.

Reference:

- https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/RequirementLang/RequirementLang.html
- https://github.com/trilemma-dev/SwiftAuthorizationSample

## 5. Public API uncertainty retained

The task does not assume a particular recursive-copy API is safe merely because it is public. Implementation must prove whether Apple `copyfile`/clone/fd-relative APIs preserve required app-bundle metadata while preventing symlink/path replacement. If no single API satisfies the closed contract, a minimal fd-relative copier is allowed only after a documented spike and focused tests.

No Apple source found authorizes the following shortcuts, which remain prohibited:

- `sudo` from the GUI;
- administrator AppleScript;
- `AuthorizationExecuteWithPrivileges`;
- arbitrary root shell commands over XPC;
- trusting a caller-provided path after authorization.

## 6. Platform decision record

| Question | Decision |
|---|---|
| Minimum macOS | Keep 12.0 |
| Initial registrar | SMJobBless via Blessed |
| SMAppService | Deferred separate migration; no dual runtime fallback |
| Per-operation intent | Authorization Services custom rights |
| XPC identity | Code-signing/audit-token based through SecureXPC |
| Source handoff | Opened directory FD, not path |
| Root external commands | Prohibited |
| Production enablement | Formal Developer ID signed/notarized HIL only |
