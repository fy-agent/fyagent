# Upstream distribution and macOS research

Date: 2026-08-31

Method: official vendor/platform sources first; local read-only evidence and maintained open-source projects are supporting evidence. Community reports are not installer authority.

## 1. OpenAI Codex / current ChatGPT desktop

OpenAI's current migration guidance says existing Codex desktop users update normally and receive the newer ChatGPT desktop experience while retaining Codex access. ChatGPT Classic can coexist, so display-name matching is unsafe.

Local read-only evidence on the test Mac:

- `/Applications/ChatGPT.app`;
- Bundle ID `com.openai.codex`;
- version `26.825.51511`;
- Team ID `2DC432GLL2`;
- accepted as a notarized Developer ID application.

Engineering implications:

- retain stable identity-based discovery and Classic coexistence protections;
- do not add a broad `ChatGPT.app` name alias;
- remove the verified equal-or-newer implicit launch;
- provide explicit “打开软件” and native result-bearing launch diagnostics;
- the reported red warning remains unassigned because the original text/log was not captured.

Primary source:

- OpenAI Help Center, “Moving to the new ChatGPT desktop app”.

## 2. OpenCode

Official OpenCode distribution separates terminal and desktop surfaces. Desktop offers direct macOS Apple Silicon and Intel DMGs and a Homebrew cask option.

Official stable endpoints verified during research:

```text
https://opencode.ai/download/stable/darwin-aarch64-dmg
https://opencode.ai/download/stable/darwin-x64-dmg
```

Local evidence:

- `/Applications/OpenCode.app`;
- Bundle ID `ai.opencode.desktop`;
- version `1.18.19`;
- Team ID `5NZ4Q7NXJ4`;
- Gatekeeper accepted the bundle;
- LaunchServices/Spotlight resolves the Bundle ID to that path.

The last point proves macOS registration is healthy; it does **not** require FyAgent to adopt Launch Services as a new runtime scanner. Repository inspection shows the existing managed inventory already scans the actual path, but lacks an OpenCode desktop product policy.

Engineering implications:

- keep one `opencode` catalog product with `cli` and `desktop` surfaces;
- add OpenCode to the existing `/Applications`/`~/Applications` managed registry;
- reuse the shared structured bundle reader and exact target inventory;
- install official architecture-specific DMG through the shared artifact/DMG transaction;
- use official release metadata to freeze version/release identity where possible;
- no hidden Homebrew dependency or GitHub proxy fallback.

Primary sources:

- OpenCode official download page;
- OpenCode official documentation;
- official `anomalyco/opencode` releases/changelog.

## 3. Grok Build

Official xAI documentation and repository describe Grok Build as a terminal/TUI product, not a macOS `.app`.

Official CLI supports:

- `grok update`;
- `grok update --check`;
- `grok update --version <V>`;
- stable/alpha channel selection;
- version reporting.

Official native distribution behavior:

- `x.ai` primary host;
- `storage.googleapis.com/grok-build-public-artifacts` official binary fallback;
- proxy environment support;
- architecture/Rosetta detection;
- staged executable self-check;
- `~/.grok` internal layout, symlinks, config and PATH updates.

Official xAI material also identifies `@xai-official/grok` as an npm distribution that can avoid the native binary hosts by using an npm registry.

Engineering implications:

- model `NativeInternal` and `OfficialNpm` as separate distribution owners;
- native check/update uses the anchored official `grok update` interface;
- native fresh install uses the official installer;
- x.ai → GCS remains an official fallback **inside** the native owner;
- official npm is an explicit fresh-install/migration option through the existing package-manager owner;
- native failure never automatically invokes npm;
- every action has persistent job state, bounded output and post-version/owner readback;
- no dedicated official mainland-China mirror was found, so only official GCS, proxy configuration and explicit npm can be claimed before HIL.

Primary sources:

- xAI Grok Build enterprise/network documentation;
- xAI CLI reference;
- xAI official installer;
- official npm package;
- xAI Grok Build source repository.

## 4. Apple application launch and privileged file operations

Apple provides native application-opening APIs with asynchronous completion/error reporting. The task uses them only inside the existing FyAgent `process_launch` owner; it does not create a second launcher or a new global application registry.

For privileged file operations, Apple documents:

- `NSWorkspace.requestAuthorization`;
- authorization types including replace-file;
- authorized `FileManager`;
- `com.apple.developer.security.privileged-file-operations` entitlement.

A signed prototype is required because the documented method set does not, by itself, prove fresh absent-target application creation and full rollback semantics.

If native authorization is insufficient, mature helper references are available:

- Blessed (SMJobBless lifecycle);
- SecureXPC (typed authenticated XPC);
- SwiftAuthorizationSample (signing/version patterns);
- Mist (macOS 12+ production integration of Blessed/SecureXPC).

Engineering implication: select exactly one system-commit adapter—native first, reviewed helper only if necessary. Never use sudo, AppleScript elevation or arbitrary XPC.

## 5. Selected decisions

| Question | Decision |
| --- | --- |
| Is OpenCode one component? | One catalog product, two independent surfaces: CLI and Desktop. |
| Does OpenCode require a new scanner? | No. Add a product policy to the existing known-root inventory and reuse structured bundle readback. |
| Is Grok a desktop app? | No. Keep CLI/TUI lifecycle. |
| Is there a verified official mainland Grok mirror? | None found in official material as of 2026-08-31. |
| China-friendly Grok strategy | Native x.ai → official GCS; or user-explicit official npm via configured registry. No automatic owner switch. |
| How to launch desktop apps | Explicit action through the existing launch owner, with native completion diagnostics. |
| How to write `/Applications` | One system-commit port: signed Apple-native authorization first, Blessed/SecureXPC helper only if required. |
| How to handle Codex migration | Preserve stable Bundle identity and fail closed around ChatGPT Classic coexistence. |
