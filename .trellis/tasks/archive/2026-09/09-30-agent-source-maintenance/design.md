# 设计与兼容性

## 1. 基线与职责

冻结基线 main = `1a4a1cfa2790528a989995fac50469061a4d78ae`。
`versions.rs` Git blob = `173e7836410ebeadd55f710c6e3ead57b38e37b0`，交付包里的 baseline 已逐字校验。
代码来源、平台身份和网络准入继续由现有 `external-agent-sources.md` 与对应源模块拥有。
不在 renderer 暴露 URL、registry、hash、路径或任意命令。

## 2. Hermes 候选补丁

官方发布使用日期标签，例如 `v2026.9.24`；该标签下 `pyproject.toml` 的 `[project].version` 为 `0.21.5`。
候选实现固定查询官方 `NousResearch/hermes-agent` 最新正式 Release，验证 draft/prerelease、tag 形状及官方页面身份，随后读取该 tag 的 `pyproject.toml`。
只解析 `[project]`，要求 `name = hermes-agent` 和稳定 semver；忽略 release 标题、正文和任意 URL。
网络沿用现有 installer proxy adapter，新建不自动跳转、只允许 HTTPS 的无默认凭据 client；每个响应最多 1 MiB，每次请求超时 20 秒。
主线、自由文本及日期 tag 均不能兜底成产品版本；获取失败返回未知，不能冒充 PyPI 旧号为官方 latest。

当前草案只返回“官方产品 stable”的版本；尚未实现本机渠道归属识别或渠道 UI 标签。
合并前必须审查此语义与现有 ToolVersion/latest_source 消费方是否一致。
如果产品需要“本安装渠道最新”语义，则先完成可信 owner 识别：PyPI 安装使用 PyPI，官方 Git 安装使用官方 release；owner 未知不能按 PATH 文本猜测。
此候选补丁不建立或声称拥有 Hermes 一键安装能力。

## 3. OpenCode 渠道维护

官网下载页已展示 `@opencode/cli` 与 v2 安装入口；这只能证明渠道出现，不能证明所有旧 stable 安装应迁移。
实际实现需按官方文档和当前上游仓库验证新版的稳定性、包身份、CLI --version 与安装归属。
保留 `opencode-ai` 对应旧安装的查询；新版单独映射；Desktop 稳定别名与版本显示保持独立。
源码 fixture 出现旧版本无需自动替换。

## 4. Claude/Grok 镜像来源

保留官方优先解析，并保持 root + 当前平台 + 已许可依赖的 exact version/integrity 一致性。
不能把多个未经确认的镜像中数值最高的版本自动宣称为“官方最新”。
增加来源/确认状态时优先复用现有 ToolVersion/latest_source 或域 DTO，先查其所有消费者，保持兼容。
当官方不可达时，应明确来源为 mirror_fallback、官方最新性未知；如果已有可信官方基准，可显示落后差异，但不能静默降级。
同一具体版本的主包/平台包 hash 不一致应跳过来源或终止，不能降级校验来换取可安装。
`#198` ownership 修复需要重新审查当前状态和精确 diff，仅复用关联内容；不得把 PR 中其他会话修改自动纳入。

## 5. 其他产品

沿用现有模块的 feed、alias、schema、产品和平台矩阵。联网检测从生产常量提取，不另维护一套会漂移的 endpoint 清单。
官方与 CDN 对照包含 metadata 解析、受限 GET、完整包摘要和平台原生身份验收，各阶段分别报告。

## 6. 发布与回退

主线当前源码版本为 0.4.7；下一补丁暂按 0.4.8 规划，正式执行前重查 main、remote tags 与 Release，使用仓库版本工具。
独立维护分支提交 PR，通过所有必需检查和合并治理流程。随后对最终确定的提交执行正常 Release 流程。
正式发布由稳定 tag 触发，或在已存在的该稳定 tag 上 dispatch `mode=formal`；preflight 仅是诊断，不能视为正式发布。
不绕开现有 artifact、签名、公证和 attestation 门禁；不移动已有已发布 tag。
回退代码使用正常 revert 提交和后续补丁版，绝不覆盖已发布安装包。
