<div align="center">
  <img src="assets/fyagent.png" width="128" alt="FyAgent icon">
  <h1>FyAgent</h1>
  <p>A desktop control center for the AI coding tools you already use.</p>
  <p><a href="README_ZH.md">简体中文</a> · <a href="README_JA.md">日本語</a></p>
</div>

FyAgent keeps provider settings, extensions, proxy routing, and usage records in one place. It works with Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw, and Hermes, so changing a model or endpoint does not mean hand-editing several config files.

> **Release status:** FyAgent is under active development. Back up important configuration before upgrading, and review the trust information for each release before installing it.

## What you can do

- Add providers from built-in presets or your own API-compatible endpoint, then switch between them without rewriting config files.
- Manage MCP servers, reusable prompts, and Skills across supported tools.
- Route requests through the local proxy, set failover rules, and test model availability.
- See token usage and estimated cost in one view.
- Reopen sessions and workspaces without hunting through tool-specific history folders.
- Back up and sync configuration while keeping secrets on the device you control.

FyAgent stores its working data locally in `~/.fyagent` and uses SQLite plus atomic file writes for safer updates. Deep-link imports use the `fyagent://` protocol and always show what will change before anything is written.

## Install

Download the build for your platform from [GitHub Releases](https://github.com/fy-agent/fyagent/releases). Release files follow these names:

- macOS: `FyAgent-X.Y.Z-macOS.dmg`, `FyAgent-X.Y.Z-macOS.zip`
- Windows: `FyAgent-X.Y.Z-Windows-x64-setup.exe`, `FyAgent-X.Y.Z-Windows-arm64-setup.exe`
- Linux x64: `FyAgent-X.Y.Z-Linux-x86_64.AppImage`, `FyAgent-X.Y.Z-Linux-x86_64.deb`, `FyAgent-X.Y.Z-Linux-x86_64.rpm`
- Linux arm64: `FyAgent-X.Y.Z-Linux-arm64.AppImage`, `FyAgent-X.Y.Z-Linux-arm64.deb`, `FyAgent-X.Y.Z-Linux-arm64.rpm`

Windows releases use an NSIS setup program; MSI and portable ZIP packages are not part of the current release surface. macOS builds are ad-hoc signed, not signed with an Apple Developer ID, and not notarized. Linux Flatpak files are for self-builds and are not official release artifacts.

Before installing, read the release notes and verify the published checksums, `signing-status.json`, and build attestation. `NotSigned` is a status, not proof that a file is safe. See the [installation guide](docs/user-manual/en/1-getting-started/1.2-installation.md) for platform-specific steps.

## Start here

1. Open **Providers** and add the service you use. A preset fills in the common fields; you only need to supply your credential and any custom endpoint.
2. Select the provider and choose **Apply**. FyAgent previews and writes the matching tool configuration.
3. Open the target coding tool and send a small test request.
4. Add MCP servers, Prompts, or Skills only after the basic provider path is working.

The full guide is available in [English](docs/user-manual/en/README.md), [简体中文](docs/user-manual/zh/README.md), and [日本語](docs/user-manual/ja/README.md). Release history lives in [docs/release-notes](docs/release-notes/README.md).

## Development

The repository uses `mise` as its supported entry point:

```bash
mise trust
mise run bootstrap
mise run dev
mise run build
```

Run `mise run check` before opening a pull request. The [development guide](docs/fyagent/development/README.md) explains the required toolchain and the smaller checks available while you work. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before making a larger change.

## Project history and license

FyAgent grew from CC Switch and keeps the upstream copyright and license notices for the code it inherited. The product name, current development, and FyAgent-owned additions are maintained by the FyAgent project.

FyAgent is source-available software, not open source as defined by the Open Source Initiative. FyAgent-owned components and modifications are licensed under the [PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt); commercial use requires separate written authorization. CC Switch-derived portions remain under the MIT License. See [LICENSE](LICENSE), [LICENSING.md](LICENSING.md), and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
