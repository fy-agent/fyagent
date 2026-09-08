# Security Threat Model

Date: 2026-08-31
Target: a root launchd helper capable only of committing known FyAgent-managed app bundles to fixed `/Applications` slots.

## 1. Security objective

Even if a malicious local process can observe public Mach service names, alter user-writable files, send arbitrary renderer input, or exploit part of the FyAgent UI/backend, it must not gain a general root primitive. The maximum intended authority is one user-authorized transaction affecting one known app slot, followed by exact verification or recovery.

## 2. Assets

- integrity and availability of `/Applications`;
- existing installed applications and backups;
- root helper executable/launchd configuration;
- Developer ID signing identity and requirement chain;
- Authorization external forms and user intent;
- source app identity and contents;
- transaction receipts/recovery state;
- FyAgent job/inventory truthfulness;
- user privacy and credentials.

## 3. Trust boundaries

```text
Untrusted renderer / webview
  | Tauri closed command
Trusted Rust backend (large user process)
  | private C ABI + source FD + auth session
Swift client bridge in same signed process
  | authenticated SecureXPC
Root helper (small privileged process)
  | fixed filesystem transaction
/Applications + root-private receipt directory
```

The Rust app is trusted more than the renderer but is not treated as infallible. Root helper handlers remain independently narrow and require operation authorization.

## 4. Attacker model

### A1 — Malicious renderer input

Can invoke exposed Tauri commands with unknown fields, stale target IDs, strings resembling paths/URLs/commands and repeated requests.

Defense:

- existing exact DTO parsers and opaque target authority;
- no helper-specific renderer command;
- complete target binding and fresh inventory revalidation;
- unknown/extra fields rejected before Swift bridge.

### A2 — Malicious same-user process

Can connect to a public Mach service, enumerate helper files, invoke embedded executables, race user files and spoof PIDs.

Defense:

- SecureXPC code-signing requirement based on actual peer/audit token;
- main-process library client, no standalone generic client CLI;
- per-operation Authorization right;
- opened source directory FD rather than path;
- operation UUID/freshness/one-shot right;
- helper does not trust PID or service name alone.

### A3 — User-writable source replacement / TOCTOU

Can replace a staged path after client validation, swap symlinks, modify nested files or point a path at another tree.

Defense:

- `O_DIRECTORY|O_NOFOLLOW` source capability;
- FD transfer over XPC;
- fd-relative fixed-component reads;
- source file identity/revision before and immediately before copy;
- reject special files/symlink escape;
- stage/installed reinspection and equivalence checks.

### A4 — Compromised/buggy FyAgent process

Can call every private bridge function and form syntactically valid requests.

Defense in depth:

- helper supports only a tiny product/slot/action enum;
- destination strings are not caller-controlled;
- fresh admin right checked inside helper;
- source must be a valid known app capability;
- helper has no shell/network/generic file API;
- one request performs one transaction;
- helper logs/receipt permit diagnosis.

Residual: a fully compromised signed FyAgent process plus user-granted admin authorization can replace one allowlisted product slot with another bundle that passes the closed product policy. Product policy and source validation therefore remain security-critical and require separate tests/review.

### A5 — Old vulnerable FyAgent downgrade

Can launch an old signed app that previously satisfied broad helper requirements.

Defense:

- helper `SMAuthorizedClients` and runtime requirement include minimum safe app version;
- helper version increments with security releases;
- newer helper rejects older app protocol/version;
- no helper downgrade/equal overwrite.

### A6 — Helper replacement/tampering

Can alter installed/bundled helper or route client to a different service.

Defense:

- client verifies helper Team/identifier/version/hardened runtime;
- formal verifier checks all architectures and nested code;
- helper status compares bundled/installed metadata and authenticated health;
- tampered state blocks system action; no auto-overwrite under ambiguous state.

### A7 — Crash/power loss during commit

Can leave target moved to backup or new target partially committed.

Defense:

- same-volume generated stage/backup;
- root-private fsynced versioned receipt;
- atomic rename commit;
- startup/request recovery state machine;
- exact identity checks before removal/restore;
- `recovery_required` blocks further writes when certainty is lost.

### A8 — Replay or authorization theft

Can resend a captured request/external form.

Defense:

- external form never leaves authenticated XPC/log-free memory;
- helper reconstructs and rechecks right immediately;
- rights destroyed at terminal state;
- canonical operation UUID and bounded in-process replay set;
- source FD is per-request and closes at terminal;
- target/source revision must still match;
- future persistent replay ledger is added only if HIL demonstrates restart replay remains possible after right destruction.

