# 2026-09-30 实时来源核对矩阵（12 个表面）

采集时间：2026-09-30 00:05–00:35（UTC+8），Linux x64 开发主机，仅读元数据 / HEAD /
下载后离线检查安装包，**未在 Windows 或 macOS 真机执行安装**。原始输出由
`evidence/source_check.py` 生成（不入库）。

| 表面 | 生产代码使用的来源 | 实测结果 | 结论 |
| --- | --- | --- | --- |
| Codex CLI | npm `@openai/codex` | npmjs/腾讯/华为/npmmirror latest 均为 0.159.0，integrity 一致；GitHub `rust-v0.159.0` 为非预发布；0.160.0-alpha 只在 alpha tag | 正常 |
| Codex Desktop | `codexapp.agentsmirror.com/latest/*` | manifest 200（schema 5，Windows 26.924.2738.0）；win-x64/win-arm64/mac-arm64 302 → `codexapp-r2.agentsmirror.com` 200 | 正常（由 Codex Desktop 安装器负责） |
| Claude Code | npm `@anthropic-ai/claude-code` + 审核镜像 | latest=next=2.1.284，stable 2.1.277；4 个 registry 根包和 4 个平台包 integrity 一致 | 正常；PR #198 的 Homebrew prefix 归属修复已并入 |
| Grok Build | npm `@xai-official/grok` | npmjs/腾讯/华为 1.0.44；**npmmirror `latest` 指向 npmjs 已下架的 0.1.4，且缺 1.0.44 根包** | 修复：镜像选出的版本标为 `mirror_fallback`，界面与安装确认不再称其为官方最新 |
| OpenCode CLI | npm `opencode-ai`，回退 GitHub `anomalyco/opencode` | `opencode-ai` 1.18.33；官方 v2 包 `@opencode/cli` 2.0.19（trusted publisher，同仓库）；GitHub latest v1.18.33 | 修复：本地 ≥2.x 时查 `@opencode/cli`，不回退 GitHub |
| OpenCode Desktop | `opencode.ai/download/stable/*`，显示版本取 GitHub latest | 稳定别名已 302 到 `opencode.ai/files/bin/2.0.19/...`；DMG bundle id 仍为 `ai.opencode.desktop`；NSIS 签名者 Anomaly Innovations、ProductName OpenCode、FileVersion 2.0.19、包名 `@opencode/desktop` | 修复：不再把 GitHub 1.18.33 当显示/期望版本（否则 macOS DMG 校验必然 SourceInvalid）；新增 Windows 路径 `@opencodedesktop\OpenCode.exe`（未在 Windows 实测） |
| Gemini CLI | npm `@google/gemini-cli` | latest 0.61.0，4 源一致；GitHub v0.61.0 | 正常 |
| OpenClaw | npm `openclaw` | latest 2026.9.6，4 源一致；GitHub v2026.9.6 | 正常 |
| Hermes | PyPI `hermes-agent` | PyPI 0.19.0（2026-07-20，滞后）；GitHub latest `v2026.9.24`（标题 Hermes Agent v0.21.5），该 tag `pyproject.toml` `[project] version = "0.21.5"`；官方安装文档不含 PyPI | 修复：改为官方稳定 tag 下的 pyproject 版本，失败为未知，不回退 PyPI |
| QoderWork CN | `static.qoder.com.cn` latest.yml / latest-mac.yml + 别名 | 0.9.18；三个别名 200 | 正常 |
| TRAE Work CN | `api.trae.cn` `data.solo` cn | 2.3.87413 / Work 0.1.69，Windows x64、darwin arm64/x64 dmg | 正常 |
| WorkBuddy | `www.workbuddy.cn/v2/update` | 三个平台 5.6.2.39298511；mac `.zip`→`.dmg` HEAD 200 | 正常 |

## 已知环境限制

- 主 crate `fyagent` 在 Linux 开发主机上 `cargo check` 本就失败（main 基线
  1a4a1cfa：lib 137 个错误、lib test 156 个），与本次改动无关，改动前后错误数一致。
  这不符合 development-environment 契约中“Linux 可作开发主机”的表述，需另开任务处理。
  主 crate 的 Rust 编译/clippy/测试以 CI 的 Windows、macOS runner 为准。
- `fyagent-user-helper` 在 Linux 上同样编译不过（`current_platform_package` 等只有
  Windows/macOS 实现）。曾用临时 `cfg(not(any(windows, macos)))` 分支让它在本机编译并跑通
  83 个测试，但仓库 supported-platform 治理禁止未登记的隐式目标 cfg，因此该临时改动未提交。
- 本机额外用 `x86_64-pc-windows-gnu` 交叉目标跑了
  `cargo clippy -p fyagent --lib --tests -- -D warnings`，通过（非 MSVC，仅作编译层面佐证）。
  macOS 目标缺 Apple SDK，无法本地交叉检查。
- 本地 `check:frontend` 中 `miseTaskContract`（Linux 不是受支持测试宿主）与
  `repositoryGovernanceScan` history 用例在未改动的基线上同样失败，属环境问题。
