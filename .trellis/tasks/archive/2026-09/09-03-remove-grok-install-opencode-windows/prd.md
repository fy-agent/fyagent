# 大陆可用的 Grok Build CLI 一键安装并补齐 OpenCode Windows x64 安装

## 0. 任务状态

- 优先级：P1
- 状态：规划完成后进入实现；用户已要求把大陆 npm 一键安装并入本任务并推进到归档
- 基线：`dev/laiyongjie` @ `b3a297ab6eed4295c7ce486d0e509744731612f1`
- 主平台：macOS（Grok 一键安装）与 Windows 11 x64（Grok 一键安装 + OpenCode Desktop）
- 回归平台：macOS arm64 / x64 的 OpenCode Desktop 源解析与既有 DMG 生命周期
- 执行方式：单任务、两条工作流并行（Grok npm 安装 / OpenCode Windows），再做前端文案、SPEC 与检查
- 核心约束：不得新建第二套下载器、签名校验器、NSIS 执行框架、Windows 用户态运行时、通用命令 helper 或完整二进制镜像系统

## 1. 目标

让中国大陆用户能在 macOS 与 Windows 上一键安装 **xAI 官方** Grok Build CLI，并且让 OpenCode Desktop 在 Windows x64 走现有签名 EXE handoff。安装成功不能被宣传成“Grok 在中国大陆可完整登录和推理”。

## 2. 背景与已确认事实

### 2.1 Grok：原生安装不适合作为大陆默认

xAI 没有官方大陆镜像。原生链是 `x.ai/cli` 再退到 GCS。`install.sh` / `install.ps1` 不能无侵入替换下载根。官方企业文档提供 `npm install -g @xai-official/grok`，安装期间不访问这两个原生主机。

当前 FyAgent 已区分 `native_internal` 与 `official_npm` 归属，但：

- 默认未安装 `install` 仍走 native（macOS 拉 `install.sh`，Windows helper 跑 `install.ps1`）；
- Windows helper 执行 `@xai-official/grok@latest`；
- macOS npm 路径虽冻结精确版本，却向 `registry.npmjs.org` 问 latest；
- 多处仍拼 `npm i -g @xai-official/grok@latest`；
- 没有 `--registry` / `GROK_NPM_REGISTRY`；
- native `update || installer` 与 npm 裸 PATH fallback 可在失败后新装。

2026-09-03 验证：腾讯云、华为云、npmmirror 均可安装精确版本 `1.0.13`，主包与 macOS/Windows 平台包 SHA-512 与官方一致；npmmirror 的 `@latest` 错误指向 `0.1.4`。证据见 `research/grok-mainland-npm-install.md`。

### 2.2 OpenCode：macOS 地址正确，源解析仍被 GitHub 阻断；Windows 缺 identity

macOS 已用官方稳定别名：

```text
https://opencode.ai/download/stable/darwin-aarch64-dmg
https://opencode.ai/download/stable/darwin-x64-dmg
```

但 `resolve_opencode_desktop_latest` 必须先访问 GitHub latest API。GitHub 不可达时即使 `opencode.ai` 可下载也会失败。

Windows 官方稳定 route：

```text
https://opencode.ai/download/stable/windows-x64-nsis
```

适合复用 Qoder/TRAE/WorkBuddy 的签名 EXE vendor-wizard handoff。研究时产物是 i386 NSIS stub、`ProductName=OpenCode`、`FileDescription` 空、signer 为 Anomaly Innovations。这些是当期证据，不是长期 pin。Windows ARM64 无官方稳定 route，fail-closed。

## 3. 已冻结的产品决策

### 3.1 Grok Build 一键安装

- 未安装时的**默认一键安装**（Settings 与 Agent 的主安装按钮，以及 `install` 且未指定 native owner）使用官方 npm 包 + 大陆优先镜像链 + 内置精确版本清单。
- 官方原生安装（`x.ai` 脚本 / PowerShell）保留为**显式次要动作**，不得作为大陆默认，也不得作为 npm 失败后的自动 fallback。
- 已是 native 的用户继续 native 更新；已是 npm 的继续 npm 更新。禁止静默迁移归属。
- 用户显式点“改用官方 npm / 改用官方原生”才允许 owner 切换。
- 版本真相是打进已签名 FyAgent 应用的清单（`include_str!` 即可，不新建独立 catalog 签名基础设施）。清单含精确版本与官方 npm SHA-512。客户端安装时不得向任何 registry 询问 `@latest`。
- 镜像顺序：腾讯云 → 华为云 → npmmirror（仅精确版本）→ npmjs。哈希不符或安装失败则换源，永不降到更旧目标版本。
- 不修改用户全局 `.npmrc`。只对本次进程设 `GROK_NPM_REGISTRY` 和 `npm --registry`。
- npm major ≥ 12 时附加 `--allow-scripts=@xai-official/grok`，禁止 dangerously-allow-all。
- 失败必须保留旧安装：先在不破坏现有安装的前提下安装/校验，成功后再作为当前安装；任一步失败不删除旧版本。
- Windows helper 只执行宿主下发的完整 `GrokNpmInstallPlan`，不得自己决定 latest。
- 产品文案不得声称“完整支持中国大陆 Grok 登录/推理”。只声称可以通过大陆 npm 镜像安装 CLI。

