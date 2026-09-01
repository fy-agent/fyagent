# Upstream and Source Evidence

Date reviewed: 2026-08-31

Source priority:

1. Anthropic/OpenCode official documentation and official release repositories;
2. actual current installer metadata/bundle inspection;
3. maintained open-source mirror implementation as a transport/provenance reference;
4. no blog, anonymous proxy or guessed endpoint is installer authority.

All observed versions/dates below are one-time planning evidence. Production code must resolve current releases dynamically.

## 1. Claude Desktop official product evidence

### One desktop app contains Claude Code

Anthropic’s current download page states that Chat, Claude Cowork and Claude Code are available “all in one place,” and specifically confirms that Claude Code runs directly in the desktop app.

Primary sources:

- https://claude.com/download
- https://code.claude.com/docs/en/desktop

Decision: remove Claude Code CLI from the Agent lifecycle install surface and manage `Claude.app` as the physical desktop component. Retain the existing `claude-code` product/configuration ID.

### macOS distribution

Anthropic’s enterprise deployment documentation states:

- Claude Desktop installs to `/Applications` under managed deployment;
- `.pkg` and `.dmg` formats are available;
- the universal build supports x64 and arm64;
- the application can update automatically unless enterprise policy disables it;
- an app under `/Applications` needs suitable administrator/write permission to update, while `~/Applications` is user-writable.

Primary source:

- https://support.claude.com/en/articles/12611117-deploy-claude-desktop-for-macos

Decision: FyAgent may provide its own explicit one-click update using the existing target/DMG/helper transaction, while keeping install/update separate from launch.

### Region boundary

Anthropic publishes an explicit list of locations where Claude is accessible. The current list contains Taiwan and many other regions but does not list China mainland.

Primary source:

- https://support.claude.com/en/articles/8461763-where-can-i-access-claude

Decision: a China-friendly installer mirror is a transport feature only. UI/docs must not claim that it enables account registration, login, service calls or circumvents Anthropic’s regional policies.

## 2. Claude installer upstream endpoints

The reviewed current official update/download identity is:

```text
https://api.anthropic.com/api/desktop/darwin/universal/dmg/latest/redirect
```

It redirects to a versioned artifact on `downloads.claude.ai`.

The public website’s `claude.ai/api/...` link returned HTTP 403 to the non-browser planning probe, while the `api.anthropic.com` endpoint is the machine endpoint documented by the reviewed mirror. The task does not depend on scraping the website link.

## 3. Reviewed Claude mirror

### Repository

```text
repository: https://github.com/Wangnov/claude-app-mirror
license: MIT
reviewed commit: a21125ce29b1275c405eddb209e5f69bd2444fe6
commit date observed: 2026-07-08
```

The project’s declared scope is narrow:

- resolve official Anthropic latest endpoints;
- download current official installers without rebuilding/modifying/repackaging;
- publish versioned GitHub Release assets;
- copy latest assets to Cloudflare R2 short links;
- publish checksums and a release manifest;
- do not bypass installer or authorization logic.

The repository says a Cloudflare trigger runs every 15 minutes and the GitHub schedule runs every six hours as a fallback. This is project documentation, not a runtime SLA for FyAgent.

Primary source:

- https://github.com/Wangnov/claude-app-mirror/blob/main/README.md

### Fixed endpoints selected for FyAgent planning

```text
manifest: https://claudeapp.agentsmirror.com/latest/manifest
macOS:    https://claudeapp.agentsmirror.com/latest/mac
```

These are product-specific code-owned endpoints. They must not become user-configurable or a generic proxy API.

### Current manifest observation

The fixed manifest returned HTTP 200 and an exact v2 branch:

```json
{
  "schemaVersion": 2,
  "generatedAt": "2026-08-28T17:22:34Z",
  "version": "1.40609.0",
  "sources": {
    "macos": {
      "universal": {
        "platform": "darwin",
        "arch": "universal",
        "format": "dmg",
        "redirect": "https://api.anthropic.com/api/desktop/darwin/universal/dmg/latest/redirect",
        "version": "1.40609.0",
        "contentLength": 351027572,
        "assetName": "Claude-mac-universal.dmg"
      }
    }
  }
}
```

The actual manifest also contains upstream URL, filename, build hash, ETag, Last-Modified and SHA-256. Per the existing executable-installer contract, those fields must not become renderer/download capabilities or downloaded-content admission comparisons. The task parser retains only flow fields and an optional size hint.

