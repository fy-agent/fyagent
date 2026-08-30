# Official Windows evidence

## Installed application evidence

- Uninstall registry properties:
  https://learn.microsoft.com/en-us/windows/win32/msi/uninstall-registry-key
- App Paths/application registration:
  https://learn.microsoft.com/en-us/windows/win32/shell/app-registration
- PackageManager APIs:
  https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager
- Alternate 32/64 registry views:
  https://learn.microsoft.com/en-us/windows/win32/winprog64/accessing-an-alternate-registry-view
- File version resource lookup:
  https://learn.microsoft.com/en-us/windows/win32/api/winver/nf-winver-verqueryvaluea

These sources support a multi-adapter inventory. None of them alone covers every desktop format.

## Installer trust and execution

- WinVerifyTrust PE verification example:
  https://learn.microsoft.com/en-us/windows/win32/seccrypto/example-c-program--verifying-the-signature-of-a-pe-file
- ShellExecuteEx process handle:
  https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-shellexecuteinfow
- UAC architecture:
  https://learn.microsoft.com/en-us/windows/security/application-security/application-control/user-account-control/architecture

The reviewed Windows APIs allow signature verification, UAC-owned elevation and waiting on a launched process. FyAgent still needs a closed helper/capability boundary because those APIs also accept arbitrary paths/verbs/arguments if exposed carelessly.

## Product installation behavior

- QoderWork CN Windows guide:
  https://help.aliyun.com/en/lingma/windows-installation
  - Official documentation distinguishes System and User installers. The current FyAgent resolver points only to the User x64 installer.
- WorkBuddy Windows guide:
  https://www.workbuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Installation-Win-Guide
  - The vendor installer asks the user to select an install path.
- TRAE official download page:
  https://www.trae.cn/download
  - Confirms a Windows x64 TraeWork download, but does not by itself define silent switches or installation scope.

Decision: do not infer system scope, silent flags or arm64 support from an EXE filename. Freeze each capability from official source plus real installer/HIL evidence.

## 2026-08-30 artifact review

The current first-party resolvers were queried and the PE version-resource and
embedded Authenticode signer certificate were inspected with bounded HTTP range
reads. The concrete versions and content-addressed URLs below are research
fixtures only. Production continues to resolve the official latest aliases/API
and must not pin these values.

| Product | Resolved Windows artifact evidence | PE `ProductName` | PE product/file version | Embedded signer leaf subject |
| --- | --- | --- | --- | --- |
| QoderWork CN | official `QoderWorkCN-Setup-User-x64.exe` latest alias | `QoderWork CN` | `0.9.15` | `Alibaba Cloud Computing Co., Ltd.` |
| TRAE Work CN | official CN `data.solo` x64 result, research fixture `2.3.78099` | `TraeWork CN` | `0.1.58` | `北京引力弹弓科技有限公司` |
| WorkBuddy | official `workbuddy-win32-x64-user` update result, research fixture `5.3.14.36279234` | `WorkBuddy` | `5.3.14` | `Tencent Technology (Shenzhen) Company Limited` |

Consequences:

- The TRAE closed product-name policy must include `TraeWork CN`; relying only
  on the installed-folder-era `TRAE SOLO CN` spelling rejects the official
  installer before launch.
- Publisher admission must bind the actual Authenticode signer certificate,
  not “any certificate present in the PKCS#7 store”. Production therefore uses
  `WinVerifyTrust`, requires one top-level signer, resolves that signer through
  `CryptMsgGetAndVerifySigner`, and compares only its bounded display subject.
- Qoder is the only reviewed fixed current-user destination: the resolved
  artifact is the User installer. WorkBuddy explicitly lets the vendor UI
  choose a location. TRAE documents Windows x64 availability but not a stable
  silent/scope contract, so it remains vendor-choice/unknown and may invoke
  UAC.
- No reviewed Qoder/TRAE/WorkBuddy MSIX package identity exists. PackageManager
  discovery is therefore not applicable to these three current EXE products;
  no speculative PFN/AUMID mapping is shipped. Codex keeps its existing,
  separately owned PackageManager implementation.
- No silent installer switches were admitted. All three products use the
  closed `agent-exe-install + product enum` helper action and vendor UI.

The range inspection proves the reviewed artifact metadata only. It does not
replace Windows-host `WinVerifyTrust`, Shell-user/UAC, process-handle, Registry
view, or post-install HIL evidence.