### 3.2 OpenCode

- 后端固定无语言前缀的官方稳定别名；`/zh` 不进入 backend identity。
- macOS 保留官方 DMG；Windows x64 使用 `windows-x64-nsis`，建模为 `PackageFormat::Exe`。
- 稳定别名必须在 GitHub API 失败时仍可安装。展示版本只能非阻断增强。
- 不默认第三方 GitHub 代理。大陆可达性必须 HIL 证明后才能写“国内可用”。
- Windows 首期只声明 x64。ARM64 fail-closed。
- Windows fresh install 成功语义是 vendor-wizard handoff，不是“已安装”。
- Windows update 不是强制首期；无 same-target HIL 则 fail-closed。macOS update 不得回退。

## 4. 功能与工程需求

### R0. Policy 与零副作用

- `lifecycle_policy.rs` 仍是唯一合法矩阵。Grok CLI 保持 `install=true` 与条件 `update`。OpenCode Desktop Windows x64 在 identity 闭合后允许 install。
- 默认 Grok `install` 规划官方 npm 计划，而不是 native fresh。
- 显式 native 安装仍走现有 native executor，但不得从 npm 失败自动跳过去。
- 请求 schema 闭集；renderer 不得提交 URL、命令、registry、版本或哈希。

### R1. Grok npm 安装计划（macOS + Windows 共用）

- 新增单一 owner：`GrokNpmInstallPlan`（包名、精确版本、选中 registry、主包/当前平台包 integrity、是否 allow-scripts）。
- 清单编译进应用。研究时 `1.0.13` 与 SHA-512 只是写入当期清单的证据；代码不得在多处硬编码该版本字符串。
- 删除所有可达的 `@xai-official/grok@latest` 安装/更新命令。
- npm 更新同一包时也使用清单精确版本 + 同一 registry 链；已安装版本 ≥ 清单版本则视为无需更新，不得降级。
- 检测 Node/npm 前置条件；npm 12+ 才加 allow-scripts。
- 安装后 `grok --version` 必须等于目标版本，否则该源失败并换源。
- 不创建 FyAgent 自建二进制 CDN。

### R2. Windows Grok helper

- 宿主把完整 npm 计划交给 helper。CLI 仍只接受 `observe|install|update` 与闭集 owner。计划走已认证 pipe 的有界字段，或等价的闭集协议扩展，不要把自由命令行交给 helper。
- helper 删除自行 `@latest`。无计划却要执行 npm 安装时 fail-closed。
- 保留 native 显式安装路径；默认未安装 install 不再规划 `NativeFresh`。
- install wire values 不改作 OpenCode 用。OpenCode 新产品必须追加未占用 wire code。
- formal elevated parent 仍只通过 frozen Explorer user helper；无 elevated fallback。
- native update 禁止 `update || installer`；npm update 必须锚定已发现的 package manager，禁止裸 PATH npm 新装。

### R3. Grok 前端、API 与文档

- 未安装时主按钮是一键安装（npm 计划）。可保留显式“官方原生安装”和已安装时的 owner 切换，但必须是用户点击，不得自动。
- 前端不得根据版本自己拼 npm 命令。
- 更新文案：说明使用官方 npm 包和国内镜像下载；不保证登录/推理在大陆可用；不展示 registry URL、哈希、绝对路径。
- 四语言 i18n、Settings API union、用户手册与测试与上述事实一致。

### R4. OpenCode stable source

- `sources/opencode.rs` 增加 `windows-x64-nsis`，映射 `Windows + X86_64 + Exe`。
- GitHub latest 降为非阻断 enrichment。versionless stable capability 必须可安装。
- 下载仍走 HTTPS、redirect 与 host allowlist。不加入第三方 proxy。
- Windows ARM64 / x86 / Linux 保持 `PlatformUnsupported`。

### R5. OpenCode Windows 闭合身份与 handoff

- 在 Windows 上用当期官方 stable EXE 冻结 WinVerifyTrust、exactly-one signer、PE ProductName、stub/installed arch、当前用户 scope、注册表证据。研究时值不得永久 pin。
- 空白 FileDescription 可接受，但必须与 ProductName + Authenticode + 稳定文件身份组合。
- 扩展现有 `DESKTOP_PRODUCTS` / helper `AgentInstallerProduct`，不新建 NSIS 框架。
- helper 固定 `ShellExecute(open)`，无静默参数，不等待退出。job 成功 = 安装器已交接。
- OpenCode CLI、Scoop、WinGet、npm、WSL、Codex MSIX 都不是 fallback。

