# 统一 FyAgent 软件一键安装免内容校验策略 — 技术设计

## 1. 设计目标

当前 Codex Desktop 安装链把远端发布 metadata、checksum 文件和安装包内部字段组合成一条准入证明。上游任一字段漂移都会在原生安装器运行前失败，造成 `PACKAGE_IDENTITY_MISMATCH` 等维护型故障。

本设计把职责拆成三层：

1. **来源与流程编排**：选择固定下载入口、平台和架构，读取展示/定位所需 metadata。
2. **操作安全**：可靠下载、受保护落盘、动态文件指纹、跨进程/跨权限同一对象交接。
3. **安装结果**：交给平台原生安装入口，并在完成后探测实际安装结果。

FyAgent 不再建立第四层“下载内容是否符合 FyAgent 期望”的准入证明。

## 2. 边界定义

### 2.1 删除的上游耦合校验

- checksum 文件与 release manifest 的摘要交叉验证；
- 下载结果与远端 SHA-256、远端 content length 或 FyAgent 内置摘要/大小的相等判断；
- Windows MSIX 的固定 Name、Publisher、Version、MinVersion、Architecture、签名文件和包结构 allowlist；
- macOS DMG/应用的固定 Bundle ID、Team ID、版本、架构、最低系统版本、`codesign` 和 `spctl` 准入；
- 平台准备结果与 release descriptor 的身份/摘要二次比对；
- 仅为上述校验存在的下载验证状态、错误码、文案、fixture 和测试。

### 2.2 保留的操作安全

- 固定、后端选择的下载 endpoint；普通 IPC 仍不能传入 URL、路径、hash、scope 或自定义命令；
- HTTPS/HTTP 状态、手动重定向策略、代理自环保护、超时、重试和取消；
- metadata 响应大小上限和严格 JSON 类型解析，防止资源耗尽或流程字段缺失；
- 平台/架构 endpoint 选择，以及必要字段缺失时的流程错误；
- 磁盘空间提示、短写/截断检测、flush/sync、原子 finalize 和清理；
- Windows 固定 helper、认证管道、Shell SID、ProgramData PackageBridge、ACL、no-follow、文件 ID/link/reparse/placeholder 检查；
- macOS 受控挂载、目标目录边界、原子替换/回滚和运行中应用处理；
- 实际下载字节生成的本地动态大小/摘要，以及其在安全交接中的一致性检查；
- 原生安装器/包管理器默认行为和安装后目标探测。

### 2.3 明确不受影响

Skills、插件、MCP、配置包、同步快照、数据库内容、FyAgent 自身 release/CI 资产等不属于软件安装包策略。其格式、解压、路径、来源、hash 和写入边界保持不变。

## 3. 数据模型

### 3.1 ReleaseDescriptor

`ReleaseDescriptor` 从“已验证内容描述”改成“安装流程描述”，仅保留：

- platform / architecture；
- display/platform version，用于展示和版本决策；
- 固定 `TrustedDownloadEndpoint`；
- 可选下载大小提示，只用于进度和磁盘估算，不参与最终准入；
- 如平台安装/后续发现需要，可携带远端提供的操作性 locator；该 locator 不与下载包内容比较。

删除 `expected_sha256` 语义。`expected_size` 如继续出现在内部模型中，必须重命名为可选 hint，并且任何差异都不能终止安装。release ID 不再包含 hash、内容大小或下载包身份字段；它只绑定用户点击时看到的平台、架构、版本和 endpoint，以保留防陈旧点击语义。

### 3.2 DownloadedArtifact

`DownloadedArtifact` 持有：

- 受保护 job 目录内的文件 capability/路径；
- 实际下载字节数；
- 下载时根据实际字节计算的本地动态 SHA-256；
- artifact kind，仅用于选择 `.msix` / `.dmg` 的平台处理分支。

动态 SHA-256 不与任何远端值比较。它只在重新打开、复制到 PackageBridge 或交给 helper 前后证明文件没有被替换。

### 3.3 平台安装对象

将语义为“已验证内容”的 `VerifiedPackage` / `verify_package` 边界改为“已安全准备、可交给平台安装”的对象和方法，例如 `PreparedInstallPackage` / `prepare_install_package`。命名必须避免继续声称 FyAgent 已验证发布者或内容。

该对象可携带从安装包或 metadata 读取的操作性 identity/locator，供当前 job 的安装后查询和启动使用，但不能含固定 allowlist 判定结果。

## 4. 数据流

```text
固定 metadata endpoint
  -> 有界解析可用版本/平台/架构/下载入口
  -> 固定 artifact endpoint
  -> 受保护下载 + 实际大小/动态摘要
  -> 平台必要准备（不做上游身份准入）
  -> 原生安装/包管理器
  -> 安装后发现、版本/可运行性探测
  -> 成功或真实失败
```

删除 `Downloading -> VerifyingDownload -> Installing` 中的内容验证 gate。状态机改成下载安全完成后进入平台准备/安装；`Verification` 语义只保留给安装后结果探测。若需要可见的短暂准备状态，使用不声称内容可信的 `preparing`/`installing` 表达，而不是继续复用“校验下载包”。

