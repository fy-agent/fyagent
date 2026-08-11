<div align="center">
  <img src="assets/fyagent.png" width="128" alt="FyAgent 图标">
  <h1>FyAgent</h1>
  <p><strong>For You Agent</strong>——AI 时代的个人随身数字人格。</p>
  <p>把你的模型、供应商、技能、提示词和工作方式，带到每一个 AI 工具里。</p>
  <p><a href="README.md">English</a> · <a href="README_JA.md">日本語</a></p>
</div>

## For You Agent：为你而生，也由你掌控

AI 越强，人越需要知道它正在使用什么模型、拿着哪些权限、改了什么配置，以及出了问题怎样恢复。FyAgent 名字里的 `Fy` 来自 **For You**：它不是替所有人做同一件事的通用机器人，而是为你保存选择、承接习惯、连接工具的个人 Agent 控制面。

这里说的“数字人格”，不是一个模仿你说话的虚拟头像。它更像一套能够表达“你怎样使用 AI”的个人上下文：你信任哪些供应商，常用哪些模型，装了哪些 MCP、Prompts 和 Skills，不同项目怎样组合工具，以及哪些数据只留在自己的设备上。理想状态下，换工具、换项目甚至换电脑时，你不必从零重新教一遍；熟悉的工作方式可以继续跟着你。

这个想法最早叫 **VibeKey**。当时我们设想把 AI 配置和操作权装进一个可插拔的实体键盘，用按键、旋钮和状态灯给人明确的控制感。但项目继续推进后，真正值得随身带走的并不是一块硬件，而是每个人自己的 AI 工作方式。于是 VibeKey 更名为 **FyAgent（For You Agent）**，产品也从硬件控制器转向跨平台桌面软件：保留“控制权在用户手中”的初衷，让它不再受一块设备限制。

现在的 FyAgent 先把这套愿景里最基础、也最容易失控的一层做好：集中管理供应商、扩展、代理路由和用量记录。目前支持 Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw 和 Hermes。换模型、换接口或换工具时，不必再到处找 JSON、TOML 和 `.env` 手改；重要配置尽量可见、可确认，也能在需要时备份和恢复。

> **发布状态：** FyAgent 仍在持续开发。升级前请备份重要配置，安装前也请阅读当次发布的可信度说明。

## 它能帮你做什么

- 用内置预设或自定义兼容接口添加供应商，随后一键切换，不再反复改配置文件。
- 统一管理 MCP 服务、常用 Prompts 和 Skills，并同步到支持的工具。
- 用本地代理转发请求，设置故障转移规则，并检查模型是否可用。
- 汇总 token 用量和预估费用，方便核对日常开销。
- 从会话和工作区列表继续之前的工作，不用翻找各个工具的历史目录。
- 备份和同步配置，同时让密钥留在你控制的设备上。

FyAgent 的工作数据默认保存在本机 `~/.fyagent`，配置更新使用 SQLite 和原子写入，尽量避免写到一半留下坏文件。通过 `fyagent://` 导入配置时，应用会先展示变更内容，再决定是否写入。

## 下载与安装

请从 [GitHub Releases](https://github.com/fy-agent/fyagent/releases) 下载适合当前系统的安装包。发布文件名如下：

- macOS：`FyAgent-X.Y.Z-macOS.dmg`、`FyAgent-X.Y.Z-macOS.zip`
- Windows：`FyAgent-X.Y.Z-Windows-x64-setup.exe`、`FyAgent-X.Y.Z-Windows-arm64-setup.exe`
- Linux x64：`FyAgent-X.Y.Z-Linux-x86_64.AppImage`、`FyAgent-X.Y.Z-Linux-x86_64.deb`、`FyAgent-X.Y.Z-Linux-x86_64.rpm`
- Linux arm64：`FyAgent-X.Y.Z-Linux-arm64.AppImage`、`FyAgent-X.Y.Z-Linux-arm64.deb`、`FyAgent-X.Y.Z-Linux-arm64.rpm`

Windows 当前只提供 NSIS 安装程序，不提供 MSI 或便携 ZIP。macOS 构建使用 ad-hoc 签名，未使用 Apple Developer ID 签名，也未经 Apple 公证。Flatpak 仅供自行构建，不属于官方发布产物。

安装前请先看发布说明，并核对校验和、`signing-status.json` 和构建证明。`NotSigned` 只是签名状态，不能单独证明文件安全。各系统的具体步骤见[安装说明](docs/user-manual/zh/1-getting-started/1.2-installation.md)。

## 第一次使用

1. 打开“供应商”，添加你正在使用的服务。选预设时，常用字段会自动填好，你只需补上凭据和自定义接口地址。
2. 选中供应商并点击“应用”。FyAgent 会预览并写入相应工具的配置。
3. 打开目标编程工具，先发一条简单请求确认链路正常。
4. 基础调用跑通后，再添加 MCP、Prompts 或 Skills，排查问题会轻松很多。

完整说明见[简体中文手册](docs/user-manual/zh/README.md)，也提供 [English](docs/user-manual/en/README.md) 和 [日本語](docs/user-manual/ja/README.md)。版本记录统一放在[发布说明索引](docs/release-notes/README.md)。

## 参与开发

本仓库统一通过 `mise` 进入开发环境：

```bash
mise trust
mise run bootstrap
mise run dev
mise run build
```

提交 PR 前请运行 `mise run check`。工具链要求和更小范围的检查命令见[开发文档](docs/fyagent/development/README.md)，准备做较大改动时请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 项目沿革与授权

从产品愿景和名称沿革看，FyAgent 是 VibeKey 的延续与转型：从“把 AI 分身装进一块键盘”，走向“不受硬件限制的 For You Agent”。从软件工程沿革看，当前桌面应用基于 CC Switch 演进，对继承的代码继续保留原作者版权和许可证声明。FyAgent 这一产品名称、当前开发工作及新增部分由 FyAgent 项目维护。

FyAgent 是源码可用软件，并非 OSI 所定义的开源软件。FyAgent 自有组件和修改采用 [PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt)；商业使用须另行取得书面授权。源自 CC Switch 的部分仍采用 MIT 许可证。详见 [LICENSE](LICENSE)、[LICENSING.md](LICENSING.md) 和 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
