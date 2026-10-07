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
   - In PowerShell, calculate the SHA-256 hash of the installer (replace filename for ARM64):
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
   - When Smart App Control is enabled on Windows 11, it automatically blocks unsigned applications. The system does not provide a "Run anyway" button and cannot whitelist individual files.
   - On personal devices, the feature can only be disabled via **Windows Security → App & browser control → Smart App Control settings**.
   - **Caution**: Disabling Smart App Control lowers overall system protection; weigh this decision carefully before turning it off. On some Windows versions, once disabled, it cannot be turned back on without resetting or reinstalling Windows. If you prefer not to lower system security, wait for a future signed release. On corporate or managed computers, follow organizational IT policy and let your IT administrator handle it.
5. **Antivirus or Defender false positives**:
   - If antivirus software or Microsoft Defender blocks or quarantines the installer, first verify the SHA-256 checksum against the release notes as described in step 1.
   - Do not disable your antivirus software or real-time protection entirely.
   - Once the hash is confirmed to match, add a minimal single-file exclusion or restore the installer from quarantine.
   - You can submit a false positive sample to your security vendor to help improve detection (Microsoft Defender submission portal: [Microsoft Security Intelligence file submission](https://www.microsoft.com/wdsi/filesubmission)).
   - You can also report the issue on the [Q&A discussions](https://github.com/fy-agent/fyagent/discussions/categories/q-a).

[Back to manual](README.md)
