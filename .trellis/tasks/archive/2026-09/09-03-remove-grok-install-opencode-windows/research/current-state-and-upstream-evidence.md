# 当前实现与上游证据

- 调研日期：2026-09-03
- FyAgent 基线：`dev/laiyongjie` @ `b3a297ab6eed4295c7ce486d0e509744731612f1`
- OpenCode upstream `dev`：`b578b7261fc9ec4917fe272df5cc4bd8a056cd5d`
- 调研时 OpenCode latest release：`v1.18.27`

本文件区分：仓库当前事实、官方上游事实、研究时产物观察、仍需 Windows/HIL 冻结的事实。研究时值不得直接升级为长期硬编码。

## 1. 官方来源

### OpenCode

- 官方下载页：`https://opencode.ai/download`
- 官方 stable Desktop endpoints：
  - `https://opencode.ai/download/stable/darwin-aarch64-dmg`
  - `https://opencode.ai/download/stable/darwin-x64-dmg`
  - `https://opencode.ai/download/stable/windows-x64-nsis`
- 用户提供的 `/zh/download/stable/...` URL 可返回同一类产物，但上游页面通过 locale router 生成本地化 path；backend source 应使用 locale-neutral product route。
- 官方仓库：`https://github.com/anomalyco/opencode`
- 上游 route implementation：`packages/console/app/src/routes/download/[channel]/[platform].ts`
- 上游 Electron Builder config：`packages/desktop/electron-builder.config.ts`
- 上游 publish workflow：`.github/workflows/publish.yml`

上游 route 在 stable channel 将三条 Desktop route映射为：

```text
darwin-aarch64-dmg -> opencode-desktop-mac-arm64.dmg
darwin-x64-dmg     -> opencode-desktop-mac-x64.dmg
windows-x64-nsis  -> opencode-desktop-win-x64.exe
```

route 由服务端解析 GitHub latest asset location，再按真实 asset location缓存并返回 body；客户端拿到的是 `opencode.ai` 下载响应。这解释了为什么 backend应优先 stable alias，同时不需要自己把 GitHub API设为安装硬门禁。

### Grok Build

- 官方产品页：`https://x.ai/grok/build`
- 官方仓库：`https://github.com/xai-org/grok-build`
- 原生 installer：macOS/Linux `https://x.ai/cli/install.sh`，Windows `install.ps1`，失败后退 GCS。
- 官方企业 npm 分发：`npm install -g @xai-official/grok`（不需要 `x.ai` / GCS）。
- 大陆适配证据与镜像结论见 `research/grok-mainland-npm-install.md`。

## 2. OpenCode 官方打包与产物观察

### 2.1 上游公开配置

OpenCode upstream当前桌面端使用 Electron/Electron Builder。prod配置包含：

```text
appId      = ai.opencode.desktop
productName = OpenCode
Windows target = nsis
oneClick   = true
perMachine = false
```

Windows publish workflow当前构建 x64与ARM64 asset，但官方下载 page/route type当前公开的是 Windows x64 NSIS。不能由“GitHub Release有 ARM64 asset”直接推导 FyAgent已支持 Windows ARM64。

### 2.2 官方 stable Windows x64 产物（2026-09-03）

HTTP 观察：

```text
status              = 200
content-type        = application/octet-stream
content-length      = 126191936
content-disposition = OpenCode Desktop Installer.exe
x-opencode-cache    = HIT
```

静态产物观察：

```text
sha256         = ed8900cc123db67ac9714c1a3051436eced0d7190c709ae53fea1e99f3dcca6c
format         = PE32 GUI, i386, Nullsoft Installer self-extracting archive
CompanyName    = OpenCode
ProductName    = OpenCode
FileDescription= empty
FileVersion    = 1.18.27
ProductVersion = 1.18.27
```

PKCS#7 certificate chain中观察到的 signer leaf subject：

```text
C=US
ST=Delaware
L=Dover
O=Anomaly Innovations, Inc https://anoma.ly/
CN=Anomaly Innovations, Inc https://anoma.ly/
```

限制：Mac上提取证书链和PE资源不能代替Windows `WinVerifyTrust`。实现时必须在当期官方artifact上重做Windows信任、exact signer、installed target与scope HIL。哈希/版本只用于说明本次调研具体看过哪个产物。

## 3. FyAgent 当前 OpenCode 实现

### 3.1 已有且应保留

`src-tauri/src/agent_install/sources/opencode.rs` 已固定：

```text
OPENCODE_DARWIN_AARCH64_DMG
OPENCODE_DARWIN_X64_DMG
OPENCODE_OFFICIAL_PAGE
```

`src-tauri/src/agent_install/desktop.rs` 已有 macOS bundle identity：

```text
ai.opencode.desktop
```

OpenCode 已被 Agent policy定义为 Desktop-only；不应恢复 OpenCode CLI installer。

### 3.2 当前缺口