### R6. 测试与 HIL

- 自动化覆盖：默认 install 生成精确版本 + registry 计划且不含 `@latest`；镜像哈希失败换源不降级；npm 12 allow-scripts 分支；helper 无计划拒绝；native 显式安装仍存在但不是默认；owner 不静默迁移；OpenCode GitHub 不可达仍可解析；OpenCode Windows product/helper；direct 请求不接受 registry/命令。
- Windows 11 x64 正式包 HIL 是 OpenCode Windows 支持声明的硬门禁。
- Grok 大陆安装 HIL 至少证明：在会阻断 `x.ai`/`storage.googleapis.com` 或官方 npmjs 的网络下，腾讯或华为镜像仍能装到清单版本。未做 HIL 时不得写“国内网络一定可用”，只能写“采用大陆 npm 镜像作为默认下载源”。

## 5. 非目标

- 不自建 Grok 二进制 CDN，不代理官方 install.sh/ps1。
- 不删除 Grok Provider、Auth、usage、只读探测。
- 不恢复非 Grok CLI 的 Settings 一键安装；OpenCode 只处理 Desktop。
- 不把 OpenCode 改成 npm/WinGet/Scoop。
- 不声明 OpenCode Windows ARM64 / Grok 登录在中国大陆一定可用。
- 不把研究时 OpenCode `1.18.27` 或证书序列号永久 pin。
- 不改写历史 release notes。
- 不为版本清单新建独立公钥体系；清单随已签名应用分发。

## 6. 验收标准

### A. Grok 一键安装（macOS + Windows）

- [ ] 未安装时默认 `install` 使用 `@xai-official/grok@<清单版本>`，并带允许列表内 `--registry`；命令与 helper 都不含 `@latest`。
- [ ] 清单版本与 SHA-512 来自编译进应用的单一文件；安装时不向镜像查询 latest。
- [ ] registry 顺序为腾讯云 → 华为云 → npmmirror → npmjs；npmmirror 只接受精确版本。
- [ ] 哈希不符、缺版本或 `grok --version` 不等于目标时换下一个源；不删除旧安装；不改目标版本。
- [ ] 不执行 `npm config set registry`。
- [ ] npm major ≥ 12 时使用 `--allow-scripts=@xai-official/grok`，旧 npm 不加该参数。
- [ ] Windows helper 无宿主计划时拒绝 npm 安装；有计划时只执行该计划。
- [ ] 已安装 native 用户更新仍走锚定 `grok update`，失败不回退安装器/npm。
- [ ] 已安装 npm 用户更新走同一清单计划 + package-manager anchor，无裸 npm fallback。
- [ ] 显式 native 安装仍可用，且不会在 npm 失败后自动执行。
- [ ] Settings/Agent 主按钮是一键安装；owner 切换必须用户显式触发。
- [ ] 文案不承诺大陆登录/推理可用。

### B. OpenCode source 与 macOS 回归

- [ ] macOS arm64/x64 继续解析官方 locale-neutral DMG stable alias。
- [ ] Windows x64 精确解析 `https://opencode.ai/download/stable/windows-x64-nsis` 为 `PackageFormat::Exe`。
- [ ] Windows ARM64/x86/Linux 明确 fail-closed。
- [ ] 模拟 GitHub latest 失败时，macOS 与 Windows stable source 仍可安装。
- [ ] macOS 既有 DMG install/update/launch 与 `ai.opencode.desktop` 无回归。

### C. OpenCode Windows x64

- [ ] 当期官方 EXE 的签名/产品/架构/scope 已在 Windows HIL 冻结或明确 fail-closed 未宣称支持。
- [ ] 复用现有 download → verify → PackageBridge → Alice helper → `open` handoff。
- [ ] helper 只新增闭集 `opencode` product，不重排旧值，不复用 Grok wire。
- [ ] job 成功文案是安装器已打开，不是产品已安装。
- [ ] Windows update 仅在 same-target HIL 后开放，否则不可用。

### D. 文档、SPEC、自动化

- [ ] SPEC / 用户手册与代码一致。
- [ ] Rust/TS 聚焦测试与仓库门禁通过。
- [ ] 未完成的 HIL 以残余风险记录，不写成已支持。

## 7. 完成定义

代码、SPEC、用户手册与测试一致；macOS/Windows 默认 Grok 一键安装走官方 npm + 国内镜像 + 精确版本清单；原生安装仅显式可达；OpenCode Windows x64 复用既有签名 EXE handoff；没有夸大陆登录或 ARM64 支持；归档前已按 Trellis 更新 SPEC。
