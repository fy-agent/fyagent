# Reuse and mature-solution assessment

Research date: 2026-08-31

## Decision table

| Problem | Mature options evaluated | Selected reuse | Reason |
| --- | --- | --- | --- |
| HTTP artifact transfer | Existing Codex streaming core; new Rust crate; curl subprocess | Extract/delegate existing Codex core | It already owns bounded redirects, cancellation, retry, protected temp files, progress and tests. A new crate/process would create a second owner. |
| DMG mount/replace/rollback | Existing managed exact transaction; Homebrew; new hdiutil flow | Existing managed exact transaction | Exact-target update, rollback and cleanup already exist. Do not duplicate them. |
| Installed desktop discovery | Existing `/Applications` + `~/Applications` inventory; Launch Services/global scan; recursive filesystem scan | Extend existing inventory only | The verified OpenCode miss is caused by absent product policy, not an unsearched location. Expanding scan scope adds ambiguity and is out of task scope. |
| Bundle metadata | Existing Codex bounded `plutil -> JSON`; Foundation `NSBundle`; new plist crate; hand XML parser | Extract existing Codex structured reader | It is already adopted, supports binary/XML plist and avoids another runtime/parser dependency. |
| Desktop application launch | Existing process-launch owner with `/usr/bin/open`; separate launcher; NSWorkspace adapter | Keep process-launch owner, replace its macOS internals with NSWorkspace completion | Preserves one business boundary while improving result/error evidence. |
| `/Applications` authorization | NSWorkspace authorization + authorized FileManager; SMJobBless/SMAppService helper; AppleScript/sudo | One system-commit port: native signed spike first, reviewed helper only if necessary | Native API adds least infrastructure. If it cannot create/replace safely, reuse Blessed + SecureXPC rather than inventing a helper framework. |
| OpenCode Desktop install | Official direct DMG/release asset; Homebrew cask command; third-party mirror | Official asset through shared pipeline | No Homebrew runtime dependency; same target/progress/error semantics as other managed desktop products. |
| Grok native install/update | Official xAI installer/updater; custom Rust installer | Official native tools behind persistent typed jobs | Vendor tools own channel, architecture/Rosetta, official GCS fallback, layout, symlinks, config and validation. |
| Grok npm install/update | Existing package-manager adapter; custom npm wrapper | Existing anchored package-manager owner + official package | npm is an independent official owner and must be user-selected, not an automatic native failure fallback. |
| Transfer speed | Existing Codex TypeScript projector; new npm package; backend guessed rate | Extract existing projector | It already handles byte/time sampling; backend only needs raw telemetry. |

## Existing project capabilities to preserve

The repository already uses the required building blocks:

- `reqwest` streaming, `tokio`, `futures`, `bytes`;
- protected temporary artifacts and Codex download progress;
- managed macOS DMG selection/replacement/rollback;
- authoritative Agent inventory and opaque target revisions;
- `platform::process_launch` as the fixed business launch boundary;
- bounded structured plist reading in the Codex macOS adapter;
- Tooling executable/package-manager observation;
- Codex frontend speed projection.

The task should mostly extract, parameterize or add a product policy. It should not introduce a general installer framework.

## Apple-native system commit

Primary Apple capabilities evaluated:

- `NSWorkspace.requestAuthorization`;
- `NSWorkspaceAuthorizationTypeReplaceFile`;
- authorized `FileManager`;
- `com.apple.developer.security.privileged-file-operations` entitlement.

SDK headers show the authorization-backed FileManager supports a narrow set of operations, including replacement. Documentation and headers alone do not prove a fresh, absent `/Applications/<App>.app` can be committed while preserving the existing transaction. Therefore a signed/notarized prototype is mandatory.

If the prototype fully supports fresh create, exact replacement, cancellation and rollback, it is the only production adapter. If not, the same `MacSystemCommitPort` may use a reviewed helper.

## Reviewed helper reuse, conditional only

The project minimum is macOS 12, so a macOS-13-only `SMAppService` implementation cannot be the sole route. Mature references:

- **Blessed**: MIT wrapper around SMJobBless lifecycle and errors;
- **SecureXPC**: MIT typed/authenticated XPC supporting privileged helpers;
- **SwiftAuthorizationSample**: code-signing, version and authorization patterns;
- **Mist**: MIT production macOS 12+ application that integrates Blessed and SecureXPC.

These projects provide helper installation/XPC/signing primitives. FyAgent still defines its own *closed business protocol* (`operation_id + revision`), but must not recreate SMJobBless/XPC plumbing. Exactly one production system-commit adapter remains after the spike.

Rejected shortcuts:

- `sudo`;
- AppleScript administrator prompt;
- deprecated AuthorizationExecuteWithPrivileges;
- arbitrary root file manager/XPC;
- keeping both native and helper paths as active fallbacks;
- silently writing `~/Applications` after authorization failure.

## OpenCode Desktop

Official distribution provides independent terminal and desktop surfaces, including direct macOS Apple Silicon and Intel DMGs. The test Mac confirms the installed desktop Bundle ID is `ai.opencode.desktop`.

Runtime design:

- add a desktop surface under existing `AgentCatalogId::OpenCode`;
- add one managed desktop product policy;
- keep known root inventory;
- use shared structured plist readback;
- use official release/DMG source through shared artifact transport;
- do not invoke Homebrew or a GitHub proxy as hidden fallback.

## Grok Build

Two official distribution owners are modeled separately:

### Native/internal

- fresh install: xAI official installer;
- check/update: anchored `grok update --check` and `grok update --version <V>`;
- x.ai primary and xAI-declared GCS fallback remain inside the native owner;
- FyAgent captures persistent job state, output, timeout and post-version/owner readback.

### Official npm

- explicit fresh-install option or explicitly approved migration;
- existing anchored package-manager adapter;
- package `@xai-official/grok` and configured npm registry;
- npm-owned update remains npm-owned.

Native failure must not automatically execute npm. This preserves user layout, PATH, symlinks and installer ownership while still offering a China-friendly official alternative.

## Solutions explicitly rejected

- random GitHub proxy or anonymous “China mirror”;
- a new Launch Services/global disk scanner for the verified OpenCode case;
- a new plist parser or Objective-C bridge solely for bundle metadata;
- `curl | bash` from renderer or generic shell IPC;
- copying xAI installer algorithms into Rust;
- page-specific speed/percent algorithms;
- automatic Grok distribution-owner conversion.
