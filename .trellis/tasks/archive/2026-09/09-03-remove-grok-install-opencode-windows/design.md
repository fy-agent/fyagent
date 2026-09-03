# 技术设计：Grok 大陆 npm 一键安装与 OpenCode Windows x64

## 1. 设计边界

本任务做两项闭合变化：

1. 将 Grok Build 默认新装从原生 `x.ai`/GCS 改为官方 npm + 大陆镜像链，版本由应用内清单决定；保留归属模型与显式原生安装。
2. 将 OpenCode Desktop 的 macOS managed-desktop owner 扩展到 Windows x64 官方 NSIS，并去掉 GitHub latest 硬依赖。

二者不共享命令执行接口。OpenCode 走 `agent_install` 桌面产物链路；Grok 仍是 CLI Tooling owner。

不得新建通用“任意 npm 安装器”或第二套 Windows helper 运行时。

## 2. 当前态与目标态

| 产品/状态 | 当前行为 | 目标行为 |
| --- | --- | --- |
| Grok 未安装默认 install | native script / PowerShell | 官方 npm + 清单精确版本 + 镜像链 |
| Grok 显式 npm | `@latest` 或临时问 npmjs latest | 同一 `GrokNpmInstallPlan` |
| Grok 显式 native | 官方脚本 | 保留，仅用户显式选择 |
| Grok native 已安装 update | 部分路径 `update \|\| installer` | 只跑锚定 updater |
| Grok npm 已安装 update | 可 `@latest` / 裸 npm | 清单精确版本 + 已发现 npm anchor |
| OpenCode macOS | 先依赖 GitHub latest | stable DMG 始终可解析 |
| OpenCode Windows x64 | `PlatformUnsupported` | 官方 NSIS + 既有 EXE handoff |
| OpenCode Windows ARM64 | fail-closed | 继续 fail-closed |

## 3. Grok npm 安装计划

### 3.1 单一 owner

在 Tooling / user-helper 共享层新增纯数据结构（helper crate 可放无 I/O 的校验，宿主负责选源与执行）：

```text
GrokNpmInstallPlan
  package            = "@xai-official/grok"
  version            = manifest.version          # 精确，永不 latest
  registry           = one of closed allowlist
  package_integrity  = sha512-...
  platform_package   = "@xai-official/grok-{os}-{arch}"
  platform_integrity = sha512-...
  allow_install_scripts = bool   # npm major >= 12
```

所有 `npm i -g @xai-official/grok@...` 字符串必须由该计划生成。禁止第二处手写 `@latest`。

Registry 闭集：

```text
https://mirrors.tencent.com/npm/
https://repo.huaweicloud.com/repository/npm/
https://registry.npmmirror.com/
https://registry.npmjs.org/
```

校验：HTTPS、无 userinfo、host 精确匹配、path 为已知前缀。npmmirror 与任何源都不得把 tag `latest` 写入 spec。

### 3.2 版本清单

文件例如 `src-tauri/src/services/tooling/grok_npm_manifest.json`，`include_str!` 编译进宿主。信任根是已签名的 FyAgent 应用，不另建 catalog 密钥。

形状：

```json
{
  "channel": "stable",
  "package": "@xai-official/grok",
  "version": "1.0.13",
  "published_at": "2026-08-28T00:00:00Z",
  "integrity": {
    "@xai-official/grok": "sha512-...",
    "@xai-official/grok-win32-x64": "sha512-...",
    "@xai-official/grok-win32-arm64": "sha512-...",
    "@xai-official/grok-darwin-x64": "sha512-...",
    "@xai-official/grok-darwin-arm64": "sha512-..."
  }
}
```

写入当期值前，实现者必须向 `registry.npmjs.org` 复核 SHA-512。研究文件中的哈希是起点。Linux 平台包可出现在清单中但不在本任务安装。缺当前平台 integrity 则 fail-closed，不改用别的 arch 包。

展示用 latest 与“是否可更新”也读这份清单，不再把 grok 的 latest 查询打到 npmjs。其他工具的 npm latest 逻辑保持不动。

### 3.3 选源与执行

```text
load manifest
  -> require Node/npm
  -> detect npm major
  -> for registry in order:
       query metadata for exact version (not latest)
       compare dist.integrity to manifest
       if mismatch/missing -> next
       npm install -g @xai-official/grok@<version>
         --registry=<registry>
         [optional --allow-scripts=@xai-official/grok]
       set GROK_NPM_REGISTRY to the same registry for this process
       grok --version == version -> success
       else next
  -> all failed -> stable error, keep old install
```

不得 `npm config set`。若 npm 已全局安装同一包，优先在不影响现有二进制可用性的前提下安装；失败则旧版本仍可运行。

默认未安装 `install`：`expected_owner=None` 时规划 `OfficialNpm` 而不是 `NativeFresh`。`install_official_npm` 与默认 install 共用计划。`expected_owner=Native` 才是 `NativeFresh`。

已安装 npm 的 update 使用同一计划；若本地规范化版本 ≥ 清单版本，返回无需更新 / 成功且不降级。

### 3.4 Windows helper 合同

宿主在 launch `grok-tool` 前构造计划。计划通过已认证 pipe 的有界消息字段下发（固定字符串长度上限，registry 必须过 allowlist）。CLI 仍是：

