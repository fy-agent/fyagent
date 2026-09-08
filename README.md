<div align="center">
  <img src="assets/brand/github/for-you-gate.svg" width="104" alt="FyAgent For You Gate">
  <h1>FyAgent</h1>
  <p><strong>For You Agent</strong>——AI 时代的个人随身数字人格。</p>
  <p>把你的模型、AI 账号、技能、提示词和工作方式，带到每一个 AI 工具里。</p>
  <p><a href="README_EN.md">English</a> · <a href="README_JA.md">日本語</a></p>
  <p>
    <a href="https://github.com/fy-agent/fyagent/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/fy-agent/fyagent?style=flat-square&label=release&color=0B66FF"></a>
    <a href="https://github.com/fy-agent/fyagent/actions/workflows/ci.yml"><img alt="CI status" src="https://img.shields.io/github/actions/workflow/status/fy-agent/fyagent/ci.yml?branch=main&style=flat-square&label=CI"></a>
    <img alt="Windows and macOS" src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS-18D3C5?style=flat-square">
    <a href="LICENSING.md"><img alt="Source-available license" src="https://img.shields.io/badge/license-source--available-555B66?style=flat-square"></a>
  </p>
  <p>
    <a href="https://github.com/fy-agent/fyagent/releases/latest"><strong>下载最新版</strong></a> ·
    <a href="docs/user-manual/zh/README.md">使用手册</a> ·
    <a href="https://github.com/fy-agent/fyagent/discussions">讨论区</a> ·
    <a href="CONTRIBUTING.md">参与贡献</a>
  </p>
</div>

## AI 时代的个人智能控制中心

FyAgent 面向正在使用 AI Agent、AI Worker 和智能助手的人。它把模型从哪里来、能连接哪些工具、会哪些技能、遵循什么指令、保存怎样的配置，集中到一个本地桌面应用里。

你不需要先弄懂 Provider、MCP 或 Prompt 这些术语。对用户来说，它们分别是 AI 的大脑来源、工具连接和行为指令。FyAgent 要做的，是把原本散落、隐蔽、容易改错的选择变得可见、可改，也能跟着你走。

> **发布状态：** FyAgent 仍在持续开发。升级前请备份重要配置，并在安装前阅读当次发布的可信度说明。

## 当前功能

| 区域        | 可以做什么                                                                                                                               |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| AI 软件配置 | 扫描 QoderWork CN、TRAE Work CN、WorkBuddy、Grok Build、Codex、Claude Code 和 OpenCode；在支持时提供安装、更新、启动、认证和资源分配入口 |
| 模型管理    | 为上述软件查看或修改模型与 Provider 设置；写入前预览变更，并在保存后检查结果                                                             |
| Skills 管理 | 从本地或发现页安装 Skills，并分配给支持的软件                                                                                            |
| MCP 管理    | 添加、导入和管理 MCP 服务，并分配给支持的软件                                                                                            |
| 提示词管理  | 管理 Grok Build、Codex、Claude Code、OpenCode、Gemini、OpenClaw 和 Hermes 的提示词                                                       |
| 记忆模块    | 编辑 OpenClaw 与 Hermes 的长期记忆文件，以及 OpenClaw 的每日记忆                                                                         |

工作数据默认保存在本机 `~/.fyagent`。具体写入位置、备份方法和各软件的支持差异见[使用手册](docs/user-manual/zh/README.md)。

## 界面预览

以下截图使用简体中文。左侧导航用于切换 AI 软件配置、模型、Skills、MCP、提示词和记忆。

<table>
  <tr>
    <td align="center" width="50%">
      <img src="assets/screenshots/skills.png" alt="FyAgent Skills 管理页">
      <br><em>Skills</em>
    </td>
    <td align="center" width="50%">
      <img src="assets/screenshots/models.png" alt="FyAgent 模型管理页">
      <br><em>模型管理</em>
    </td>
  </tr>
  <tr>
    <td align="center" colspan="2">
      <img src="assets/screenshots/home.png" alt="FyAgent AI 软件配置页">
      <br><em>AI 软件配置</em>
    </td>
  </tr>
</table>

## 快速开始

