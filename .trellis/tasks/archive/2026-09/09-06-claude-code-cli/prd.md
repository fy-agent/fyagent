# Claude Code CLI 安装与登录

## Goal

Claude 仅保留 CLI 生命周期，复用可信镜像安装基础设施和官方 CLI 登录

## Background

Agent lifecycle currently defaults Claude to Desktop and rejects CLI, although `agent_install/cli.rs` already maps Claude to Tooling and `auth_actions.rs` already implements `claude auth login/status`. Official documentation and npm metadata confirm the official npm distribution with exact-version platform dependencies; installation can reuse the existing Grok registry/execution policy.

## Requirements

- Claude's public Agent surface is CLI only; never download/launch Claude Desktop through these actions.
- One-click install uses the official package from reviewed mainland-first registries with pinned version and verified integrity. Never trust a mirror's self-asserted hash as the reference or change global npm configuration.
- Preserve existing installation ownership on updates; do not silently replace another managed distribution or run user-controlled CLI code elevated on Windows.
- Reuse the official CLI login/status adapter. Show the vendor-owned effects before launching login; only verified status can report logged in.
- No install/auth claim beyond actual evidence; missing Node/npm, unsupported platform, ambiguous ownership and unverified source remain actionable failures.

## Acceptance Criteria

- [x] Catalog/policy/readiness/Agent actions agree that Claude is CLI-only.
- [x] Root and host package integrity are checked against compiled reviewed truth; mismatch/failure advances only through the closed registry chain.
- [x] Installer invocation is exact-version, per-operation registry, bounded and ordinary-user; no global npmrc/shell-profile mutation or desktop install.
- [x] Login launches only the fixed official CLI command; bounded allowlisted status output is parsed without secret disclosure.
- [x] Tests cover source, policy, update owner, unsafe Windows execution refusal, UI copy and login cancel/handoff semantics.
- [x] Owning specs and evidence are updated before archival; native limitations are explicit.

## Out of Scope

Claude Desktop, custom OAuth/credential-store implementation, new managed Anthropic account provider, automatic authentication during install, system package installation and release/deployment.