### A9 — Product policy drift

Rust and Swift disagree on Bundle ID, target basename or version rule.

Defense:

- one policy source generates both projections;
- deterministic snapshot/drift tests;
- protocol carries enum IDs, not duplicated strings;
- helper revalidates source/stage/target using generated policy.

### A10 — Supply-chain compromise

Dependency source/tag changes or malicious prebuilt artifact enters release.

Defense:

- source-only exact pins/Package.resolved;
- official repository allowlist;
- dependency source audit and license inventory;
- no runtime download;
- build nested code in trusted formal runner;
- inside-out signature seals exact compiled bytes;
- update requires review, not automated floating dependency bump.

## 5. Forbidden capability matrix

| Capability | Renderer | Rust backend | Swift bridge | Root helper |
|---|:---:|:---:|:---:|:---:|
| Arbitrary path input | no | no wire authority | no | no |
| Arbitrary URL/network | no | existing source owner only | no | no |
| Arbitrary command/process | no | no helper call | no | no |
| Product display-name matching | no | no | no | no |
| Fixed product enum | typed | yes | yes | yes |
| Opaque target/revision | yes | authority owner | private projection | closed target slot only |
| Source directory FD | no | creates | transfers | validates/reads |
| Authorization external form | no | lifetime coordination only | creates/transfers | verifies/destroys |
| `/Applications` mutation | no | no direct write | no | one closed transaction |
| Root recovery receipt | no | status only | status only | owns |

## 6. Protocol invariants

- exact protocol version and fixed keys;
- maximum request/reply size;
- canonical UUID operation ID;
- all reserved bytes zero;
- one source directory FD exactly;
- no string capable of expressing a path/URL/command/destination;
- closed product/action/slot enums;
- revisions fixed-length and validated;
- one terminal reply; no post-terminal messages;
- connection invalidation closes source FD and clears authorization;
- terminal snapshot immutable;
- error strings bounded and path-redacted.

## 7. Filesystem invariants

- target parent exactly `/Applications` after canonical/root FD validation;
- target basename from generated policy only;
- direct child only;
- parent and generated paths not symlinks;
- stage/backup absent before creation;
- root-owned generated artifacts with restrictive mode;
- source/stage/target fixed product identity and version relationship;
- update target revision revalidated immediately before rename;
- only exact expected replacement can be deleted;
- only exact expected backup can be restored/removed;
- parent and receipt fsync at transaction boundaries;
- bounded receipt count and version.

## 8. Authorization invariants

- custom right represents an individual operation class;
- app requests right immediately before helper call;
- helper checks right immediately before mutation with no helper-side UI;
- cancellation/denial is distinct from transport failure;
- external form never logged/persisted;
- rights destroyed after operation;
- helper install right and app-commit right are not interchangeable;
- rollback is internal compensation under the already authorized commit.

## 9. Security test program

### Static/contract

- forbidden field/name scans across Rust, TypeScript, Swift and generated headers;
- exact enum/version/reserved-bit parsing;
- dependency URL/pin/license allowlist;
- no `Process`, `system`, `popen`, shell, network client or generic filesystem route in helper target;
- product policy drift scan.

### Unit/fuzz

- malformed/truncated/oversized XPC Codable payloads;
- unknown enums, duplicate routes, extra fields;
- Authorization serialization length and invalid external forms;
- file descriptor type/ownership/closure races;
- receipt corruption and phase transitions;
- path component/symlink/hard-link/special-file corpus.

### Integration

- wrong signed client/server fixtures;
- old/new app/helper versions;
- source path swap after FD open;
- stale target revision;
- simulated copy/rename/fsync errors;
- kill helper at each commit phase;
- fresh inventory mismatch after helper returns success.

### HIL

- real admin authorization dialogs and cancellation;
- SMJobBless install/update/disabled helper;
- launchd root context;
- Developer ID/notarized peer requirements;
- real `/Applications` filesystem semantics and metadata;
- crash recovery and removal.

## 10. Residual risks

- SMJobBless is deprecated and may require migration in a future macOS release.
- SecureXPC/Blessed are stable but lightly maintained; exact source pins and local tests become important ownership.
- A root helper is persistent high-impact code; every new product/slot expands its authority and requires policy/security review.
- A fully compromised current signed app plus fresh admin authorization can exercise all intended helper routes.
- Portable fake-filesystem tests cannot prove APFS metadata, launchd, code-signing or Authorization Services behavior.

These residuals are accepted only with the narrow protocol, formal HIL and a documented future SMAppService migration trigger.
