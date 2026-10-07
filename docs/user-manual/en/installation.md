# Installation and updates

Download the package for your platform from [fy-agent/fyagent Releases](https://github.com/fy-agent/fyagent/releases). Check that release's signing notes, SHA-256, source SHA and attestation.

| Platform | Requirements                             | Package                                                                          |
| -------- | ---------------------------------------- | -------------------------------------------------------------------------------- |
| Windows  | Windows 10 or later, x64 / ARM64         | `FyAgent-X.Y.Z-Windows-x64-setup.exe` or `FyAgent-X.Y.Z-Windows-arm64-setup.exe` |
| macOS    | macOS 12 or later, Intel / Apple Silicon | `FyAgent-X.Y.Z-macOS.dmg`                                                        |

On Windows, run the machine-wide installer and finish the wizard. If a trust prompt appears, verify the download source and signing status of that build, and follow your organization's security policy.

On macOS, open the DMG, drag `FyAgent.app` into Applications, and launch it there. If macOS blocks it, verify the download and release evidence, then follow System Settings → Privacy & Security → Open Anyway. Keep Gatekeeper and download quarantine protection enabled.

On first launch, choose a purpose for recommendations or select 「跳过引导」 (Skip guide). Follow [AI software configuration](agents.md) to scan and install software. MCP installation identifies required runtimes such as Node.js / npx or uv / uvx; prepare those before proceeding.

To update FyAgent, download a matching package from the same Releases page and follow its instructions. To uninstall, use Windows Settings → Apps or move the macOS application to Trash. Personal data defaults to `~/.fyagent/`; back up anything you want to keep before changing those files.

## Installing AI software from FyAgent

In AI software configuration, use 「安装前检查」 (Check before installing) or 「更新前检查」 (Check before updating). If several locations are available, select the target, then review the software, version, OS and architecture, source information, and intended writes. The target picker contains a location choice; the following summary performs a different job: preflight and explicit start confirmation.

Compare the download source with the software's official links. A displayed domain or URL does not mean the package has passed validation; execution still checks the selected source, package identity, and target. CLI actions use the admitted package or existing CLI update method, without accepting arbitrary package names, URLs, or paths from the summary.

Preparation does not install software. Start explicitly with 「开始安装」 (Start installation), 「开始更新」 (Start update), or 「开始命令行安装／更新」 (Start CLI installation/update). A Windows vendor wizard uses 「下载并打开安装窗口」 (Download and open installer); finish the official wizard, then refresh installation status and version. Opening the wizard is not evidence of a completed installation.

「暂不开始」 (Not now) or closing the summary before starting creates no task. After starting, closing a summary/window or changing pages does not cancel background work. Request cancellation only when the progress controls offer it. FyAgent does not terminate a vendor wizard after opening it.

If the result is unknown, unconfirmed, or still running, refresh installation status or rescan and retain the current task message. Select the target again if its location changed. An unresolved result requiring recovery cannot be replaced by another installation attempt. Check observed installation status, version, and task result separately; closing a summary, opening a wizard, or clicking again proves neither completion nor recovery.

[Back to manual](README.md)
