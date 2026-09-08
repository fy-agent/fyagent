# Reuse and upstream evidence

## 0. Research method

- Access date: 2026-09-01.
- Primary sources were preferred: Microsoft Learn, vendor documentation/download pages, first-party GitHub repositories/workflows and first-party support/release notes.
- External projects were evaluated as possible capability owners, not automatically selected because they are popular.
- Production constants must still be derived from current artifact inspection and native HIL; web documentation alone is not an executable identity.

## 1. Windows platform capabilities

### Explorer ordinary-user launch and authenticated IPC

Primary sources:

- [Execute In Explorer sample](https://github.com/microsoft/Windows-classic-samples/tree/main/Samples/ExecuteInExplorer)
- [Named Pipe Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)

Repository fit:

- FyAgent already freezes the Explorer interactive user and uses a bounded one-shot named-pipe helper;
- this is the correct owner for formal elevated Windows builds to perform ordinary-user observation/execution;
- no token-manipulation library, local HTTP daemon or generic RPC is needed.

Decision: **extend the existing closed helper protocol**, never add arbitrary command/args/URL/path fields.

### Windows PackageManager / MSIX

Primary sources:

- [Windows.Management.Deployment.PackageManager](https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager)
- [Microsoft windows-rs](https://github.com/microsoft/windows-rs)

Observed official contract:

- `PackageManager` can enumerate packages for a specific user and filter by package family/name/publisher;
- it can add/register packages for the current user and return deployment progress/status;
- package identity and application IDs provide stronger discovery/launch authority than display names or paths.

Repository fit:

- FyAgent already uses `windows-rs` and PackageManager for Codex;
- the correct reuse decision is a narrow packaged-product capability extraction/delegation, not a second MSIX stack;
- exact package name/publisher/family/AUMID must come from the reviewed Claude/Codex artifact and installed package, not from a friendly name.

Decision: **reuse existing PackageManager owner** for proven packaged applications.

### App Paths

Primary source:

- [Application Registration — App Paths](https://learn.microsoft.com/en-us/windows/win32/shell/app-registration)

Observed official contract:

- App Paths is the preferred Windows registry mechanism for mapping an executable name to a fully qualified path;
- both per-user and machine registrations exist.

Repository fit:

- existing Agent inventory already scans App Paths;
- App Paths remains a candidate hint only: the target file must still pass product/PE/signature checks.

Decision: **reuse existing App Paths adapter**, never execute arbitrary registry commands.

### WinGet

Primary sources:

- [WinGet install command](https://learn.microsoft.com/en-us/windows/package-manager/winget/install)
- [WinGet troubleshooting and installer-scope caveats](https://learn.microsoft.com/en-us/windows/package-manager/winget/troubleshooting)
- [Microsoft winget-cli source](https://github.com/microsoft/winget-cli)

Positive evidence:

- mature Microsoft-owned, open-source package manager;
- supports exact ID/source/version/architecture selection;
- has installer manifests, hash checks and multiple installer types.

Material constraints:

- official docs require exact filtering to avoid ambiguity, while default queries are substring matching;
- official troubleshooting notes that EXE installer scope and exit semantics are not always deterministic;
- availability/registration and execution context still need native validation;
- using the CLI would add another command/output lifecycle around the same vendor installers;
- package presence does not remove the need for FyAgent post-install identity/readback.

Decision: **do not adopt WinGet as the baseline or mandatory dependency**. Permit a narrow product-level adapter only after exact package/source/scope/architecture/HIL proof and only if it is safer than direct existing owners.

### Store product pages

A Microsoft Store product page/deep link is useful as an official manual fallback, but opening the page is not evidence that a package was installed. FyAgent must not report an automatic install until PackageManager inventory confirms it.

Decision: **manual fallback unless an exact package deployment API/identity is proven**.

## 2. Product evidence

### Claude Desktop

Primary sources:

- [Deploy Claude Desktop for Windows](https://support.claude.com/en/articles/12622703-deploy-claude-desktop-for-windows)
- [Claude download](https://claude.com/download)

Current official facts:

- Anthropic provides Windows MSIX packages for managed deployment;
- Anthropic also offers a user-friendly installer;
- the official deployment guidance distinguishes per-user deployment, UAC/admin-dependent capabilities and centralized update management;
- x64 and ARM64 artifacts are documented by the official distribution surface.

Engineering conclusion:

- MSIX is the first reusable candidate because FyAgent already owns PackageManager deployment, but it is not preselected: the consumer installer may be required to preserve Cowork/service behavior;
- package name/publisher/family/AUMID cannot be guessed from documentation text and must be extracted from current artifacts/installed packages;
- update ownership must be frozen to avoid FyAgent, vendor updater, Store or MDM competing over the same installation;
- if MSIX cannot satisfy the lifecycle safely, the user installer may use the existing signed-EXE runner; no PowerShell installer is needed.

### OpenCode Desktop

Primary sources:

- [anomalyco/opencode](https://github.com/anomalyco/opencode)
- [OpenCode README desktop downloads](https://github.com/anomalyco/opencode/blob/dev/README.md)
- [OpenCode publish workflow](https://github.com/anomalyco/opencode/blob/dev/.github/workflows/publish.yml)
- [OpenCode releases](https://github.com/anomalyco/opencode/releases)

Current official facts:

- the project ships a desktop application and documents a Windows desktop EXE;
- the current publish workflow builds Windows desktop targets for x64 and ARM64;
- the workflow verifies Authenticode status for generated Windows Electron artifacts before publishing;
- the repository is the upstream source and release owner.

Engineering conclusion:

- reuse FyAgent’s fixed-GitHub metadata primitive and signed-EXE runner;
- do not depend on Scoop merely because the README mentions it;
- README asset tables and workflow matrix can temporarily differ, so current release assets must be inspected per architecture;
- upstream signature verification is useful provenance but FyAgent must independently validate the downloaded artifact and freeze the actual signer/product identity;
- do not invoke/copy the upstream Electron updater as FyAgent’s hidden lifecycle implementation.

### Codex / new ChatGPT desktop app

Primary sources:

- [Moving to the new ChatGPT desktop app](https://help.openai.com/en/articles/20001276-moving-to-the-new-chatgpt-desktop-app)
- [ChatGPT release notes](https://help.openai.com/en/articles/6825453-chatgpt-release-notes)

Current official facts:

- OpenAI now describes a new ChatGPT desktop app on macOS and Windows that includes Chat, Work and Codex;
- existing Codex app users are instructed to update normally and the app becomes the new ChatGPT desktop app;
- users of the previous ChatGPT desktop app may temporarily have a separate ChatGPT Classic installation.

Engineering conclusion:

- the current exact `OpenAI.Codex` assumptions may be stale or transitional;
- display-name matching is especially unsafe because `ChatGPT`, `ChatGPT Classic` and historical Codex may coexist;
- the task needs clean-install, Codex-upgrade and ChatGPT Classic coexistence HIL to record exact package identities/AUMIDs/process package families; a small exact migration set is implemented only if existing identity policy is proven stale;
- until that evidence exists, do not loosen existing identity policy.

### Grok Build

Primary sources:

- [Grok Build overview](https://docs.x.ai/build/overview)
- [Grok Build CLI reference](https://docs.x.ai/build/cli/reference)
- [xAI enterprise deployment](https://docs.x.ai/build/enterprise)

Current official facts:

- Grok Build is documented as a CLI/TUI/headless/ACP coding agent;
- official install/update flows are CLI-oriented.

Important product distinction:

- xAI also documents a separate **Grok Bot** desktop product. It is not evidence that Grok Build has a desktop installer and must not be substituted for the `grokbuild` product ID.

Engineering conclusion:

- retain Grok Build as the sole CLI installation surface;
- reuse the existing hardened `services/tooling/grok` owner;
- do not migrate the product to Grok Bot or create a desktop alias without a separate product decision.

### QoderWork

Primary sources:

- [QoderWork Windows Installation Guide](https://docs.qoder.com/qoderwork/install-windows)
- [QoderWork Quick Start](https://docs.qoder.com/qoderwork/quick-start)

Current official facts:

- Windows uses `.exe` installers;
- user and system installers have the same functions but different scope/permission requirements.

Engineering conclusion:

- existing signed-EXE runner is the right owner;
- current user vs system selection and UAC behavior must be explicit;
- do not guess that a user installer can replace a requested system install or vice versa;
- current task keeps the product install-only under FyAgent.

### TRAE Work

Primary sources:

- [TRAE documentation](https://docs.trae.ai/)

Current official facts:

- TRAE Work is offered as a standalone desktop client;
- current Windows release/source details must be read from the product’s official release endpoint and artifact during implementation.

Engineering conclusion:

- preserve the existing fixed vendor source and signed-EXE runner;
- use native HIL to correct current product/signer/path drift only;
- do not replace it with a broad package manager search.

### WorkBuddy

Primary sources:

- [WorkBuddy official site](https://www.workbuddy.ai/)
- [WorkBuddy Microsoft Store listing](https://apps.microsoft.com/detail/xpfg2p1xdmx1x0)
- [Tencent Cloud WorkBuddy documentation](https://intl.cloud.tencent.com/document/product/1300/81046)

Current official facts:

- WorkBuddy is a Tencent desktop AI agent with Windows support;
- an official Microsoft Store product page exists with product ID `XPFG2P1XDMX1X0`.

Engineering conclusion:

- Store listing identity is useful source evidence but is not sufficient to infer package family/publisher/AUMID or programmatic installation semantics;
- inspect the installed Store package and compare it with the existing signed EXE distribution;
- adopt PackageManager only if exact identity and lifecycle behavior are safer than the current EXE; otherwise keep EXE and expose Store as manual fallback;
- preserve install-only policy.

### Duplicate non-Grok Tooling lifecycle

Repository evidence shows that the legacy Settings/Tooling surface still constructs public npm, Shell and PowerShell install/update flows for non-Grok tools. This directly conflicts with the requested unified product policy.

Decision: **retire non-Grok lifecycle actions and copied install commands on both OSes**. Keep only read-only discovery/configuration code that has an identified consumer. Backend stale actions must fail before side effects.

## 3. Reuse decision matrix

| Capability | Existing FyAgent owner | External candidate reviewed | Decision |
| --- | --- | --- | --- |
| Product/surface/action policy | Agent lifecycle policy | none needed | reuse exactly one owner |
| Windows packaged app inventory/deployment | Codex PackageManager/windows-rs | WinGet, Store links | reuse/extract narrowly; no generic package manager |
| Windows unpackaged discovery | App Paths + Uninstall + known paths + PE identity | WinGet/Scoop | reuse current evidence chain |
| Windows EXE execution | signed artifact + Explorer user helper | PowerShell/cmd/vendor scripts | reuse current runner; scripts rejected |
| Download/retry/cancel/progress | current artifact/job owners | package-manager downloaders | reuse current owner |
| Desktop update | current product source + shared job | vendor auto-updater | one explicit owner per product; vendor updater not invoked invisibly |
| OpenCode release metadata | existing fixed GitHub helper | new GitHub client | reuse/extract narrow fixed-repo primitive |
| Grok CLI | hardened Grok Tooling owner | generic Tooling command builder | keep Grok as the sole CLI lifecycle and route formal Windows through the closed ordinary-user helper |
| Codex log dedup | session usage pending/cache | external logging crate/rate limiter | fix semantic state locally; no new logging dependency |

## 4. Dependency conclusion

Baseline implementation requires no new runtime dependency. Existing Microsoft Windows crates and repository owners cover the necessary OS primitives. WinGet and upstream project source are valuable reference/evidence, but adopting them as a new execution framework would duplicate lifecycle and weaken exact identity/post-readback control.
