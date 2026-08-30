<div align="center">
  <img src="assets/brand/github/for-you-gate.svg" width="104" alt="FyAgent For You Gate">
  <h1>FyAgent</h1>
  <p>Manage models, Skills, MCP servers, prompts, and memory files for AI software from one local desktop app.</p>
  <p><a href="README.md">简体中文</a> · <a href="README_JA.md">日本語</a></p>
  <p>
    <a href="https://github.com/fy-agent/fyagent/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/fy-agent/fyagent?style=flat-square&label=release&color=0B66FF"></a>
    <a href="https://github.com/fy-agent/fyagent/actions/workflows/ci.yml"><img alt="CI status" src="https://img.shields.io/github/actions/workflow/status/fy-agent/fyagent/ci.yml?branch=main&style=flat-square&label=CI"></a>
    <img alt="Windows and macOS" src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS-18D3C5?style=flat-square">
    <a href="LICENSING.md"><img alt="Source-available license" src="https://img.shields.io/badge/license-source--available-555B66?style=flat-square"></a>
  </p>
  <p>
    <a href="https://github.com/fy-agent/fyagent/releases/latest"><strong>Download</strong></a> ·
    <a href="docs/user-manual/en/README.md">Manual</a> ·
    <a href="https://github.com/fy-agent/fyagent/discussions">Discussions</a> ·
    <a href="CONTRIBUTING.md">Contribute</a>
  </p>
</div>

## What FyAgent is

FyAgent is a local desktop configuration tool. It brings commonly used settings from multiple AI applications into one interface and shows the affected files and settings before a write.

The current production interface has six areas: AI software configuration, Models, Skills, MCP, Prompts, and Memory. Each application exposes a different set of controls; FyAgent shows only the actions supported by its current integration.

> **Current status:** FyAgent is under active development. Back up important configuration before an upgrade and read the release notes for the version you install.

## Current features

| Area                      | Available tasks                                                                                                                                                                                |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AI software configuration | Scan QoderWork CN, TRAE Work CN, WorkBuddy, Grok Build, Codex, Claude Code, and OpenCode; where supported, open installation, update, launch, authentication, and resource-assignment controls |
| Models                    | View or change model and Provider settings for the applications above; preview writes and check the result after saving                                                                        |
| Skills                    | Install Skills from local files or discovery results, then assign them to supported applications                                                                                               |
| MCP                       | Add, import, and manage MCP servers, then assign them to supported applications                                                                                                                |
| Prompts                   | Manage prompts for Grok Build, Codex, Claude Code, OpenCode, Gemini, OpenClaw, and Hermes                                                                                                      |
| Memory                    | Edit long-term memory files for OpenClaw and Hermes, plus OpenClaw daily memory files                                                                                                          |

Working data is stored in `~/.fyagent` by default. See the [manual](docs/user-manual/en/README.md) for exact paths, backup instructions, and application-specific limits.

## Interface

The screenshots below use Simplified Chinese. The left navigation opens AI software configuration, Models, Skills, MCP, Prompts, and Memory.

<table>
  <tr>
    <td align="center" width="50%">
      <img src="assets/screenshots/main-zh-2.png" alt="FyAgent Skills management page">
      <br><em>Skills</em>
    </td>
    <td align="center" width="50%">
      <img src="assets/screenshots/main-zh-3.png" alt="FyAgent MCP management page">
      <br><em>MCP</em>
    </td>
  </tr>
  <tr>
    <td align="center" colspan="2">
      <img src="assets/screenshots/main-zh-1.png" alt="FyAgent WorkBuddy model configuration page">
      <br><em>Models</em>
    </td>
  </tr>
</table>

## Quick start

1. Download the package for your system from [GitHub Releases](https://github.com/fy-agent/fyagent/releases/latest).
2. Open **AI software configuration** and scan the device for installed applications.
3. Select an application and open its configuration page. Use the controls available there for models, Skills, MCP, prompts, or authentication.
4. Review the affected settings before saving. After the write, follow the page guidance to inspect the result or test the connection.

See the full [English manual](docs/user-manual/en/README.md), or switch to [简体中文](docs/user-manual/zh/README.md) or [日本語](docs/user-manual/ja/README.md).

## Downloads and release verification

Release files use these names:

- macOS: `FyAgent-X.Y.Z-macOS.dmg`
- Windows: `FyAgent-X.Y.Z-Windows-x64-setup.exe`, `FyAgent-X.Y.Z-Windows-arm64-setup.exe`

Windows releases use an NSIS setup program; MSI and portable ZIP packages are not part of the current release. macOS builds are signed with an Apple Developer ID and notarized.

Before installing, read the release notes and check the published checksums, `signing-status.json`, and build attestation. `NotSigned` describes the signing state; it does not prove that a file is safe. See the [installation guide](docs/user-manual/en/1-getting-started/1.2-installation.md) for platform-specific steps and the [release notes index](docs/release-notes/README.md) for version history.

## FAQ

<details>
<summary><strong>Where does FyAgent store its data?</strong></summary>

FyAgent uses `~/.fyagent` on the local device by default. See [Configuration files](docs/user-manual/en/6-faq/6.1-config-files.md) for exact locations and backup guidance.

</details>

<details>
<summary><strong>Where should I ask for installation or configuration help?</strong></summary>

Check the [FAQ manual](docs/user-manual/en/6-faq/6.2-questions.md), then open a [Q&A discussion](https://github.com/fy-agent/fyagent/discussions/categories/q-a) with the FyAgent version, operating system, related application, and steps already tried. Use the [Bug Report](https://github.com/fy-agent/fyagent/issues/new?template=bug_report.yml) form for a reproducible software defect.

</details>

<details>
<summary><strong>Is FyAgent open source?</strong></summary>

FyAgent is source-available, not open source as defined by the OSI. FyAgent-owned components and modifications use PolyForm Noncommercial License 1.0.0; CC Switch-derived portions remain under the MIT License. See [Licensing](LICENSING.md).

</details>

## Community and contributions

- Usage questions and troubleshooting: [Q&A](https://github.com/fy-agent/fyagent/discussions/categories/q-a)
- Feature ideas: [Ideas](https://github.com/fy-agent/fyagent/discussions/categories/ideas)
- Configuration and usage notes: [Show and tell](https://github.com/fy-agent/fyagent/discussions/categories/show-and-tell)
- Reproducible defects and scoped work: [Issues](https://github.com/fy-agent/fyagent/issues)

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md), [SUPPORT.md](SUPPORT.md), and [CONTRIBUTING.md](CONTRIBUTING.md) for community expectations and contribution paths.

## Local development

A first checkout requires a global `mise >= 2026.8.6`:

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

Build for the current system with:

```bash
mise run build
```

Run `mise run check` before submitting a change. The [development guide](docs/fyagent/development/README.md) covers the full toolchain, focused checks, and release requirements.

## Project origin and license

FyAgent began as VibeKey, a concept for a physical keyboard and companion driver. The project later moved to cross-platform desktop software focused on AI application configuration and local data management, and was renamed **FyAgent (For You Agent)**.

The current desktop app evolved from CC Switch and retains upstream copyright and license notices for inherited code. FyAgent-owned components and modifications use the [PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt); commercial use requires separate written authorization. See [LICENSE](LICENSE), [LICENSING.md](LICENSING.md), and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
