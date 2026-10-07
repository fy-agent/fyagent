# 安装与更新

从 [fy-agent/fyagent Releases](https://github.com/fy-agent/fyagent/releases) 下载对应系统和架构的安装包，核对该 Release 的签名说明、SHA-256、源码 SHA 和 attestation。

| 系统    | 要求                                   | 安装包                                                                           |
| ------- | -------------------------------------- | -------------------------------------------------------------------------------- |
| Windows | Windows 10 及以上，x64 / ARM64         | `FyAgent-X.Y.Z-Windows-x64-setup.exe` 或 `FyAgent-X.Y.Z-Windows-arm64-setup.exe` |
| macOS   | macOS 12 及以上，Intel / Apple Silicon | `FyAgent-X.Y.Z-macOS.dmg`                                                        |

Windows：运行安装程序并完成向导。安装程序面向整台机器；遇到信任提示时，先核对下载来源与该构建的签名状态，遵循组织安全策略。安装包被拦截时见[下文](#windows-安装包被拦截时)。

macOS：打开 DMG，将 `FyAgent.app` 拖入「应用程序」，再从那里启动。若系统拦截，核对下载与验证信息后，在「系统设置 → 隐私与安全性」按系统提示选择「仍要打开」。保留 Gatekeeper 和下载隔离保护。

启动后进入「AI软件配置」，按用途选择推荐软件，或点击「跳过引导」浏览目录。扫描、安装目标软件的步骤见 [AI 软件配置](agents.md)。需要 Node.js / npx 或 uv / uvx 的 MCP，会在安装时标明运行环境要求；先准备对应环境再继续。

更新 FyAgent 时，从同一 Releases 页面下载适配的安装包并按该 Release 说明安装。卸载时，Windows 使用系统「设置 → 应用」，macOS 将应用移到废纸篓。个人配置默认保存在 `~/.fyagent/`；处理这些文件前先备份需要保留的数据。

## Windows 安装包被拦截时

当前 Windows 安装包（`FyAgent-X.Y.Z-Windows-x64-setup.exe` 与 `FyAgent-X.Y.Z-Windows-arm64-setup.exe`）为面向整台机器的 NSIS 安装程序（安装时会出现 UAC 管理员授权）。目前 Windows 安装包尚未包含 Authenticode 签名，在 GitHub Release 页面的「Windows installer signing status」表格中标记为 `NotSigned`。

遇到系统或安全软件拦截提示时，请遵循「先校验、后放行」的原则，不要随意关闭系统安全防护：

1. **先校验（文件哈希与构建证明）**：放行或运行前，务必先核对文件来源与完整性。
   - 仅从官方 [GitHub Releases](https://github.com/fy-agent/fyagent/releases) 页面下载安装包。
   - 打开 PowerShell，运行以下命令计算安装包的 SHA-256（ARM64 安装包替换为对应文件名）：
     ```powershell
     Get-FileHash .\FyAgent-X.Y.Z-Windows-x64-setup.exe -Algorithm SHA256
     ```
     核对计算输出的 Hash 是否与对应 Release 页面表格中的 SHA-256 完全一致。如果哈希对不上，说明文件损坏或已被篡改，请立即删除文件，切勿运行。
   - 若安装了 GitHub CLI，还可通过 Release 附带的构建证明（attestation）验证其真实性：
     ```powershell
     gh attestation verify .\FyAgent-X.Y.Z-Windows-x64-setup.exe --repo fy-agent/fyagent
     ```
2. **Microsoft Defender SmartScreen 提示「Windows 已保护你的电脑」**：
   - 运行未签名的安装程序时，若弹出 SmartScreen 提示窗口，点击「更多信息」。
   - 确认显示的应用名称和文件名与下载的安装程序一致。
   - 点击「仍要运行」，随后在弹出的用户账户控制（UAC）窗口中确认管理员授权即可继续安装。
3. **文件被标记为来自 Internet（解除锁定）**：
   - 从网络下载的文件可能被系统添加 Web 标记。若双击安装包无反应或被系统阻止，右键点击安装包选择「属性」。
   - 在「常规」选项卡底部的「安全」区域，勾选「解除锁定」复选框并点击「确定」，再运行安装包。（注：仅当属性中出现「解除锁定」复选框时才需要此操作。）
4. **智能应用控制（Smart App Control，Windows 11）直接阻止**：
   - Windows 11 开启智能应用控制时，会自动阻止运行未签名应用。此时系统不会提供「仍要运行」按钮，也无法对单个文件单独添加白名单或放行。
   - 若要在个人设备上运行，只能在「Windows 安全中心 → 应用和浏览器控制 → 智能应用控制设置」中将其关闭。
   - **注意**：关闭智能应用控制会降低系统整体防护能力，关闭前请审慎权衡；且在部分 Windows 版本上，一旦关闭后不能直接重新打开（可能需要重置或重装系统）。若不想降低系统防护，请等待后续提供签名的版本。企业或组织托管设备请遵循组织 IT 策略，由系统管理员处理。
5. **杀毒软件或 Defender 误报隔离**：
   - 若安全软件将安装包误报为威胁并拦截或隔离，首先务必按第 1 步核对 SHA-256 哈希值，确保文件未被篡改。
   - 切勿为了安装而整体关闭杀毒软件或实时防护。
   - 确认哈希一致后，仅对该安装文件做最小范围的单文件排除或从隔离区恢复。
   - 可向安全厂商提交误报样本协助改进识别（Microsoft Defender 误报提交入口：[Microsoft 安全智能提交页面](https://www.microsoft.com/wdsi/filesubmission)）。
   - 也可前往 [Q&A 讨论区](https://github.com/fy-agent/fyagent/discussions/categories/q-a) 反馈拦截情况。

[返回手册](README.md)