- source resolver对 Windows任何arch返回 `PlatformUnsupported`；测试显式锁定该行为。
- `DESKTOP_PRODUCTS` 中 OpenCode的 `windows_product_names`、`windows_relative_exes` 为空。
- Windows signer policy没有 OpenCode。
- `AgentInstallerProduct` helper闭集只有 QoderWork、TRAE Work、WorkBuddy。
- `resolve_opencode_desktop_latest` 必须先成功访问 GitHub latest API，然后才返回稳定 alias。

最后一项是中国大陆友好目标中的真实结构性问题：下载 URL虽为 `opencode.ai`，客户端安装决策仍被 GitHub API可达性阻断。

## 4. 可复用 Windows EXE owner

当前 QoderWork CN / TRAE Work CN / WorkBuddy 已有：

1. fixed first-party source；
2. streamed job-owned artifact；
3. stable file revalidation；
4. PE ProductName/FileDescription + Authenticode exact signer + architecture admission；
5. protected PackageBridge；
6. frozen Explorer user / Alice helper / authenticated pipe；
7. fixed `ShellExecuteExW(open)`；
8. successful launch即 vendor-wizard handoff，不等待、不kill、不做 post-install proof。

OpenCode当前官方产物也是签名NSIS EXE，适合扩展该closed product adapter。Codex MSIX owner不适用。

## 5. Grok Build 当前残留面

### Policy / Agent façade

- `agent_install/lifecycle_policy.rs`：Grok CLI `install=true, update=true`。
- `agent_install/cli.rs` / `mod.rs`：install/update会路由到 Tooling lifecycle。
- tests锁定“Grok保留install/update”。

### Tooling backend

- `ToolLifecycleAction = Install | Update | InstallOfficialNpm`。
- macOS可下载并执行 `https://x.ai/cli/install.sh`。
- Windows可生成官方PowerShell installer。
- official npm route可执行 `npm i -g @xai-official/grok@latest`。
- development Windows native update仍有 `grok update || PowerShell installer` fallback。
- npm executor在无法从bin anchor npm时可退回裸 `npm`，存在创建新安装的可能。

### Windows helper

- CLI公开 `grok-tool --action observe|install|update [--owner ...]`。
- wire family为observe/install/update各三种owner variant。
- helper能执行 NativeFresh与OfficialNpm fresh mutation。

### Frontend / API / docs

- Settings `AboutSection` 将Grok设为唯一 writable tool，未安装显示Install。
- Settings和Agent均有 official npm安装/切换CTA。
- `GrokToolingPort`公开 `installOfficialNpm()`。
- `settingsApi` action union包含 `install|update|install_official_npm`。
- 四语言i18n和当前用户手册宣称FyAgent可安装Grok。
- Windows runtime、external-agent lifecycle、user-facing copy SPEC均固化该政策。

结论：此前变化只移除了**非 Grok** CLI installers；Grok install是被有意保留，不是已删除后的零散死代码。

## 6. Grok 安装决策（Round 6 修订）

```text
default fresh install: official npm + mainland registry chain + bundled exact version
explicit native install: keep, never auto-fallback from npm failure
owner-preserving update: keep; native has no installer fallback; npm has no @latest / bare PATH new-install
silent owner migration: forbidden
read-only observe/version/owner/conflicts: keep
```

OpenCode 不得复用 Grok helper wire family；新产品追加未占用值。

## 7. Source 与中国大陆网络决策

### 采用

- client-facing official `opencode.ai` stable aliases；
- no GitHub API hard prerequisite；
- versionless release capability；
- optional version metadata only if it cannot gate or destabilize action；
- shared proxy-aware HTTP client与现有 host/redirect policy；
- mainland HIL before claims。

### 不采用

- `/zh`硬编码（locale不是product identity）；
- GitHub asset direct URL作为primary；
- anonymous GitHub proxy/community mirror；
- Homebrew/npm/Scoop/WinGet/WSL fallback；
- ETag/Last-Modified/version filename guessing。

当前普通网络成功下载只能证明调研环境可达，不能证明中国大陆网络。任务必须明确这一不确定性。

## 8. 尚待执行阶段闭合的事实

以下不得在规划阶段猜测：

1. 当前 Electron NSIS在Windows实际默认安装目录；
2. current-user Uninstall/App Paths key与value；
3. installed EXE exact relative path、ProductName、signer、architecture；
4. custom path行为；
5. repeat install是否原位更新或并排新装；
6. one-click取消/失败的ShellExecute与installer交互；
7. 中国大陆多个网络的DNS/TLS/throughput/cache行为；
8. Windows ARM64稳定route和原生支持。

这些项目已作为Phase 0/HIL硬门禁写入PRD与实施清单。

## 9. OpenCode Windows 实现残余（2026-09-03）

- 源解析与 helper 产品 `opencode` / wire 14 已落地。
- Windows 身份未做 WinVerifyTrust HIL，因此 `windows_product_names`、relative EXE、signer 仍为空；Windows 不会开放 install/download。
- 归档前不得把 OpenCode 写成“Windows 已支持”。macOS DMG 行为不因此回退。

