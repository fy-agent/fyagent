# Contributing to FyAgent

> [中文版本](#贡献指南)

Thank you for your interest in contributing to FyAgent! Please read our [Code of Conduct](./CODE_OF_CONDUCT.md) before participating.

## How to Contribute

There are many ways to contribute:

- **Report bugs** — Found something broken? [Open a bug report](https://github.com/fy-agent/fyagent/issues/new?template=bug_report.yml).
- **Suggest features** — Have an idea? [Submit a feature request](https://github.com/fy-agent/fyagent/issues/new?template=feature_request.yml).
- **Improve docs** — Spot a typo or missing info? [Report a doc issue](https://github.com/fy-agent/fyagent/issues/new?template=doc_issue.yml).
- **Contribute code** — Fix bugs or implement features via pull requests.
- **Translate** — Help us improve English, Simplified Chinese, Traditional Chinese, and Japanese translations.

> **Security vulnerabilities**: Please do NOT use public issues. See our [Security Policy](./SECURITY.md) instead.

## Development Setup

### Prerequisites

- Git and Git LFS where visual assets require it
- [mise](https://mise.jdx.dev/getting-started.html) 2026.8.6 or newer,
  installed globally
- [Tauri 2.0 prerequisites](https://v2.tauri.app/start/prerequisites/)

The repository pins Node.js, pnpm, Rust, uv, and uv-managed Python through their
owned version and lock files. Do not substitute arbitrary system runtimes.
After reviewing the repository configuration, initialize the environment:

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

`mise trust` is a developer security decision and is never run automatically
by a project task. `bootstrap` does not install privileged OS packages, change
Git remotes, refresh locks, tag, or publish.

### Before a Pull Request

```bash
mise run check
```

This is the complete current-host gate. GitHub's stable `CI / Required` check
is the multi-platform merge authority.

Useful focused tasks include:

```bash
mise run typecheck
mise run format:check
mise run test:unit
mise run test:i18n
mise run test:desktop:mock
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
mise run release:check
```

See the generated
[canonical task catalog](docs/fyagent/development/mise-tasks.md). Active
development documentation uses `mise run <task>` rather than direct project
pnpm/Cargo/system-Python commands. GitHub Actions is the deliberate exception:
workflows install their exact tools without trusting or executing repository
mise configuration.

Use the [current development documentation](docs/fyagent/development/README.md)
to find the relevant flow, then inspect the current code, configuration,
tests, and workflows before changing it. Historical design material is useful
for an explicit investigation, but it does not override executable behavior.

### Local Build Boundary

```bash
mise run build:renderer
mise run build:binary
mise run build
mise run build:debug
```

These tasks build only the current host OS and architecture. Formal Windows
x64/ARM64 and macOS Universal Release assets are produced by GitHub Actions.
Local builds do not cross the supported host boundary.

## Repository and remote roles

The canonical source of truth is [`fy-agent/fyagent`](https://github.com/fy-agent/fyagent).
Remote names are local conventions, so verify the repository role instead of
assuming that every checkout has the same configuration:

- A maintainer checkout may use the canonical repository as its writable
  `origin`.
- An external contributor normally uses a personal fork as `origin` and adds
  the canonical FyAgent repository as another fetch source. That source is
  commonly named `upstream`, or `fyagent` when CC Switch maintenance reserves
  `upstream` for its separate contract.
- CC Switch synchronization uses a distinct fetch-only maintenance remote. It
  is neither the canonical FyAgent repository nor a contributor's fork, and it
  must never become a normal push target.

Documentation and project tasks do not create, rename, or rewrite a
contributor's remotes.

## Repository and remote roles

The canonical source of truth is [`fy-agent/fyagent`](https://github.com/fy-agent/fyagent).
Remote names are local conventions, so verify the repository role instead of
assuming that every checkout has the same configuration:

- A maintainer checkout may use the canonical repository as its writable
  `origin`.
- An external contributor normally uses a personal fork as `origin` and adds
  the canonical FyAgent repository as another fetch source. That source is
  commonly named `upstream`, or `fyagent` when CC Switch maintenance reserves
  `upstream` for its separate contract.
- CC Switch synchronization uses a distinct fetch-only maintenance remote. It
  is neither the canonical FyAgent repository nor a contributor's fork, and it
  must never become a normal push target.

Documentation and project tasks do not create, rename, or rewrite a
contributor's remotes.

## Code Style

- **Frontend**: Prettier formatting and strict TypeScript
- **Backend**: rustfmt, locked Cargo checks, Clippy with warnings denied, and tests
- **Tauri 2.0**: Command names must use camelCase
- **Runtime tests**: Node 24 native Fetch with MSW/Tauri fakes; do not restore a
  Fetch polyfill or suppress deprecation warnings
- **User-visible text**: update all four registered locales and preserve
  accessibility roles, keyboard/focus behavior, labels, and error states

Run all checks before submitting:

```bash
mise run check
```

Formatting, visual-baseline updates, icon generation, version application,
dependency updates, and other source-modifying tasks must be invoked explicitly
and reviewed; they do not belong in ordinary read-only checks.

## Pull Request Guidelines

1. **Open an issue first** for new features — PRs for features that are not a good fit may be closed.
2. **Branch from current canonical `main`** — Fetch the canonical source through the remote appropriate to your role, then create a feature branch (e.g., `feat/my-feature` or `fix/issue-123`). Do not commit directly or force-push to `main`.
3. **Keep PRs focused** — Open one focused PR against canonical `main` for one feature or fix. Avoid unrelated changes.
4. **Follow the PR template** — Fill in the summary, related issue, and checklist.
5. **Wait for the exact-head gate** — The stable `CI / Required` check must succeed on the exact PR head; another SHA or an individual job is not a substitute.
6. **Use the repository merge path** — Accepted changes enter `main` through the protected Merge Queue as merge commits. Clean meaningless fixup/checkpoint commits before merge-ready when practical; do not use squash/rebase to erase meaningful ancestry or upstream provenance. `git log --first-parent main` is the normal one-PR/one-boundary mainline view. This policy does not promise source-branch deletion or require a human approval unless live GitHub protection says so.

### PR Checklist

- [ ] `mise run check` passes on the current host
- [ ] Updated i18n files if user-facing text changed
- [ ] Exact tests, platform limitations, risk, and rollback are recorded
- [ ] Durable behavior changes update executable tests and maintained docs
- [ ] Upstream tag/SHA/conflict or Release asset/permission impact is recorded
      when applicable

### Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(provider): add support for new provider
fix(tray): resolve menu not updating after switch
docs(readme): update installation instructions
ci: add format check workflow
chore(deps): update dependencies
```

## AI-Assisted Contributions

We welcome AI-assisted contributions, but **the responsibility stays with you**. AI tools lower the cost of writing code — they do not lower the cost of reviewing it. Maintainers are not obligated to clean up AI-generated output.

By submitting a PR, you agree to the following:

1. **You have read and understood your code.** You must be able to explain any line in your PR. If you cannot, it is not ready for review.
2. **You have tested it yourself.** Every change must be verified locally — not just "it looks right." Do not submit code for platforms or features you cannot test.
3. **PRs must be small and focused.** One issue, one PR. Large, sprawling, multi-topic PRs will be closed.
4. **Open an issue first.** Drive-by PRs with no prior discussion — especially AI-generated ones — may be closed without review.
5. **Maintainers may close without explanation.** PRs that appear to be unreviewed AI output — hallucinated fixes, unnecessary refactors, bulk changes with no context — may be closed at the maintainer's discretion.

**In short**: AI is a tool, not a substitute for understanding. Use it to help you contribute better, not to shift work onto maintainers.

## Planning and durable behavior

For complex cross-layer, security, persistence, or release work, record the
goal, non-goals, affected boundaries, risks, validation, and rollback in the
issue, pull request, or another planning artifact agreed with the maintainers.
The repository does not require a particular task framework for contribution,
build, CI, or release work.

When implementation establishes a durable engineering rule, update the code
or configuration that enforces it, its executable tests, and the maintained
developer documentation that explains the flow. Optional AI-assistance notes
under `.trellis/spec/` may be refreshed when useful, but they are neither a
contributor prerequisite nor a substitute for executable evidence. Never
rewrite archived tasks or prior workspace-journal entries. Record a correction
in new current material or an appended entry.

## Upstream CC Switch changes

CC Switch synchronization uses its own fetch-only maintenance remote
(`upstream` in the maintained `upstream:*` tasks). That remote is separate from
both the canonical FyAgent source and a contributor fork. The checkout's normal
push target remains `origin`: canonical for a maintainer checkout or the
personal fork for an external contributor. Formal upstream tag work verifies
the immutable tag object and peeled commit, preserves a two-parent merge commit
and MIT ancestry, and stops before any automatic commit or push. A PR that
contains upstream work must record the tag, full SHAs, conflict decisions,
FyAgent-specific contracts preserved, tests, and rollback boundary.

## Evidence and release changes

A PR must distinguish local/static checks from real native-runner or published
Release evidence. Changes to workflows, permissions, installers, asset names,
attestations, or publication logic must run `mise run release:check` and name
the remaining remote gates. Do not claim a signed/notarized artifact, a native
platform result, a protected branch/environment, or a published Release from a
local check alone.

## Licensing and contribution rights

By submitting a contribution, you represent that you have the right to
authorize your own contribution under this repository's licensing model. Do
not submit code, assets, or data under incompatible terms. Identify the source
and license of any third-party material included with a contribution.

FyAgent plans to offer commercial licensing for FyAgent-owned code. Until a
legally reviewed and deployed contributor license agreement or explicit
relicensing process exists, a pull request does not automatically transfer
copyright. Maintainers should not merge substantial external contributions
that would affect commercial licensing capacity until that process exists.

These requirements do not remove or alter the attribution or MIT licensing of
CC Switch-derived portions, including the original attribution to Jason Young.

## Internationalization (i18n)

FyAgent maintains four locale resources. When modifying user-facing text:

1. Update **all four** locale files:
   - `src/i18n/locales/en.json`
   - `src/i18n/locales/ja.json`
   - `src/i18n/locales/zh.json`
   - `src/i18n/locales/zh-TW.json`
2. Use the `t()` function from i18next for all UI text.
3. Never hardcode user-facing strings.

## Questions?

- [Open a question](https://github.com/fy-agent/fyagent/issues/new?template=question.yml)
- [GitHub Discussions](https://github.com/fy-agent/fyagent/discussions)

---

# 贡献指南

> [English Version](#contributing-to-fyagent)

感谢你对 FyAgent 的贡献兴趣！参与之前请阅读我们的[行为准则](./CODE_OF_CONDUCT.md)。

## 如何贡献

你可以通过多种方式参与贡献：

- **报告 Bug** — 发现问题？[提交 Bug 报告](https://github.com/fy-agent/fyagent/issues/new?template=bug_report.yml)。
- **建议功能** — 有想法？[提交功能请求](https://github.com/fy-agent/fyagent/issues/new?template=feature_request.yml)。
- **改进文档** — 发现错误或缺失？[报告文档问题](https://github.com/fy-agent/fyagent/issues/new?template=doc_issue.yml)。
- **贡献代码** — 通过 Pull Request 修复 Bug 或实现新功能。
- **翻译** — 帮助改进英文、简体中文、繁体中文和日文翻译。

> **安全漏洞**：请不要使用公开 Issue 报告。请参阅我们的[安全策略](./SECURITY.md)。

## 开发环境搭建

### 前提条件

- Git；视觉资产需要时安装 Git LFS
- 全局安装 [mise](https://mise.jdx.dev/getting-started.html) 2026.8.6 或更高版本
- [Tauri 2.0 开发环境](https://v2.tauri.app/start/prerequisites/)

仓库通过各自的版本与 lock 文件固定 Node.js、pnpm、Rust、uv 和由 uv 管理的 Python；
不要替换为任意系统运行时。检查仓库配置后初始化环境：

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

`mise trust` 是开发者自己的安全决策，项目任务绝不会自动执行它。`bootstrap` 不安装
提权系统包、不修改 Git remote、不刷新 lock、不打 tag，也不发布。

### 提交 Pull Request 前

```bash
mise run check
```

这是当前宿主的完整门禁；GitHub 上名称稳定的 `CI / Required` 是多平台合并权威。

常用聚焦任务：

```bash
mise run typecheck
mise run format:check
mise run test:unit
mise run test:i18n
mise run test:desktop:mock
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
mise run release:check
```

完整 API 见生成的
[canonical task catalog](docs/fyagent/development/mise-tasks.md)。活动开发文档使用
`mise run <task>`，不使用直接项目 pnpm/Cargo/系统 Python 命令。GitHub Actions 是
明确例外：workflow 安装精确工具，但不会信任或执行仓库 mise 配置。

每项变更先通过[当前开发文档](docs/fyagent/development/README.md)定位相关流程，再检查
当前代码、配置、测试和 workflow。历史设计材料可用于明确的溯源调查，但不能覆盖可执行
行为。

### 本地构建边界

```bash
mise run build:renderer
mise run build:binary
mise run build
mise run build:debug
```

这些任务只构建当前宿主系统和架构。正式 Windows x64/ARM64 和 macOS Universal
Release 资产由 GitHub Actions 生成。本地构建不会跨越受支持的宿主边界。

## 仓库与 remote 角色

唯一规范来源是 [`fy-agent/fyagent`](https://github.com/fy-agent/fyagent)。remote 名称是
本地约定，操作前应核对仓库角色，不能假设每个 checkout 都采用相同配置：

- 维护者 checkout 可以把规范仓库作为可写的 `origin`。
- 外部贡献者通常把个人 fork 作为 `origin`，并把 FyAgent 规范仓库添加为额外 fetch
  来源。该来源通常可命名为 `upstream`；需要执行 CC Switch 维护合同时，则用 `fyagent`
  等名称，把 `upstream` 留给独立的上游合同。
- CC Switch 同步使用单独的只读 fetch 维护 remote。它既不是 FyAgent 规范仓库，也不是
  贡献者 fork，且不得成为常规 push 目标。

文档和项目任务不会创建、重命名或改写贡献者的 remote。

## 仓库与 remote 角色

唯一规范来源是 [`fy-agent/fyagent`](https://github.com/fy-agent/fyagent)。remote 名称是
本地约定，操作前应核对仓库角色，不能假设每个 checkout 都采用相同配置：

- 维护者 checkout 可以把规范仓库作为可写的 `origin`。
- 外部贡献者通常把个人 fork 作为 `origin`，并把 FyAgent 规范仓库添加为额外 fetch
  来源。该来源通常可命名为 `upstream`；需要执行 CC Switch 维护合同时，则用 `fyagent`
  等名称，把 `upstream` 留给独立的上游合同。
- CC Switch 同步使用单独的只读 fetch 维护 remote。它既不是 FyAgent 规范仓库，也不是
  贡献者 fork，且不得成为常规 push 目标。

文档和项目任务不会创建、重命名或改写贡献者的 remote。

## 代码规范

- **前端**：使用 Prettier 格式化和严格 TypeScript
- **后端**：使用 rustfmt、locked Cargo check、拒绝 warning 的 Clippy 和测试
- **Tauri 2.0**：命令名必须使用 camelCase
- **运行时测试**：使用 Node 24 原生 Fetch 与 MSW/Tauri fake；不得恢复 Fetch
  polyfill 或抑制弃用告警
- **用户可见文本**：同步四份 locale，并保持角色、键盘/焦点、标签和错误状态等
  无障碍行为

提交前运行所有检查：

```bash
mise run check
```

格式化、视觉基线更新、图标生成、版本 apply、依赖升级等修改型任务必须显式执行并
审阅，不能混入普通只读检查。

## Pull Request 指南

1. **先开 Issue 讨论** — 新功能请先开 Issue，不适合项目方向的 PR 可能会被关闭。
2. **基于规范仓库的最新 `main` 创建分支** — 通过符合自身角色的 remote 获取规范来源，再创建功能分支（如 `feat/my-feature` 或 `fix/issue-123`）；不得直接提交或 force-push 到 `main`。
3. **保持 PR 专注** — 针对规范仓库的 `main` 开一个聚焦 PR，每个 PR 只做一个功能或修复，避免无关改动。
4. **遵循 PR 模板** — 填写概述、关联 Issue 和检查清单。
5. **等待精确 head 门禁** — 名称稳定的 `CI / Required` 必须在 PR 的精确 head 上成功；其他 SHA 或单个 job 不能替代。
6. **使用仓库合并路径** — 接受的改动通过受保护的 Merge Queue，以 merge commit 进入 `main`。在进入 merge-ready 前应尽量整理无意义的 fixup/checkpoint 提交；不得用 squash/rebase 抹掉有意义的工程 ancestry 或 upstream provenance。日常查看主线使用 `git log --first-parent main`，保持一个 PR 一个主线边界。除非 GitHub 实时保护策略另有要求，此流程不承诺删除源分支，也不声称必须获得人工批准。

### PR 检查清单

- [ ] 当前宿主的 `mise run check` 通过
- [ ] 如修改了用户可见文本，已更新国际化文件
- [ ] 已记录精确测试、平台限制、风险和回退
- [ ] 长期行为变更已更新可执行测试与维护中的文档
- [ ] 适用时已记录上游 tag/SHA/冲突，或 Release 资产/权限影响

### 提交信息规范

我们使用 [Conventional Commits](https://www.conventionalcommits.org/)：

```
feat(provider): add support for new provider
fix(tray): resolve menu not updating after switch
docs(readme): update installation instructions
ci: add format check workflow
chore(deps): update dependencies
```

## AI 辅助贡献

我们欢迎 AI 辅助的贡献，但**责任始终在你身上**。AI 工具降低了写代码的成本，但并没有降低 review 的成本。维护者没有义务替你清理 AI 的产出。

提交 PR 即表示你同意以下规则：

1. **你已阅读并理解了你的代码。** 你必须能解释 PR 中的每一行。如果做不到，说明还没准备好提交 review。
2. **你已亲自测试过。** 每个改动都必须在本地验证——而不是"看起来对"。不要提交你自己无法测试的平台或功能的代码。
3. **PR 必须小而聚焦。** 一个 Issue 对应一个 PR。大而散、跨多个主题的 PR 会被直接关闭。
4. **先开 Issue 讨论。** 没有事先讨论的"路过式 PR"——尤其是 AI 生成的——可能会被直接关闭。
5. **维护者可以直接关闭。** 看起来是未经审阅的 AI 产出的 PR——虚构的修复、不必要的重构、缺乏上下文的批量改动——维护者可自行决定关闭。

**一句话总结**：AI 是工具，不是理解力的替代品。用它来帮助你更好地贡献，而不是把工作转移给维护者。

## 规划与长期行为

跨层、安全、持久化或发布等复杂工作，应在 Issue、Pull Request 或维护者同意的其他规划
载体中记录目标、非目标、影响边界、风险、验证和回退。贡献、构建、CI 与发布不强制使用
某一种任务框架。

实现形成长期工程规则时，要同步更新执行规则的代码或配置、可执行测试以及解释该流程的
维护中文档。`.trellis/spec/` 下维护中的可选 AI 辅助材料可在确有帮助时更新，但它不是
贡献前提，也不能替代可执行证据。不得改写已归档任务或既有 workspace journal 记录；
需要纠正时，应写入新的当前材料或追加记录。

## 上游 CC Switch 变更

CC Switch 同步使用独立的只读 fetch 维护 remote（维护中的 `upstream:*` 任务将其命名为
`upstream`）。该 remote 与 FyAgent 规范来源和贡献者 fork 都不同。checkout 的常规 push
目标仍是 `origin`：维护者 checkout 指向规范仓库，外部贡献者 checkout 指向个人 fork。
正式上游标签工作会验证不可变 tag object 与 peeled commit，保留双亲 merge commit 和
MIT ancestry，并在任何自动 commit/push 前停止。包含上游变更的 PR 必须记录 tag、完整
SHA、冲突裁决、保留的 FyAgent 专属契约、测试与回退边界。

## 证据与发布变更

PR 必须区分本地/静态检查与真实原生 runner 或已发布 Release 证据。修改 workflow、
权限、安装包、资产名、attestation 或发布事务时，运行 `mise run release:check` 并列出
尚未完成的远程门禁。不得用本地检查声称已完成签名/公证、原生平台、受保护分支/环境
或正式 Release 发布。

## 许可与贡献权利

提交贡献即表示你确认有权按本仓库的许可模式授权你自己的贡献。请勿提交采用不兼容条款的
代码、资产或数据；贡献中包含第三方材料时，请说明其来源和许可证。

FyAgent 计划为 FyAgent 自有代码提供商业许可。在经过法律审查并部署贡献者许可协议或明确的
再许可流程之前，Pull Request 不会自动转让版权。维护者在该流程建立前不应合并会影响商业许可
能力的重大外部贡献。

这些要求不会移除或改变 CC Switch 衍生部分的署名或 MIT 许可，包括对原作者 Jason Young 的
署名。

## 国际化（i18n）

FyAgent 维护四份 locale 资源。修改用户可见文本时：

1. **同时更新四份**语言文件：
   - `src/i18n/locales/en.json`
   - `src/i18n/locales/ja.json`
   - `src/i18n/locales/zh.json`
   - `src/i18n/locales/zh-TW.json`
2. 所有 UI 文本使用 i18next 的 `t()` 函数。
3. 不要硬编码用户可见的字符串。

## 有疑问？

- [提问](https://github.com/fy-agent/fyagent/issues/new?template=question.yml)
- [GitHub 讨论区](https://github.com/fy-agent/fyagent/discussions)