```text
grok-tool --action observe|install|update [--owner none|native|npm]
```

禁止把 registry/version/integrity 做成自由 CLI 参数（避免日志与注入面）。OpenCode 的 `AgentInstallerProduct` 扩展不得重排 Grok wire family。

Helper npm 执行：

```text
npm.cmd i -g @xai-official/grok@<plan.version> --registry=<plan.registry> [allow-scripts]
```

无计划或计划校验失败：`ToolExecutionFailed` / 稳定错误，不回退 `@latest`，不回退 PowerShell。

### 3.5 macOS

删除 `fetch_npm_latest_for_package` 作为 grok 安装前置。`execute_official_npm` 接收完整计划。anchor 到已发现 npm 时把同一 argv 交给该 npm；无 anchor 的**新装**可以使用 PATH 上的 npm（这是新装，不是 update fallback）。**更新**仍禁止在 anchor 缺失时用裸 npm。

删除 `update || installer`。native update 只跑锚定二进制。

## 4. OpenCode 官方 stable source

### 4.1 Canonical endpoints

```text
macOS arm64  https://opencode.ai/download/stable/darwin-aarch64-dmg
macOS x64    https://opencode.ai/download/stable/darwin-x64-dmg
Windows x64  https://opencode.ai/download/stable/windows-x64-nsis
```

后端不用 `/zh`。

### 4.2 去掉 GitHub 硬依赖

```text
fixed opencode.ai stable alias
  -> always construct backend release capability
optional GitHub version enrichment
  -> display only; failure does not change installability
```

`release_id` 由 product/platform/arch/format/alias/endpoint 生成。`display_version` 可为 `None`。Windows 映射：

```text
platform     = windows
architecture = x86_64
format       = exe
endpoint     = opencode-windows-x64-nsis
alias        = stable
```

Arch token 分平台校验。Windows ARM64/x86 → `PlatformUnsupported`。

## 5. OpenCode Windows identity 与执行

复用：

```text
fixed stable alias
  -> shared streaming download
  -> artifact revalidation
  -> PE + WinVerifyTrust + exact signer
  -> PackageBridge
  -> AgentInstallerProduct::OpenCode
  -> Alice helper pipe
  -> ShellExecuteExW(verb=open, no args)
  -> job succeeded as installer handoff
  -> later inventory proves installed
```

追加 wire code，不重排 qoderwork/trae-work/workbuddy，不复用 Grok install 值。

ProductName `OpenCode` + exact signer + 允许的 i386 installer stub 特例。空白 FileDescription 可接受。`windows_product_names` / relative EXE / signer 只有 HIL 证据后才能写入；若本机无法做 Windows HIL，Windows install readiness 必须保持 fail-closed，不得靠研究时路径猜测。

## 6. 前端与文案

- Grok 主 CTA：安装（后端默认 npm 计划）。次要：官方原生安装、改用另一归属。
- 不把镜像 URL、完整性哈希、npm 命令放进 DOM。
- OpenCode Windows handoff：安装器已打开，完成后刷新。不要写“已安装”。
- 不保证中国大陆 Grok 在线服务。

## 7. 测试设计

### Grok

- 计划生成：精确版本、四源顺序、无 `@latest`、无 `npm config set`。
- 哈希失败换源；全部失败保留旧版语义（mock）。
- npm 11 不加 allow-scripts；npm 12 添加窄范围参数。
- 默认 install → OfficialNpm；`expected_owner=Native` → NativeFresh。
- helper：无计划拒绝；有计划只使用 plan argv。
- planner：native update 无 installer fallback；npm update 无 bare fallback。
- UI：主按钮存在；不展示命令。

### OpenCode

- 三条 canonical alias；Windows x64 EXE；unsupported arch。
- GitHub failure 时 source 仍成功。
- helper 新产品 parser/wire/bridge。
- macOS source 回归。

## 8. 回滚

- Grok 镜像安装若大面积失败：默认 install 可临时改回显式提示，但不得恢复 `@latest`。
- OpenCode Windows 身份漂移：只关 Windows OpenCode install，保留 macOS。
- helper OpenCode 协议问题：撤回新增 product code。
- Grok update 无法锚定：关闭该 owner 的 update，不得用新装伪装更新。

## 9. 并行实现边界

| 工作流 | 可写 | 不可写 |
| --- | --- | --- |
| Grok npm | `tooling/grok.rs`、`lifecycle.rs`、`versions.rs` 中 grok latest 分支、新 `grok_npm*.rs/json`、`user-helper/src/grok.rs`、`user-helper/src/windows.rs` 的 grok npm 执行、grok 相关测试 | `agent_install/sources/opencode.rs`、OpenCode product identity、`AgentInstallerProduct` 枚举 |
| OpenCode Windows | `sources/opencode.rs`、`desktop.rs` 中 OpenCode 解析、`windows.rs` OpenCode 产品描述、`user-helper/src/cli.rs` 的 product 追加、OpenCode 测试 | Grok `@latest` 命令、`GrokNpmInstallPlan`、默认 install 规划 |

共享文件（`lifecycle_policy.rs`、i18n、AboutSection、Agent 卡片、SPEC、用户手册）由第二阶段或主会话在两工作流代码落地后统一改，避免并行互相覆盖。
