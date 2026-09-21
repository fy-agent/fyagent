# Issue #27：CLI 安装前磁盘空间证据

核查日期：2026-09-21；来源仅官方产品文档、官方 npm registry 元数据和当前仓库代码。

## 当前实现边界

- `src-tauri/src/agent_install/cli.rs` 只把 Claude Code、Grok Build、OpenCode 映射到 CLI tooling；Codex 映射为 `None`，其 CLI 生命周期保持只读/禁用。
- 实际 CLI lifecycle preflight 在 `services/tooling/install_preflight.rs`：macOS 只接受 Claude/Grok；Windows 通过普通用户 helper 只接受 Claude/Grok。OpenCode 的 CLI 映射不等于本安装路径可执行。
- `agent_install/preflight.rs:177-182` 的 CLI 分支没有 release URL/size metadata；`storage_budget(Cli,None)` 在 `:80-89` 返回 `cli_unknown`、`requiredBytes=null`、`artifactSizeBytes=null`。
- `:363-415` 仍检查临时卷和目标卷，但 `requiredBytes=null` 时只拒绝 0 bytes；因此当前不足以证明 CLI 包/依赖所需空间，不能只把 `available>0` 当“容量充足”。

## 官方证据

| CLI | 官方安装/要求 | 可绑定版本的官方 npm metadata（本次查询快照） | 能否当完整安装预算 |
|---|---|---|---|
| Claude Code | Anthropic 要求 Node.js 18+、4GB+ RAM，并给出 `npm install -g @anthropic-ai/claude-code`；未给磁盘最低值：[官方安装页](https://docs.anthropic.com/en/docs/claude-code/getting-started) | `@anthropic-ai/claude-code@2.1.278` 根包 `unpackedSize=184,119`；darwin-arm64 optional 包 `unpackedSize=217,695,985`，另有平台包与可选依赖：[registry metadata](https://registry.npmjs.org/@anthropic-ai/claude-code/2.1.278) | 平台包尺寸可作版本/平台绑定的输入，但根包、缓存、npm 临时目录、依赖和脚本开销仍未形成厂商“最低磁盘”保证。 |
| Codex CLI | OpenAI 官方 README 给出 npm、独立安装器和平台包；官方安装/构建页列 macOS 12+、4GB RAM minimum，未给磁盘最低值：[README](https://github.com/openai/codex/blob/main/README.md)、[install.md](https://github.com/openai/codex/blob/main/docs/install.md) | `@openai/codex@0.155.1` 根包 `unpackedSize=13,206`，通过 optional dependency 别名绑定 `0.155.1-darwin-arm64`；该版本平台 tarball 的 `unpackedSize=317,014,114`：[registry metadata](https://registry.npmjs.org/@openai/codex/0.155.1-darwin-arm64) | 可作为精确版本平台包的下界/输入；不能宣称完整 npm 安装预算，且本仓库当前没有 Codex CLI lifecycle。 |
| Grok Build | xAI 官方安装页只给 `curl ... install.sh`；企业文档明确 npm alternative `npm install -g @xai-official/grok`，未给磁盘最低值：[安装页](https://docs.x.ai/build/overview)、[企业部署](https://docs.x.ai/build/enterprise) | `@xai-official/grok@1.0.34` 根包 `unpackedSize=18,363`；darwin-arm64 optional 包 `unpackedSize=41,790,726`：[registry metadata](https://registry.npmjs.org/@xai-official/grok/1.0.34) | 平台包 metadata 可绑定 exact version/platform；仍缺依赖闭包、缓存、临时空间和官方最低磁盘保证。 |
| OpenCode | 官方文档支持 `npm install -g opencode-ai`，说明 npm postinstall 会选择平台 native binary；未给磁盘最低值：[官方安装页](https://opencode.ai/en/docs)、[v2 CLI 文档](https://opencode.ai/v2/docs) | `opencode-ai@1.18.31` 根包 `unpackedSize=7,865`；darwin-arm64 optional `opencode-darwin-arm64@1.18.31` `unpackedSize=144,470,642`：[registry metadata](https://registry.npmjs.org/opencode-darwin-arm64/1.18.31) | 同上；此外当前产品生命周期策略默认 OpenCode 为 Desktop，不能把这条 CLI metadata 直接用于现有 CLI preflight。 |

npm 官方文档确认 `npm view` 可读取指定版本的嵌套 registry metadata，[npm view](https://docs.npmjs.com/cli/v11/commands/npm-view/)；registry 以 package/version 解析 tarball，[registry](https://docs.npmjs.com/misc/registry/)。`dist.unpackedSize` 是已发布包的解包尺寸，不是 npm 全局安装的完整磁盘峰值；根包的 optional platform package 也不会被根包尺寸涵盖。

## 可实现方案与缺口

1. **采用 exact npm manifest + 当前平台 package metadata，预算 `3 ×`。** 对 Claude/Grok/OpenCode/Codex 的 exact version，读取官方 packument 的当前平台 optional package，绑定 version、platform、integrity、`unpackedSize`；至少把 root、platform package、已解析 dependency closure 和 npm cache/staging 的可证明尺寸纳入预算。优点是去掉 `available>0`；缺口是 registry metadata 不提供一次 npm lifecycle 的峰值/全局 cache 清理行为，也没有厂商最低磁盘承诺，故只能标注“产品保守预留预算”，不能写成厂商要求。
2. **只用根包 `unpackedSize` 或当前 2 GiB downloader cap。** 不采用：四个 CLI 都有 platform optional/native artifact，根包数值明显不覆盖它们；桌面下载上限也不能证明 CLI 依赖闭包。
3. **没有可信 exact metadata 时 fail closed。** 保持 `cli_unknown`，返回 null required/size，并明确“未完成容量证明”；不能把非零 free space 当成功，也不能虚构包体大小。可同时展示 metadata 缺失原因，等待刷新/重新解析。
4. **直接使用某次查询的四个当前数值作为固定常量。** 不采用：版本会变化；只能把查询结果作为带版本、平台、时间的证据样本或测试 fixture，不能作为厂商保证或永久预算。

结论：Issue #27 的最小诚实修复是引入 exact-version/platform metadata 预算路径，并保留 metadata 缺失时 `cli_unknown` fail-closed；研究证据支持“产品预留预算”，不支持任何厂商最低磁盘容量声明。
