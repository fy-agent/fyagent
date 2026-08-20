# 当前软件一键安装与 Codex Desktop 校验链地图

## 目的

记录本任务规划时从当前代码确认的验证链、维护耦合点和必须保留的操作安全边界。源码与测试仍是实现权威；行号用于快速定位，后续代码变化时应重新搜索。

## 当前安装入口

- `src/components/settings/AboutSection.tsx`：Claude Code、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes 的安装/升级入口，委托 npm、工具自身命令或官方脚本；完成后调用 `get_tool_versions` 重新探测。
- `src-tauri/src/commands/misc.rs`：`run_tool_lifecycle_action` 及平台/安装来源命令规划。现有 CLI 工具链没有 FyAgent 下载包 hash/identity 准入。
- `src-tauri/src/services/codex_desktop/mod.rs:904`：Codex Desktop 自建下载、平台验证、安装和安装后验证的主流程，是本任务主要修改面。

## Release source 与内容证明

- `src-tauri/src/codex_desktop/source.rs:41-66`：metadata endpoint 同时包含 Checksums 与 Manifest；artifact URL 仍由固定 endpoint enum 控制。
- `src-tauri/src/codex_desktop/source.rs:182-193`：当前先取 checksums，再取 manifest，并执行 `validate_release_metadata`。
- `src-tauri/src/codex_desktop/source.rs:408-428`：`ValidatedRelease` 携带 expected SHA/size/minimum OS；失败枚举包含 manifest/artifact checksum mismatch。
- `src-tauri/src/codex_desktop/source.rs:496-509`：checksums、manifest SHA 和平台 artifact 字段交叉验证入口。
- `src-tauri/src/codex_desktop/types.rs:262-339`：`ReleaseDescriptor` 把 expected SHA/size 作为构造与 release ID 事实。

## 下载器与状态机

- `src-tauri/src/codex_desktop/download.rs:296-360`：`DownloadedArtifact` 通过 release expected kind/size/hash 重新验证文件。
- `src-tauri/src/codex_desktop/download.rs:590-701`：响应长度、累计字节、最终字节数和 SHA 必须与 release metadata 相等，并发布 download verification progress。
- `src-tauri/src/services/codex_desktop/mod.rs:992-1045`：checksum mismatch 触发 metadata re-anchor；随后进入 `VerifyingDownload` 并调用平台 `verify_package`。
- `src-tauri/src/services/codex_desktop/mod.rs:1055-1079`：安装后 verification 是用户结果验证，必须保留，但固定 identity/platform/version 相等判断需改为动态安装结果语义。
- `src-tauri/src/codex_desktop/jobs.rs`、`src/shared/codex-desktop/types.ts`、`tests/fixtures/codexDesktopDtoContract.v1.json`：`verification` 同时承担下载校验与安装后验证，需要删除前者而保留后者。

## Windows 上游耦合与操作安全

- `src-tauri/src/codex_desktop/platform/windows/mod.rs:129`：硬编码官方 Publisher。
- `src-tauri/src/codex_desktop/platform/windows/mod.rs:660-704`：下载后验证包并与 release 绑定。
- `src-tauri/src/codex_desktop/platform/windows/mod.rs:876-928`：固定 Name、Publisher、Architecture、Version、MinVersion 比较；本次用户错误来自这些分支之一。
- `src-tauri/src/codex_desktop/platform/windows/manifest.rs`：有界 ZIP/MSIX 结构、签名文件存在、XML manifest 解析；生产准入用途应删除，若保留最小读取只能作为操作 locator。
- `src-tauri/src/codex_desktop/platform/windows/helper.rs:126-224`：pin 当前从 release expected size/SHA 建立，需改为实际下载结果的动态值。
- `src-tauri/src/codex_desktop/platform/windows/package_bridge.rs:204-422`：PackageBridge 复制与 sealed object 反复核对 size/hash；机制保留，但输入改为动态本地指纹。
- `src-tauri/user-helper/src/windows.rs`：固定 helper、authenticated protocol、protected file URI 和 `AddPackageByUriAsync` 是原生安装边界；默认系统检查不绕过。
- `tests/codexUserHelperContract.test.ts`、`tests/codexWindowsUserScopeContract.test.ts`：固定 CLI、Shell SID、ACL/no-follow/file-ID、AddPackage-only 和无任意路径/URL能力必须继续成立。

## macOS 上游耦合与操作安全

- `src-tauri/src/codex_desktop/platform/macos/dmg.rs`：DMG verify/attach、唯一应用发现、版本/identity 比较、目标替换与回滚混合在同一模块；删除内容准入但保留受控 attach、定位、替换、回滚和 detach。
- `src-tauri/src/codex_desktop/platform/macos/bundle.rs:514-641`：固定 Bundle ID、codesign、Team ID、spctl 准入。
- `src-tauri/src/codex_desktop/platform/macos/bundle.rs:700-1064`：local/runtime/launch 也依赖固定 bundle identity；当前 job 的安装后结果需改为实际路径与动态 identity，运行时仍必须绑定具体安装对象。
- 当前 Windows 主机不能提供真实 Apple Silicon/macOS HIL，不能把 fake fixture 结果表述为原生成功。

## 不受影响的校验

- `src-tauri/src/services/skill.rs`：Skills 下载、ZIP、路径与写入边界；不属于可执行软件安装包策略。
- `src-tauri/src/services/sync_protocol.rs`：同步快照 hash/size；不属于一键软件安装。
- release/CI、NSIS、数据库和配置导入相关验证均不在任务写集。

## 实现停止条件

- 任何方案要求普通 renderer 传 URL/path/hash/scope/bypass 或任意 helper 命令时停止。
- 任何方案削弱 Shell SID、ACL、no-follow、file-ID、动态本地摘要、macOS 目标路径/回滚或安装后结果探测时停止。
- 若 Windows 动态安装结果发现必须新增持久化 schema，返回规划阶段评审，不在实现中顺带扩张。