1. 从 [GitHub Releases](https://github.com/fy-agent/fyagent/releases/latest) 下载适合当前系统的安装包。
2. 打开“AI 软件配置”，扫描本机已安装的软件。
3. 选择一个软件并进入配置页。根据页面提供的入口设置模型、Skills、MCP、提示词或认证。
4. 保存前检查预览中的修改范围；保存后按页面提示确认配置或测试连接。

完整步骤见[简体中文手册](docs/user-manual/zh/README.md)，也提供 [English](docs/user-manual/en/README.md) 和 [日本語](docs/user-manual/ja/README.md)。

## 下载与发布验证

发布文件名如下：

- macOS：`FyAgent-X.Y.Z-macOS.dmg`
- Windows：`FyAgent-X.Y.Z-Windows-x64-setup.exe`、`FyAgent-X.Y.Z-Windows-arm64-setup.exe`

Windows 当前提供 NSIS 安装程序，不提供 MSI 或便携 ZIP。macOS 构建使用 Apple Developer ID 签名，并经过 Apple 公证。

安装前请阅读发布说明，并核对校验和、`signing-status.json` 和构建证明。`NotSigned` 只表示签名状态，不能单独证明文件安全。各系统的步骤见[安装说明](docs/user-manual/zh/1-getting-started/1.2-installation.md)，版本记录见[发布说明索引](docs/release-notes/README.md)。

## 常见问题

<details>
<summary><strong>FyAgent 会把数据保存在哪里？</strong></summary>

默认保存在本机 `~/.fyagent`。具体配置位置和备份方法见[配置文件说明](docs/user-manual/zh/6-faq/6.1-config-files.md)。

</details>

<details>
<summary><strong>遇到安装或配置问题，应该去哪里提问？</strong></summary>

先查看[常见问题手册](docs/user-manual/zh/6-faq/6.2-questions.md)，再到 [Q&A 讨论区](https://github.com/fy-agent/fyagent/discussions/categories/q-a)说明 FyAgent 版本、操作系统、相关软件和已经尝试过的步骤。可稳定复现的软件缺陷请提交 [Bug Report](https://github.com/fy-agent/fyagent/issues/new?template=bug_report.yml)。

</details>

<details>
<summary><strong>FyAgent 是开源软件吗？</strong></summary>

FyAgent 是源码可用软件，不是 OSI 定义的开源软件。FyAgent 自有组件和修改采用 PolyForm Noncommercial License 1.0.0；继承自 CC Switch 的部分继续使用 MIT 许可证。详见[授权说明](LICENSING.md)。

</details>

## 社区与贡献

- 使用问题与排查：[Q&A](https://github.com/fy-agent/fyagent/discussions/categories/q-a)
- 功能想法：[Ideas](https://github.com/fy-agent/fyagent/discussions/categories/ideas)
- 配置与使用经验：[Show and tell](https://github.com/fy-agent/fyagent/discussions/categories/show-and-tell)
- 可复现缺陷与明确任务：[Issues](https://github.com/fy-agent/fyagent/issues)

社区行为准则、支持范围和贡献流程见 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)、[SUPPORT.md](SUPPORT.md) 与 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 本地开发

首次 checkout 需要全局安装 `mise >= 2026.8.6`：

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

构建当前系统版本：

```bash
mise run build
```

提交前运行 `mise run check`。完整工具链、分层检查和发布要求见[开发文档](docs/fyagent/development/README.md)。

## Star History

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/fy-agent/fyagent/star-history/assets/star-history-dark.svg">
  <img src="https://raw.githubusercontent.com/fy-agent/fyagent/star-history/assets/star-history.svg" alt="FyAgent Star History">
</picture>

## 项目来源与授权

FyAgent 的前身 VibeKey 是一个实体键盘与配套驱动的产品构想。项目后来转为跨平台桌面软件，重点改为 AI 软件配置与本地数据管理，并更名为 **FyAgent（For You Agent）**。

当前桌面应用基于 CC Switch 演进，并保留继承代码的原作者版权和许可证声明。FyAgent 自有组件和修改采用 [PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt)，商业使用须另行取得书面授权。详见 [LICENSE](LICENSE)、[LICENSING.md](LICENSING.md) 和 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
