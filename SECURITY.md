# 安全策略

## 支持的版本

本项目的安全维护政策是：仅最新发布的 FyAgent 版本接收安全更新，更早版本不再接收安全更新。

源码版本以 [Cargo 工作区配置](src-tauri/Cargo.toml) 的 `[workspace.package].version` 为准；这不代表该版本已经发布，也不构成安全验证保证。报告时请注明所用发布版本；使用源码构建时另附提交 SHA。

## 威胁模型与权限边界

FyAgent 是本地桌面应用，会读取和修改 AI 编程工具的配置、管理凭据、处理会话数据，并提供本地 HTTP 代理。来自网络、导入文件或其他程序写入的本机文件，都不能仅因进入本地应用而被视为可信。

### 渲染进程与原生命令

发布构建的前端资源来自 `frontendDist: "../dist"`；开发配置另有 `devUrl: "http://localhost:3000"`，见 [Tauri 配置](src-tauri/tauri.conf.json)。本策略把随应用发布的渲染代码作为可信组件，不把流经它的外部数据视为可信，也不以此豁免不可信输入触发的脚本执行或越权操作。

[主窗口能力配置](src-tauri/capabilities/default.json) 关联 `main` 窗口及其权限集合；[权限声明](src-tauri/permissions/) 和 [原生命令注册](src-tauri/src/lib.rs) 定义了 IPC 的相关入口。文件写入、凭据读取、网络请求和进程启动须结合各命令的输入来源及后端校验评估，不能仅凭调用发生在渲染进程中就判定安全。

当前 Tauri 配置的内容安全策略（CSP）如下，以配置文件为准：

```text
default-src 'self'; img-src 'self' data: https: http:; script-src 'self' 'sha256-ls27Lc1OytcVawXPVMplz6LyKz2i9KYAVBw3u79qyB4='; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost https: http:
```

`script-src` 包含一个明确的 SHA-256 哈希许可，不能描述成仅允许 `'self'` 或禁止所有内联脚本；图片和连接来源允许 HTTP/HTTPS，样式允许内联。CSP 不替代 IPC 参数校验、导入确认或凭据访问控制。这里不作全库 HTML 注入点数量、动态求值调用数量或“不存在脚本注入”的绝对保证。

[会话终端命令](src-tauri/src/commands/session_manager.rs) 将调用方提供的 `command` 交给 [终端模块](src-tauri/src/session_manager/terminal/mod.rs)，按命令执行；该模块当前仅支持在 macOS 恢复终端会话。工作目录的转义或独立参数传递不能用来证明命令文本安全。不可信会话数据或导入内容若能影响实际执行命令，仍属于报告范围。

### 凭据与本机文件

[托管账号服务](src-tauri/src/services/managed_auth/service.rs) 使用 [Secret 服务](src-tauri/src/services/secret/) 管理凭据；原生后端分别使用 [macOS Keychain](src-tauri/src/services/secret/platform/macos.rs) 和 [Windows Credential Manager](src-tauri/src/services/secret/platform/windows.rs)。这不意味着凭据永远不会写入其他文件：例如 [OpenCode 连接实现](src-tauri/src/services/managed_auth/consumers/opencode.rs) 会将所需认证材料写入目标工具的 `auth.json`。登录、迁移、连接写入、刷新及日志中的凭据处理都在报告范围内。

旧 WebDAV/S3 云同步不再作为现行功能入口。[设置结构与前端输出](src-tauri/src/settings.rs) 仅保留旧字段的兼容数据，并从前端输出中移除；[设置保存](src-tauri/src/commands/settings.rs) 保留已有旧字段。[命令模块](src-tauri/src/commands/mod.rs) 与原生命令注册不再提供旧云同步入口。本策略不将这些旧字段描述为仍在使用的远程同步载荷。

## 报告范围

### 在范围内

以下入口或路径涉及不可信输入或敏感权限；请提供输入来源、处理链路与实际影响：

