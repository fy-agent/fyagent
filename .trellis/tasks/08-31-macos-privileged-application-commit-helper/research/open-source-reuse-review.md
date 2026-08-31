# Open-source Reuse Review

Date reviewed: 2026-08-31
Method: official repositories, source code, Package.swift/project pins, licenses and current product usage were inspected. README claims were compared with source.

## 1. Selected packages

### Blessed

Repository: https://github.com/trilemma-dev/Blessed

License: MIT

Latest tag observed: `0.6.0`
Reviewed HEAD: `fc7c930f3cc1894a2481e1f884c336007712e6a8` (README-only changes after the tag)

Reusable capability:

- wraps Authorization Services + `SMJobBless` behind `authorizeAndBless` / `bless`;
- validates and explains the demanding helper/app plist, signature, executable and version requirements through `BlessError`;
- uses EmbeddedPropertyList and Required for helper metadata/requirement diagnostics;
- recognizes that `SMAuthorizedClients` controls installation/update authority, not runtime communication.

Not reusable:

- product operation policy;
- helper health/readback semantics;
- root file transaction;
- Rust/Tauri integration.

Assessment:

- mature, small, source-available, MIT;
- not actively changing, but the latest tagged implementation remains used by an active macOS 12+ product (Mist);
- exact `0.6.0` pin is preferred over floating source;
- `SMJobBless` deprecation is an Apple-platform residual, not a package-specific hidden fallback.

Decision: **adopt for helper installation/update authorization only**.

### SecureXPC

Repository: https://github.com/trilemma-dev/SecureXPC

License: MIT

Latest tag observed: `0.8.0`
Reviewed HEAD: `1cece54562c7626d042f007d2f38cfe325565850`

Reusable capability already present in tag `0.8.0`:

- typed Codable routes;
- SMJobBless/SMAppService Mach service criteria;
- code-signing checks based on the real calling process;
- `SecCodeCreateWithXPCMessage`/audit-token-oriented validation;
- `FileDescriptorForXPC` and `FileHandleForXPC` using XPC FD duplication;
- client/server requirement APIs;
- bounded error/connection handling primitives.

Post-0.8.0 HEAD changes include:

- stronger/default hardened-runtime client requirements;
- executable-path handling fixes;
- sequence interruption/race fixes;
- documentation/diagnostics improvements.

Assessment:

- this package removes the highest-risk wheel: custom XPC routing plus caller identity validation;
- exact HEAD revision is a reasonable initial candidate because the security/reliability fixes are untagged;
- implementation must refresh upstream and prefer a newer exact release tag if one includes the reviewed fixes;
- no binary dependency, no transitive packages.

Decision: **adopt for typed authenticated XPC and source FD transfer**; exact tag/revision gate required.

### Authorized

Repository: https://github.com/trilemma-dev/Authorized

License: MIT
Reviewed tag/HEAD: `1.0.0` / `e490b9d3f4a0e8b17a8b39b5a9750b8e0be7548a`

Reusable capability:

- Swift wrapper for non-deprecated Authorization Services APIs;
- custom right definition using canned `authenticate-admin` rule;
- Authorization request/check/destroy lifecycle;
- Codable `Authorization` based on protected 32-byte external form;
- async request API usable on the project's minimum macOS 12.

Limits:

- does not define FyAgent's right names or product policy;
- does not make external forms safe if callers log/leak them;
- system credential caching remains controlled by macOS.

Decision: **adopt directly for per-operation authorization**; exact `1.0.0` pin.

### Transitive packages

| Package | Reviewed tag/commit | License | Use |
|---|---|---|---|
| EmbeddedPropertyList | `2.0.2` / `21bd832e28a9a66ecdb7b4c21910bb0487a22fe5` | MIT | read embedded helper info/launchd plists |
| Required | `0.1.1` / `82a4fbd388346ca40b1bbe815014dc45a75d503c` | MIT | evaluate code-signing requirements for diagnostics |

They enter only through exact Swift package resolution; no runtime plugin loading.

## 2. Reference projects

### SwiftAuthorizationSample

Repository: https://github.com/trilemma-dev/SwiftAuthorizationSample

