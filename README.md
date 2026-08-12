<div align="center">
  <img src="assets/brand/github/for-you-gate.svg" width="104" alt="FyAgent For You Gate">
  <h1>FyAgent</h1>
  <p><strong>For You Agent</strong>——AI 时代的个人随身数字人格。</p>
  <p>把你的模型、AI 账号、技能、提示词和工作方式，带到每一个 AI 工具里。</p>
  <p><a href="README_EN.md">English</a> · <a href="README_JA.md">日本語</a></p>
  <p>
    <a href="https://github.com/fy-agent/fyagent/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/fy-agent/fyagent?style=flat-square&label=release&color=0B66FF"></a>
    <a href="https://github.com/fy-agent/fyagent/actions/workflows/ci.yml"><img alt="CI status" src="https://img.shields.io/github/actions/workflow/status/fy-agent/fyagent/ci.yml?branch=main&style=flat-square&label=CI"></a>
    <img alt="Windows, macOS and Linux" src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-18D3C5?style=flat-square">
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

今天，FyAgent 先从最具体、也最容易失控的配置层做起，支持 Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw 和 Hermes。

> **发布状态：** FyAgent 仍在持续开发。升级前请备份重要配置，并在安装前阅读当次发布的可信度说明。

## 愿景：成为 AI 时代的随身数字人格

这里的“数字人格”不是一个模仿你说话的虚拟头像，而是你怎样选择、塑造和管理 AI 的长期载体：它用什么模型、能调用什么工具、会哪些技能、按照什么方式工作，又应该记住什么。

- **愿景：** 成为 AI 时代每个人的随身数字人格。
- **使命：** 让强大的 AI 变得可控、可信、可陪伴。
- **产品价值：** 成为 AI 时代的方向盘，让人始终知道自己的 AI 从哪里获得能力、怎样行动，以及控制权在谁手中。

AI 越强，人越容易担心权限交给了谁、配置为什么失效、换一个工具后 AI 为什么又从头开始。FyAgent 希望把这些选择留在人这一侧：不是给所有人复制同一个机器人，而是帮助每个人逐步拥有、培养和管理自己的 AI。

长期记忆与可延续的数字人格是产品继续建设的方向；下面列出的才是当前版本已经提供的能力。

## 今天已经能做什么

| 用户看到的能力 | 当前功能 |
| --- | --- |
| AI 大脑 | 管理模型供应商与模型选择，使用预设或自定义兼容接口，一键切换 |
| 工具连接 | 集中管理 MCP 服务，并同步到支持的 AI 工具 |
| AI 技能 | 维护 Skills，让常用能力不必在每个工具里重复安装和配置 |
| 行为指令 | 管理可复用的 Prompts，把常用工作方法带到不同工具中 |
| 调度与恢复 | 通过本地代理转发请求，设置故障转移，并检查模型是否可用 |
| 使用账本 | 汇总 token 用量和预估费用，方便核对日常开销 |
| 工作延续 | 从会话和工作区继续此前工作，并备份、同步配置 |

FyAgent 的工作数据默认保存在本机 `~/.fyagent`。配置更新使用 SQLite 和原子写入；通过 `fyagent://` 导入配置时，应用会先展示变更内容，再决定是否写入。

## 快速开始

