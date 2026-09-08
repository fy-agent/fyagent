# Official source and network strategy

Research date: 2026-08-31

Policy: official-first, product-scoped, distribution-owner-aware, fail closed. No anonymous proxy is enabled by default.

## 1. Common source contract

Every installable product keeps a backend-owned policy for:

- metadata endpoints and ordered source categories;
- initial/redirect host allowlists;
- platform, architecture and format classification;
- version grammar and release capability;
- operational size/timeout/retry bounds;
- post-download handoff validation;
- post-install local identity/version readback;
- safe diagnostics.

A mirror changes transport location, not identity or target authority. Renderer input never includes a URL, host, package format, command or validation bypass.

Recommended UI source states:

```text
unchecked
checking
official_primary
official_fallback
official_npm
configured_mirror
unreachable
ambiguous
identity_rejected
```

Raw URLs with credentials/query strings are not exposed.

## 2. Grok Build

### 2.1 Native/internal owner

Official xAI materials and installer establish:

- native install/update uses `x.ai`;
- CLI binary fallback uses `storage.googleapis.com/grok-build-public-artifacts`;
- stable/alpha/enterprise channel semantics;
- architecture and Rosetta handling;
- compressed/raw artifact fallback;
- executable self-check;
- `~/.grok` internal layout, symlinks, config and PATH behavior.

FyAgent therefore does not implement another native installer. It creates a frozen, typed job around official actions:

```text
check: anchored grok update --check
update: anchored grok update --version <frozen-version>
fresh: protected local copy of fixed official installer, fixed argument shape
verify: rediscover executable, version and distribution owner
```

The native tool remains responsible for x.ai → official GCS transport fallback. FyAgent records stage/source category where observable, timeout, exit status, bounded redacted output and final readback.

### 2.2 Official npm owner

xAI also documents `@xai-official/grok` as an official distribution route that can use the configured npm registry and avoid the native download hosts.

FyAgent behavior:

- present npm as an explicit fresh-install option, not an invisible fallback;
- use the existing anchored package-manager adapter;
- freeze the selected official package version;
- verify final command/version and npm ownership;
- keep npm updates within the npm owner.

A failed native job can offer a new “use official npm method” action, but it does not execute it automatically.

### 2.3 Mainland China interpretation

No independent official mainland-China mirror was found. Conservative options are:

- native x.ai primary + xAI-declared official GCS fallback;
- standard proxy environment (`HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`) used by official tools;
- explicit official npm distribution using the user's/enterprise registry.

Do not:

- label a random proxy as official;
- hardcode a community mirror;
- route deployment/auth credentials through artifact mirrors;
- migrate a native installation to npm after an update error;
- claim reachability without mainland-network HIL.

## 3. OpenCode Desktop

Official stable desktop endpoints verified during research:

```text
https://opencode.ai/download/stable/darwin-aarch64-dmg
https://opencode.ai/download/stable/darwin-x64-dmg
```

The preferred runtime resolver still freezes an official release identity and exact asset using official metadata when available. The stable endpoints may be used only under a reviewed binding/cache strategy or as HIL references.

Requirements:

- official OpenCode/repository metadata and artifact hosts only;
- macOS + arm64/x64 + DMG unique selection;
- no hardcoded research-time version;
- ambiguous/missing asset fails closed;
- no automatic GitHub proxy or Homebrew command;
- shared artifact transport and existing DMG transaction;
- local post-install readback through shared structured bundle metadata;
- Bundle ID `ai.opencode.desktop` and executable shape verified locally.

Existing executable-installer policy remains: remote digest, size, Team ID, signature or publication version are not added as downloaded-content admission comparisons merely because upstream publishes them.

## 4. QoderWork, TRAE Work and WorkBuddy

Keep current product-specific official metadata/source parsers unless Phase 0 proves a maintained vendor API can replace one completely.

Required changes:

- move artifact transfer to shared streaming core;
- review existing metadata/redirect/artifact host lists;
- remove full-memory DMG and duplicate temp write;
- use shared structured bundle readback;
- commit to exact target through one system-commit port;
- project consistent source/job status.

Do not create a permissive “any Electron DMG” resolver.

## 5. Codex

Codex keeps its dedicated release/mirror policy and product port. Shared extraction is limited to transport, protected temp artifact, progress and managed DMG primitives.

Must preserve:

- checked-release/session authority;
- release ID binding;
- mirror trust/redirect rules;
- platform/architecture selection;
- Stable application identity and post-install readback;
- cancellation, single-flight and cleanup.

The task also removes the equal-or-newer implicit launch, but does not generalize Codex into renderer-controlled Agent installation.

## 6. Optional configured mirror model

This task does not build a mirror service or UI. A future operator-configured mirror, if needed, must be product-scoped:

```text
mirror id
product/distribution owner
metadata or artifact base
HTTPS host allowlist
release mapping
existing identity policy reference
operator provenance
review/expiry date
enabled=false by default
```

Constraints:

- no renderer URL;
- no wildcard host by default;
- no HTTP downgrade;
- no auth forwarding unless the product policy explicitly owns it;
- unchanged post-install identity/readback;
- UI label “configured mirror”, never “official”.

## 7. Network HIL matrix

Record date, network class and selected owner/source without public IPs or credentials.

| Product/owner | Normal network | Mainland network | Primary blocked | Official fallback/alternative | Slow/interrupted |
| --- | --- | --- | --- | --- | --- |
| Grok native | required | required | required | GCS required | required |
| Grok official npm | required | required | n/a | configured registry | required |
| OpenCode Desktop | required | required | required failure UX | no hidden proxy | required |
| Codex | regression | required | existing policy | existing policy | required |
| Qoder/Trae/WorkBuddy | regression | required | product-specific | product-specific | required |

Pass requires correct owner/source state, bounded timeout, terminal diagnostics and post-action readback—not merely “some installation eventually appeared”.