License: MIT
Reviewed commit: `85f45622f819ca5b5dcf8867801a6b5d3edf63b2`

Good references:

- `SMPrivilegedExecutables` / `SMAuthorizedClients` generation;
- helper embedded info/launchd plist;
- minimum client version in requirement to mitigate downgrade;
- helper install-state as a multi-part condition;
- helper update/removal examples;
- per-command custom authorization right;
- explicit warning that SecureXPC client restrictions and operation minimization are both required.

Rejected as direct implementation:

- sample command execution surface;
- path-based update route;
- auto-incrementing source property lists during build;
- Xcode 13-era project structure as a frozen template.

Decision: **reference only**.

### Mist

Repository: https://github.com/ninxsoft/Mist

License: MIT
Reviewed commit: `aed0e49a307d7630a139f8876a9b2651be79f4b8` (2026-07-07)

Evidence value:

- active product supports macOS 12+;
- still embeds helper at `Contents/Library/LaunchServices`;
- uses Blessed from `0.6.0` and SecureXPC from `0.8.0`;
- has real `SMPrivilegedExecutables`, `SMAuthorizedClients`, helper status UI and universal product build context.

Rejected business code:

- helper request includes arbitrary path/arguments for multiple operations;
- default route can execute broad commands;
- status inspection shells out to `launchctl`;
- operations and file semantics are Mist-specific.

Decision: **production packaging/reference only; copy no business routes**.

## 3. Alternatives reviewed

### SMAppService sample apps

Examples such as `alienator88/HelperToolApp` demonstrate modern daemon registration and System Settings approval, but:

- require macOS 13+;
- often expose a root `runCommand`/bash surface;
- do not solve FyAgent's macOS 12 compatibility or closed app transaction.

Decision: not selected for this task. Keep as future migration references only.

### Microsoft ProcexpForMac

Repository: https://github.com/microsoft/ProcexpForMac

Useful modern reference:

- SMAppService daemon bundle layout;
- Developer ID versus local self-signed testing;
- inside-out signing and explicit System Settings approval.

Limit:

- macOS-13+ registration model and product-specific operations.

Decision: future SMAppService/release verifier reference, not initial registrar.

### New/unproven helpers

Other repositories advertise code-signed XPC or whitelisted commands but may target macOS 13/14+, expose many root primitives, lack tests, or have limited adoption. None provides a safer drop-in known-app replacement transaction than the selected focused packages plus FyAgent's existing transaction owner.

Decision: do not expand the dependency set.

## 4. Reuse matrix

| Need | Owner |
|---|---|
| Helper bless/install error handling | Blessed |
| Authorization wrapper/custom rights | Authorized |
| Typed XPC / peer validation / FD transfer | SecureXPC |
| Requirement/build/update patterns | SwiftAuthorizationSample reference |
| macOS 12 production packaging evidence | Mist reference |
| Modern macOS 13 migration evidence | Apple docs + ProcexpForMac reference |
| Download/source resolution/DMG mount | existing FyAgent Rust owner |
| Product/target authority | existing FyAgent backend owner |
| Root known-app commit/rollback | minimal FyAgent-specific helper operation |
| Job/readback/UI | existing FyAgent Agent/Codex lifecycle |

## 5. Supply-chain requirements

- exact package resolution committed;
- source-only dependencies; no prebuilt binary artifacts;
- dependency URLs restricted to reviewed official repositories;
- resolved commit/tag included in build evidence;
- license files/NOTICE updated;
- package update requires explicit security and compatibility review;
- release build fails if lockfile or expected revision drifts;
- Swift dependency build occurs before formal nested signing and is sealed by the final app signature;
- no dynamic plugin loading or runtime package download.

## 6. Final review outcome

The selected stack saves substantial low-level work without delegating FyAgent's authority:

- **reuse:** helper installation, Authorization wrapper, typed XPC, process identity and FD transfer;
- **own narrowly:** product/slot policy, operation authorization names, exact transaction and recovery;
- **reject:** generic root commands, arbitrary paths, copied sample business operations, floating dependencies and dual registrar fallback.

This is the smallest mature reuse set found that supports macOS 12 and the required one-step administrator authorization model.
