# 原始证据与选择

## 2026-09-05 已执行的证据获取

通过已授权 `gh api` 的分页端点读取 `repos/fy-agent/fyagent/{dependabot,code-scanning,secret-scanning}/alerts?state=open`，秘密列表仅投影类型、位置、状态，未保存 secret 字段。

- GitHub API：https://docs.github.com/en/rest/dependabot/alerts 、https://docs.github.com/rest/code-scanning/code-scanning 、https://docs.github.com/en/rest/secret-scanning/secret-scanning 。默认分支告警不等于本地 HEAD 结果；不把权限不足/接口失败当作零告警。
- Tauri：https://github.com/tauri-apps/tauri/security/advisories/GHSA-7gmj-67g7-phm9 。2.11.1 修复 Windows 自定义协议远端域被误认为 local 的问题。保留本地 ACL 仍需升级框架，不能自行添加第二套 IPC 安全实现。
- Vitest：https://github.com/vitest-dev/vitest/security/advisories/GHSA-5xrq-8626-4rwp 。上游网页与 audit 修复界限有差异（3.2.5 / 3.2.6），采用 registry 已存在的 3.2.7 并重新 audit，不选择最低且仍被公告命中的版本。
- pnpm audit：https://pnpm.io/10.x/cli/audit 。本地 `mise exec -- pnpm audit --json` 保持默认全严重性，不加 ignore-unfixable/ignore-registry-errors。初始输出 `/tmp/fyagent-round2-npm-before.json`。
- RustSec：https://github.com/rustsec/rustsec/tree/main/cargo-audit 。临时安装 cargo-audit 0.22.2（`--locked --root /tmp/fyagent-round2-tools`），不改用户工具版本。初始 `cargo-audit audit --file src-tauri/Cargo.lock --json`；数据库 commit `5a0ebedfe8bdd2e295b171f4162f8c977bcad9a5`，1239 advisories。初始报告 `/tmp/fyagent-round2-rust-before.json`。
- Gitleaks：https://github.com/gitleaks/gitleaks 。v8.30.1 官方 darwin_arm64 artifact，经同 release SHA-256 校验后执行。`dir` 扫描 HEAD 的 git archive；`git --log-opts=--all` 扫描本地全部 refs。均使用 `--redact=100`，不提交原始报告或线上验证凭证。
- dependency-cruiser：https://github.com/sverweij/dependency-cruiser 、https://github.com/sverweij/dependency-cruiser/blob/main/doc/rules-reference.md 。MIT，18.2.0 与当前 Node 24 匹配。临时 dlx 未找到 TS compiler 的结果已废弃；必须与项目 TypeScript 共存并确认覆盖，禁止拿 66 JS modules 结果冒充 TS 扫描。

## 选型边界

parse5 复用已存在锁图节点，负责 HTML 标准语法；业务信任限制仍由构建器负责。dependency-cruiser 仅新增开发依赖，不进入产品 runtime。所有直接新增依赖仍需执行 lock/许可证/审计及构建测试。

## 初始告警分类

最终逐组处置、独立复扫结果及保留风险见 [告警处置记录](./alert-disposition.md)，验收结果见 [集成评审](../review.md)。以下保留的是初始调查时的候选分类，不能替代最终记录。

- 远端 51 个 Dependabot 告警涉及同一依赖多个 advisory；独立 npm/Rust 扫描发现额外问题，必须同时治理。
- CodeQL 66 项：HTML/URL/动态 DOM、结构 merge、scanner regexp、日志、Windows 指针/RNG、测试常量、Swift 依赖及 Release 权限上下文。逐组审查源/汇/用途，不直接依靠 severity 作可利用性判决。
- Secret scanning 2 项定位历史 `subscription.rs` 的 Gemini OAuth client 常量。必须与官方上游公开 desktop client 对照；client ID 不等于用户 token，不能擅自撤销第三方 client。
- Gitleaks 当前 17 个候选集中于 deeplink 示例和测试文件。匹配规则不等于有效私人 secret；将逐个核对 test/example 上下文，不通过全目录 allowlist 隐藏。
