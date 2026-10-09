# Installation and updates

Download the package for your platform from [fy-agent/fyagent Releases](https://github.com/fy-agent/fyagent/releases). Check that release's signing notes, SHA-256, source SHA and attestation.

| Platform | Requirements                             | Package                                                                          |
| -------- | ---------------------------------------- | -------------------------------------------------------------------------------- |
| Windows  | Windows 10 or later, x64 / ARM64         | `FyAgent-X.Y.Z-Windows-x64-setup.exe` or `FyAgent-X.Y.Z-Windows-arm64-setup.exe` |
| macOS    | macOS 12 or later, Intel / Apple Silicon | `FyAgent-X.Y.Z-macOS.dmg`                                                        |

On Windows, run the machine-wide installer and finish the wizard. If a trust prompt appears, verify the download source and signing status of that build, and follow your organization's security policy. If the installer is blocked, see [below](#if-windows-blocks-the-installer).

On macOS, open the DMG, drag `FyAgent.app` into Applications, and launch it there. If macOS blocks it, verify the download and release evidence, then follow System Settings → Privacy & Security → Open Anyway. Keep Gatekeeper and download quarantine protection enabled.

On first launch, choose a purpose for recommendations or select 「跳过引导」 (Skip guide). Follow [AI software configuration](agents.md) to scan and install software. MCP installation identifies required runtimes such as Node.js / npx or uv / uvx; prepare those before proceeding.

To update FyAgent, download a matching package from the same Releases page and follow its instructions. To uninstall, use Windows Settings → Apps or move the macOS application to Trash. Personal data defaults to `~/.fyagent/`; back up anything you want to keep before changing those files.

## If Windows blocks the installer

Windows installer packages (`FyAgent-X.Y.Z-Windows-x64-setup.exe` and `FyAgent-X.Y.Z-Windows-arm64-setup.exe`) use NSIS for machine-wide installation and require UAC administrator elevation during setup. Windows packages currently do not include an Authenticode signature and are marked as `NotSigned` in the "Windows installer signing status" table on GitHub Releases.

If the system or security software blocks the installer, follow the "verify first, allow later" principle rather than disabling security protections:

1. **Verify file hashes and build attestation first**: Always confirm file integrity and source authenticity before allowing or running the installer:
   - Download installer packages only from official [GitHub Releases](https://github.com/fy-agent/fyagent/releases).
   - Open PowerShell, run the following command in the directory containing the installer to calculate its SHA-256 hash (replace X.Y.Z with the actual version number; replace filename for ARM64):
     ```powershell
     Get-FileHash .\FyAgent-X.Y.Z-Windows-x64-setup.exe -Algorithm SHA256
     ```
     Compare the resulting hash against the SHA-256 published in the release table. If the hash does not match, the file is corrupted or tampered with; delete it immediately and do not run it.
   - If GitHub CLI is installed, verify the build attestation:
     ```powershell
     gh attestation verify .\FyAgent-X.Y.Z-Windows-x64-setup.exe --repo fy-agent/fyagent
     ```
2. **Microsoft Defender SmartScreen displays "Windows protected your PC"**:
   - When SmartScreen shows this prompt, click **More info**.
   - Confirm that the application and file name match the downloaded installer.
   - Click **Run anyway**, then accept the User Account Control (UAC) administrator prompt to continue installation.
3. **File marked as downloaded from the Internet (Unblock)**:
   - Files downloaded from a browser may receive a web zone identifier. If double-clicking the installer does nothing or is blocked, right-click the file and select **Properties**.
   - On the **General** tab, look at the **Security** section at the bottom.
   - Check the **Unblock** box and click **OK** before running the installer again. (Note: this is only needed if the "Unblock" checkbox is present.)
4. **Smart App Control (Windows 11) blocks the application directly**:
   - When Smart App Control is enabled on Windows 11, it first uses a cloud-powered security service to determine whether an application is safe; applications deemed safe are allowed to run, but if safety cannot be confirmed and the application lacks a valid signature, it is blocked. When blocked by Smart App Control, the system does not provide a "Run anyway" button and cannot whitelist individual files.
   - On personal devices, the feature can only be disabled via **Windows Security → App & browser control → Smart App Control settings**.
   - **Caution**: Disabling Smart App Control lowers overall system protection; weigh this decision carefully before turning it off. Recent Windows updates allow Smart App Control to be re-enabled without resetting or reinstalling Windows; older versions or devices that have not yet received this update may still require a reset or clean installation, so refer to your local device settings and Microsoft documentation. If you prefer not to lower system security, wait for a future signed release. On corporate or managed computers, follow organizational IT policy and let your IT administrator handle it.
5. **Antivirus or Defender false positives**:
   - If antivirus software or Microsoft Defender blocks or quarantines the installer, first verify the SHA-256 checksum against the release notes as described in step 1.
   - Do not disable your antivirus software or real-time protection entirely.
   - Once the hash is confirmed to match, add a minimal single-file exclusion or restore the installer from quarantine.
   - You can submit a false positive sample to your security vendor to help improve detection (Microsoft Defender submission portal: [Microsoft Security Intelligence file submission](https://www.microsoft.com/wdsi/filesubmission)).
   - You can also report the issue on the [Q&A discussions](https://github.com/fy-agent/fyagent/discussions/categories/q-a).

## Installing AI software from FyAgent

In AI software configuration, use 「安装前检查」 (Check before installing) or 「更新前检查」 (Check before updating). If several locations are available, select the target, then review the software, version, OS and architecture, source information, and intended writes. The target picker contains a location choice; the following summary performs a different job: preflight and explicit start confirmation.

Compare the download source with the software's official links. A displayed domain or URL does not mean the package has passed validation; execution still checks the selected source, package identity, and target. CLI actions use the admitted package or existing CLI update method, without accepting arbitrary package names, URLs, or paths from the summary.

Preparation does not install software. Start explicitly with 「开始安装」 (Start installation), 「开始更新」 (Start update), or 「开始命令行安装／更新」 (Start CLI installation/update). A Windows vendor wizard uses 「下载并打开安装窗口」 (Download and open installer); finish the official wizard, then refresh installation status and version. Opening the wizard is not evidence of a completed installation.

「暂不开始」 (Not now) or closing the summary before starting creates no task. After starting, closing a summary/window or changing pages does not cancel background work. Request cancellation only when the progress controls offer it. FyAgent does not terminate a vendor wizard after opening it.

If the result is unknown, unconfirmed, or still running, refresh installation status or rescan and retain the current task message. Select the target again if its location changed. An unresolved result requiring recovery cannot be replaced by another installation attempt. Check observed installation status, version, and task result separately; closing a summary, opening a wizard, or clicking again proves neither completion nor recovery.

[Back to manual](README.md)