| 输入或路径 | 当前实现依据 |
| --- | --- |
| `fyagent://` 深链接的解析与导入，包括供应商、提示词、MCP 和 Skill 内容 | [协议配置](src-tauri/tauri.conf.json)、[深链接命令](src-tauri/src/commands/deeplink.rs)、[解析与导入](src-tauri/src/deeplink/) |
| 本地 HTTP 代理入站请求及上游响应；包括配置为非回环监听时其他主机可达的请求 | [代理配置](src-tauri/src/proxy/types.rs)、[监听实现](src-tauri/src/proxy/server.rs)、[代理处理](src-tauri/src/proxy/) |
| SQL 导入、本机数据库备份恢复，以及导入后的配置写入 | [导入与恢复命令](src-tauri/src/commands/import_export.rs)、[数据库备份实现](src-tauri/src/database/backup.rs) |
| 便携连接配置的读取、预览与导入 | [配置包命令](src-tauri/src/commands/config_pack.rs)、[配置包服务](src-tauri/src/services/config_pack/) |
| 会话文件、会话迁移包、恢复到目标工具的数据与路径 | [会话命令](src-tauri/src/commands/session_manager.rs)、[迁移命令](src-tauri/src/commands/session_migration.rs)、[迁移实现](src-tauri/src/session_manager/migrate/) |
| 托管账号登录回调、设备码登录响应、旧凭据迁移及向目标工具写入认证材料 | [登录实现](src-tauri/src/services/managed_auth/login.rs)、[凭据迁移](src-tauri/src/services/managed_auth/migration.rs)、[工具连接实现](src-tauri/src/services/managed_auth/consumers/) |
| 第三方可修改的工具配置、导入的 MCP 命令与 Skill 内容 | [供应商服务](src-tauri/src/services/provider/)、[MCP 服务](src-tauri/src/services/mcp.rs)、[Skill 服务](src-tauri/src/services/skill.rs) |

另外，以下问题同样在范围内：

- 不可信内容在 WebView 中执行脚本、导航到非预期来源或触达原生命令的完整利用链。
- API Key、令牌等凭据进入日志、错误信息、导出内容或不应接收它们的目标。
- 构建、发布与签名链路受到攻击。

### 不在范围内

- 仅从 DevTools 或自行修改的前端直接调用 IPC，未展示不可信来源进入该接口的路径。这一排除不适用于深链接、导入数据、代理请求或脚本注入导致的调用。
- 路径和内容均由用户主动选择、且没有不可信输入参与的常规文件操作。若第三方输入控制路径或内容，不能据此排除。
- 用户亲自编写并明确启用的 MCP、终端或脚本按预期执行命令。若可执行内容来自导入或深链接，确认界面隐瞒、截断或错误展示命令、参数、环境变量或脚本正文，以及未经用户明确决定就启用的情形，仍可报告；这是报告范围要求，不是对所有入口均已提供确认界面的声明。
- 用户主动对自己的本地实例实施、且不涉及不可信输入跨越边界的拒绝服务。
- 未给出受支持版本上可行利用路径的自动扫描结果，或仅影响不受支持版本的问题。

## 私密报告渠道

**请不要通过公开的 GitHub Issue 报告安全漏洞。** 请使用 [GitHub 安全公告的私密报告入口](https://github.com/fy-agent/fyagent/security/advisories/new)。

报告请包含：

- 漏洞描述、潜在影响、受影响版本及操作系统；源码构建另附提交 SHA。
- 输入由谁控制，以及从该来源到受影响代码的完整数据路径。
- 在受支持版本上的复现步骤、所需配置与用户交互。
- 脱敏后的最小样例；不要附真实 API Key、令牌或完整认证文件。

## 响应与披露政策

- 确认收到：48 小时内。
- 初步评估：7 天内。
- 关键问题修复：14 天内。

采用协调披露流程：私密提交、确认并修复、发布补丁，再公开漏洞详情。除非报告者要求匿名，否则在发布说明中致谢。

维护者可在修复就绪后通过安全公告流程申请 CVE 编号；属于本策略范围不等于一定获得 CVE。严重度采用 CVSS（v3.1 或 v4.0）评分，并反映所需的用户交互与前置本地访问条件。

安全修复通过补丁版本发布，并在 [GitHub Releases](https://github.com/fy-agent/fyagent/releases) 公告。建议使用最新发布版本。