### Current DMG observation

The fixed `/latest/mac` endpoint returned:

```text
HTTP: 200
Content-Type: application/x-apple-diskimage
Content-Length: 351027572
```

The current DMG was downloaded to an ephemeral planning directory, attached read-only, inspected and removed. Results:

```text
exact top-level app: Claude.app
CFBundleIdentifier: com.anthropic.claudefordesktop
CFBundleShortVersionString: 1.40609.0
CFBundleVersion: 1.40609.0
CFBundleExecutable: Claude
LSMinimumSystemVersion: 12.0
architectures: x86_64 arm64
Developer ID Team ID: Q6L2SF6YDW
Gatekeeper: accepted, Notarized Developer ID
codesign --verify --deep --strict: passed
```

No downloaded installer or mounted image remains from the inspection.

### Trust decision

Adopt the fixed mirror endpoints as a reviewed transport/cache for this one product, following the already-established AgentsMirror pattern. Do not import its CI/shell scripts into FyAgent.

The package remains governed by:

- fixed endpoint selection in Rust;
- bounded manifest parser;
- existing streamed downloader;
- exact single-app DMG transaction;
- closed local Bundle ID/version product routing;
- operating-system trust/install result;
- post-install inventory readback.

If the mirror’s repository ownership, license, provenance, endpoint or behavior changes materially, disable the managed source and retain only official-page fallback until a new review is complete.

## 4. OpenCode official evidence

### Official distribution

OpenCode’s official download surface separates terminal and desktop products and publishes architecture-specific macOS desktop downloads.

Primary sources:

- https://opencode.ai/download
- https://github.com/anomalyco/opencode
- https://github.com/anomalyco/opencode/releases

Current fixed desktop endpoints already present in FyAgent:

```text
https://opencode.ai/download/stable/darwin-aarch64-dmg
https://opencode.ai/download/stable/darwin-x64-dmg
```

### Current release observation

The official GitHub latest release API returned, at review time:

```text
tag: v1.18.25
published: 2026-08-28T05:58:20Z
arm64 DMG: opencode-desktop-mac-arm64.dmg
x64 DMG: opencode-desktop-mac-x64.dmg
latest-mac.yml present
```

The exact asset URLs and digests are date-specific publication evidence, not production pins.

### Local installed app observation

The test Mac currently has:

```text
application: /Applications/OpenCode.app
CFBundleIdentifier: ai.opencode.desktop
version: 1.18.19
Developer ID Team ID: 5NZ4Q7NXJ4
```

This proves the current scan/update use case: the installed app is older than the reviewed official latest release at the time of planning.

### Upstream updater boundary

The OpenCode desktop source tree/release assets contain Electron updater metadata and updater code. It is useful evidence for official release/version naming, but it is not selected as FyAgent’s executor.

Calling the upstream updater would bypass FyAgent’s:

- job/progress/cancellation;
- target selection and location preservation;
- helper authorization for `/Applications`;
- rollback/recovery transaction;
- authoritative post-install reread.

Decision: reuse official release metadata and fixed DMG distribution, not the upstream update implementation.

## 5. Source-policy conclusions

| Product | Metadata authority | Artifact transport | Execution owner |
| --- | --- | --- | --- |
| Claude Desktop | reviewed fixed mirror manifest derived from official Anthropic endpoint; official docs establish product identity | fixed AgentsMirror macOS endpoint | existing FyAgent managed desktop downloader/DMG/helper |
| OpenCode Desktop | fixed official GitHub latest release/version owner | fixed official OpenCode stable DMG endpoint | existing FyAgent managed desktop downloader/DMG/helper |
| Qoder/TRAE/WorkBuddy | existing reviewed vendor source adapters, but only when fresh install needs a release | existing fixed/allowlisted vendor endpoints | existing FyAgent managed desktop downloader/DMG/helper |

No generic mirror, arbitrary source URL or product-specific updater is justified.

## 6. Caveats

- Network behavior varies; one successful local probe is not a China-wide availability SLA.
- Mirror schedule/freshness is maintained externally and must be observed through manifest version/time, not assumed.
- Anthropic may change desktop packaging, Bundle ID, updater or service-region policy; implementation must fail closed and require a new review on identity/schema drift.
- OpenCode release asset names and stable endpoint behavior may change; fixed endpoint and mounted bundle checks remain required.
- The local machine did not contain Claude Desktop before the ephemeral DMG inspection, so installed-update HIL remains future work.

