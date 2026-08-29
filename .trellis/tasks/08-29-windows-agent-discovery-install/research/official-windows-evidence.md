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