## 5. Release source 与下载器

### 5.1 Metadata

- `AgentsMirrorSource` 不再先下载 checksums 并交叉验证 manifest；只获取固定 manifest endpoint。
- manifest parser 只要求当前平台流程必需字段：目标可用、版本可解析、固定 endpoint 可选中。远端 URL/delta 继续忽略，防止 metadata 变成下载能力。
- `sha256`、team/publisher/bundle identity、content length 等字段可以忽略或作为非准入提示；缺失或改变不能产生内容校验错误。
- metadata body 上限、UTF-8/JSON 解析和重试/取消保留。

### 5.2 Artifact download

- Content-Length 或 metadata size 仅作为可选进度总量和空间估算；实际字节不同不失败。
- 保留流式写入、最大安全容量/磁盘错误、取消、flush/sync、受控 rename 和重新打开。
- 下载过程中计算实际大小和动态摘要；finalize 后重新打开同一受保护对象并核对动态摘要，确认本地交接未漂移。
- checksum mismatch 与 metadata re-anchor 特殊重试分支删除。

## 6. Windows 平台

- 父进程不再把 MSIX manifest 当成准入证明；删除固定 Stable Name/Publisher/Version/MinVersion/Architecture 比较和结构/签名存在性检查。
- PackageBridge 继续使用固定目录、严格 ACL、held handles、no-follow、文件 ID/link/reparse/placeholder 检查。它改用 `DownloadedArtifact` 的实际大小和动态摘要，而不是 release metadata 的 expected 值。
- helper 协议仍只接受固定 action/job/pipe，不暴露任意路径/URL/命令。helper 继续通过 `PackageManager.AddPackageByUriAsync` 的默认选项安装，系统是 MSIX 格式与签名链的最终 authority。
- 如果安装流程需要 package identity 才能查询结果，identity 由当前受保护包或系统安装结果动态获得，仅用于当前操作定位；不得与 `WINDOWS_CODEX_STABLE_IDENTITY` 或固定 Publisher 比较。
- 安装后查询必须绑定同一 Shell SID，并对动态 locator 指向的实际已安装 Main package 做存在性、版本和 launch target 探测。当前已安装应用的旧 Stable locator 可保留为兼容发现入口，但不能再参与下载包准入。
- `PACKAGE_IDENTITY_MISMATCH` 不得由下载内容或平台准备阶段产生。仍有必要的本地运行时/启动对象漂移错误应使用结果/运行时语义明确的错误码。

## 7. macOS 平台

- DMG 仍需受控 `hdiutil attach`，因为挂载是取得安装目标的必要步骤；不再运行 `hdiutil verify`、`codesign`、`spctl`、Team ID、Bundle ID、版本、架构或最低系统版本准入比较。
- 挂载结果只做操作性发现：找到唯一可复制的顶层 `.app`。没有可安装应用或存在无法唯一选择的多个应用时，返回“无法定位安装目标”，而不是“内容身份不匹配”。
- 目标路径、目录边界、symlink/escape 防护、运行中应用处理、原子替换和失败回滚保留。
- 安装后从实际目标读取动态 bundle/path/version 信息并验证目标存在、可读取、可启动；不与固定 OpenAI Bundle ID/Team ID 比较。
- runtime/restart 继续绑定安装后得到的实际路径和动态 identity，避免模糊进程名操作。

## 8. 前端与 DTO

- 删除只描述下载内容校验的 job stage/state、错误码、错误文案和详情映射；保留安装后 verification。
- remote release DTO 不再承诺 `expectedSize`/checksum 是可信内容事实。若保留大小字段，改为可空的显示/进度 hint，并同步 Rust/TypeScript parser/fixture。
- Legacy 与 V2 必须继续复用共享 installer core；不能出现一套仍展示“校验包”、另一套已移除的分叉。
- 四种 locale 同步更新，不新增未翻译字面量。

## 9. 兼容、迁移与回滚

- 不改变数据库 schema，不迁移 Skills/MCP/配置数据。
- 普通 Tauri installer command 仍只接受 release ID；不存在用户可控 bypass 开关。
- 现有 CLI 工具安装行为保持不变，只增加契约/回归证明。
- 回滚方式是还原本任务代码提交；任务不写永久安装迁移状态。临时 artifact/PackageBridge 仍由现有清理边界回收。
- 若实现发现动态 Windows 结果定位必须新增持久化 schema，必须返回规划阶段重新评审，而不能在实现中顺带增加。

## 10. 验证边界

- 单元/契约测试必须证明身份、Publisher、Team ID、版本、MinVersion、远端 hash/size 改变不会阻止进入平台安装调用。
- 反向测试必须证明 URL/path/command 注入、临时文件替换、ACL/reparse/file-ID 漂移、取消、磁盘/短写和安装后目标缺失仍被阻止。
- Windows 本机可运行真实安装 HIL 时，应验证当前 `PACKAGE_IDENTITY_MISMATCH` 路径消失并观察原生 PackageManager 结果。
- 当前 Windows 主机不能证明 macOS DMG、codesign/Gatekeeper 或 Apple Silicon 真实安装行为；macOS 结论以测试和代码审查为证据并明确剩余风险。
