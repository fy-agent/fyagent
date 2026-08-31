# macOS `/Applications` authorization options

Date: 2026-08-31

Scope: design gate only. No authorization implementation is claimed complete.

## 1. One system-commit port

All desktop transactions depend on one closed interface:

```text
MacSystemCommitPort
  preflight(operation capability)
  commit(operation capability)
  rollback(operation capability)
  status()
```

The coordinator and renderer do not know whether the production adapter is Apple-native authorization or a privileged helper. After feasibility work, exactly one adapter is compiled/registered for production.

## 2. Gate A — Apple-native authorization first

Apple documents on supported macOS releases:

- `NSWorkspace.requestAuthorization` for privileged file operations;
- `NSWorkspaceAuthorizationTypeReplaceFile`;
- authorized `FileManager`;
- `com.apple.developer.security.privileged-file-operations` entitlement.

This is preferred because it is a typed platform API and may reuse the existing managed transaction without a persistent privileged service.

A signed/notarized prototype for the actual FyAgent Developer ID must prove:

1. entitlement request/provisioning/package preservation;
2. fresh commit when `/Applications/<App>.app` does not yet exist;
3. exact replacement of an existing app;
4. rollback or compensating restore;
5. cancel/deny/expired authorization leaves target/staging known;
6. closed Rust/Objective-C bridge with no renderer path API;
7. macOS 12 and current supported macOS HIL.

SDK headers currently document only a limited authorized FileManager method set, so fresh create cannot be assumed from documentation. This is a real decision gate.

If all required operations pass, the native adapter is selected and no helper is introduced.

## 3. Gate B — reviewed helper only when Gate A is insufficient

If native authorization cannot perform fresh create or preserve transaction invariants, evaluate one helper adapter under the same port.

Because FyAgent supports macOS 12, a macOS-13-only SMAppService path cannot be the sole solution. Reviewed mature components/references:

- Blessed: SMJobBless lifecycle/error wrapper;
- SecureXPC: typed authenticated XPC, including privileged helpers;
- SwiftAuthorizationSample: signing, version and downgrade patterns;
- Mist: production macOS 12+ reference using Blessed and SecureXPC.

The helper protocol remains FyAgent-specific but closed:

```text
query_helper_status()
commit_known_application(operation_id, revision)
rollback_known_application(operation_id, revision)
```

Security requirements:

- `operation_id` resolves only to a short-lived backend-owned protected manifest;
- closed product enum maps to a fixed `/Applications/<basename>.app`;
- mutual code-signing requirements and helper version/downgrade checks;
- replay/expiry/containment/symlink/source/target revision validation;
- no arbitrary path, URL, command, executable, destination, copy/delete or bypass fields;
- root process does not network, parse remote metadata, access user TCC folders or show GUI;
- bounded, path-redacted replies/logs.

Helper installation/XPC/signing plumbing should reuse the reviewed components. Do not hand-roll SMJobBless or a generic XPC protocol.

## 4. Selection outcomes

The task records one honest result:

- **Native selected:** entitlement + authorized FileManager supports all required operations and signed HIL passes.
- **Helper selected:** native is insufficient, Blessed/SecureXPC adapter passes dependency review, signing/notarization, peer authentication, transaction and macOS 12/current HIL.
- **Blocked:** neither path safely satisfies the contract; `/Applications` automatic action remains disabled/manual.

Production must not keep native and helper as a runtime fallback chain.

## 5. Prohibited shortcuts

- `sudo`;
- `osascript ... with administrator privileges`;
- deprecated `AuthorizationExecuteWithPrivileges`;
- generic root XPC/file manager;
- renderer-provided path/URL/command;
- install to `~/Applications` after authorization error while reporting system success;
- unsigned development proof substituted for signed/notarized HIL.
