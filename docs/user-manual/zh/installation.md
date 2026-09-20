# 安装与更新

从 [fy-agent/fyagent Releases](https://github.com/fy-agent/fyagent/releases) 下载对应系统和架构的安装包，核对该 Release 的签名说明、SHA-256、源码 SHA 和 attestation。

| 系统    | 要求                                   | 安装包                                                                           |
| ------- | -------------------------------------- | -------------------------------------------------------------------------------- |
| Windows | Windows 10 及以上，x64 / ARM64         | `FyAgent-X.Y.Z-Windows-x64-setup.exe` 或 `FyAgent-X.Y.Z-Windows-arm64-setup.exe` |
| macOS   | macOS 12 及以上，Intel / Apple Silicon | `FyAgent-X.Y.Z-macOS.dmg`                                                        |

Windows：运行安装程序并完成向导。安装程序面向整台机器；遇到信任提示时，先核对下载来源与该构建的签名状态，遵循组织安全策略。

macOS：打开 DMG，将 `FyAgent.app` 拖入「应用程序」，再从那里启动。若系统拦截，核对下载与验证信息后，在「系统设置 → 隐私与安全性」按系统提示选择「仍要打开」。保留 Gatekeeper 和下载隔离保护。

启动后进入「AI软件配置」，按用途选择推荐软件，或点击「跳过引导」浏览目录。扫描、安装目标软件的步骤见 [AI 软件配置](agents.md)。需要 Node.js / npx 或 uv / uvx 的 MCP，会在安装时标明运行环境要求；先准备对应环境再继续。

更新 FyAgent 时，从同一 Releases 页面下载适配的安装包并按该 Release 说明安装。卸载时，Windows 使用系统「设置 → 应用」，macOS 将应用移到废纸篓。个人配置默认保存在 `~/.fyagent/`；处理这些文件前先备份需要保留的数据。

[返回手册](README.md)