1. 从 [GitHub Releases](https://github.com/fy-agent/fyagent/releases/latest) 下载适合当前系统的安装包。
2. 打开“供应商”，添加你正在使用的服务；预设会填好常用字段。
3. 选中供应商并点击“应用”，确认 FyAgent 即将写入的配置。
4. 在目标 AI 工具里发送一条简单请求。基础连接正常后，再添加工具连接、Skills 或行为指令。

完整说明见[简体中文手册](docs/user-manual/zh/README.md)，也提供 [English](docs/user-manual/en/README.md) 和 [日本語](docs/user-manual/ja/README.md)。

## 下载与发布可信度

发布文件名如下：

- macOS：`FyAgent-X.Y.Z-macOS.dmg`、`FyAgent-X.Y.Z-macOS.zip`
- Windows：`FyAgent-X.Y.Z-Windows-x64-setup.exe`、`FyAgent-X.Y.Z-Windows-arm64-setup.exe`
- Linux x64：`FyAgent-X.Y.Z-Linux-x86_64.AppImage`、`FyAgent-X.Y.Z-Linux-x86_64.deb`、`FyAgent-X.Y.Z-Linux-x86_64.rpm`
- Linux arm64：`FyAgent-X.Y.Z-Linux-arm64.AppImage`、`FyAgent-X.Y.Z-Linux-arm64.deb`、`FyAgent-X.Y.Z-Linux-arm64.rpm`

Windows 当前只提供 NSIS 安装程序，不提供 MSI 或便携 ZIP。macOS 构建使用 ad-hoc 签名，未使用 Apple Developer ID 签名，也未经 Apple 公证。Flatpak 仅供自行构建，不属于官方发布产物。

安装前请阅读发布说明，并核对校验和、`signing-status.json` 和构建证明。`NotSigned` 只是签名状态，不能单独证明文件安全。各系统的具体步骤见[安装说明](docs/user-manual/zh/1-getting-started/1.2-installation.md)，版本记录见[发布说明索引](docs/release-notes/README.md)。

## 常见问题

<details>
<summary><strong>FyAgent 会把数据保存在哪里？</strong></summary>

默认保存在本机 `~/.fyagent`。具体配置位置和备份方法见[配置文件说明](docs/user-manual/zh/6-faq/6.1-config-files.md)。
</details>

<details>
<summary><strong>遇到安装或配置问题，应该去哪里提问？</strong></summary>

先查看[常见问题手册](docs/user-manual/zh/6-faq/6.2-questions.md)，再到 [Q&A 讨论区](https://github.com/fy-agent/fyagent/discussions/categories/q-a)描述版本、系统、相关工具和已经尝试过的步骤。可稳定复现的软件缺陷请提交 [Bug Report](https://github.com/fy-agent/fyagent/issues/new?template=bug_report.yml)。
</details>

<details>
<summary><strong>FyAgent 已经具备长期记忆和完整数字人格了吗？</strong></summary>

还没有。当前版本先解决模型、工具连接、Skills、行为指令、配置与使用记录的统一管理。长期记忆和跨工具延续的数字人格属于产品愿景，只有在真实功能完成并经过验证后，才会列为现有能力。
</details>

<details>
<summary><strong>FyAgent 是开源软件吗？</strong></summary>

FyAgent 是源码可用软件，不是 OSI 定义的开源软件。FyAgent 自有组件和修改采用 PolyForm Noncommercial License 1.0.0；继承自 CC Switch 的部分继续使用 MIT 许可证。详见[授权说明](LICENSING.md)。
</details>

## 参与社区

- 使用问题与排查：[Q&A](https://github.com/fy-agent/fyagent/discussions/categories/q-a)
- 尚未成形的功能想法：[Ideas](https://github.com/fy-agent/fyagent/discussions/categories/ideas)
- 分享你的 AI 配置与工作方式：[Show and tell](https://github.com/fy-agent/fyagent/discussions/categories/show-and-tell)
- 可复现缺陷与明确任务：[Issues](https://github.com/fy-agent/fyagent/issues)

社区行为准则、支持范围和贡献流程见 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)、[SUPPORT.md](SUPPORT.md) 与 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 参与开发

本仓库统一通过 `mise` 进入开发环境：

```bash
mise trust
mise run bootstrap
mise run dev
mise run build
```

提交 PR 前请运行 `mise run check`。工具链与小范围检查命令见[开发文档](docs/fyagent/development/README.md)。

## 项目沿革与授权

FyAgent 的前身 VibeKey 曾设想把 AI 配置和操作权装进一块可以随身携带的实体键盘。项目继续推进后，我们发现真正需要随身带走的不是一块硬件，而是每个人自己的 AI 选择、习惯和工作方式。于是产品从硬件控制器转向跨平台桌面软件，也把名字改成了 **FyAgent（For You Agent）**。

当前桌面应用基于 CC Switch 演进，并继续保留继承代码的原作者版权和许可证声明。FyAgent 产品名称、当前开发与新增部分由 FyAgent 项目维护。

FyAgent 自有组件和修改采用 [PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt)，商业使用须另行取得书面授权。详见 [LICENSE](LICENSE)、[LICENSING.md](LICENSING.md) 和 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
