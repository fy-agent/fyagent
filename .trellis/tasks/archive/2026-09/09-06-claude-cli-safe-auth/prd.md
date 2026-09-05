# Claude CLI 与用户配置可逆修改

## Goal

独立工作树交付 Claude Code CLI 镜像安装及官方登录入口，并修复认证切换的配置覆盖、修改披露与可撤销备份底线

## Requirements

- R1: Work only in `feat/claude-cli-safe-auth` in the isolated worktree; leave the concurrent main checkout untouched.
- R2: Expose Claude Code CLI, not Claude Desktop, with a mainland-first verified install source and the official CLI login/status flow. Reuse Tooling, Agent jobs, ordinary-user execution and reviewed dependencies.
- R3: Separate account credentials from model request routing. Auth connection/account changes must not replace Codex `config.toml`; reject unavailable third-party settings before any write.
- R4: Before a user-file mutation, disclose the actual native-resolved targets, retained backup and undo behavior, then require explicit confirmation. Successful login alone must not silently project credentials into consumer files.
- R5: Put backup-before-write, atomic replacement, failure preservation and reversible recovery below feature callers. Keep one rolling preimage; refusal or failure to back up must leave primary bytes unchanged. Recovery must not clobber later external edits.
- R6: Update owning specs before task archival, record honest validation boundaries, commit only this work and finish with a clean worktree.

## Task Map

| Child                          | Owns                                                                                          |
| ------------------------------ | --------------------------------------------------------------------------------------------- |
| `09-06-reversible-config-auth` | Shared file safety, recovery/disclosure, Codex auth/config separation and regression coverage |
| `09-06-claude-code-cli`        | CLI-only catalog/lifecycle, verified mirror installation and official login handoff           |

## Acceptance Criteria

- [x] Claude actions never install or launch Claude Desktop; installation and sign-in remain separate outcomes.
- [x] Unknown/mismatched package sources and elevated-user CLI execution fail closed.
- [x] Auth account switching leaves an unrelated, commented Codex config byte-identical; a saved login is not mistaken for a request-source change.
- [x] Missing provider credentials, stale previews, cancellation, failed backup and failed replacement have zero destructive primary-file effects.
- [x] Users see real targets and backup locations before confirmation and can recover the retained preimage without overwriting an external change.
- [x] Both children pass their focused tests and the final applicable prearchive gate; specs, task records and commits are complete.

## Out of Scope

No Claude Desktop support, bespoke OAuth implementation, automatic changes to shell profiles/global npm registry, credential sharing across consumers, main-checkout changes, PR/merge/release, or native Windows/HIL claims based on macOS tests.
