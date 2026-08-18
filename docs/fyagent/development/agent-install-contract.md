# Agent install contract (G1 #25–#29, #32)

This is the public contract for the six-agent install chain. Current code and
tests are executable authority. This page explains the product gates so a
reviewer can check a PR without reading the Trellis task folder.

Related GitHub issues: [#25](https://github.com/fy-agent/fyagent/issues/25),
[#26](https://github.com/fy-agent/fyagent/issues/26),
[#27](https://github.com/fy-agent/fyagent/issues/27),
[#28](https://github.com/fy-agent/fyagent/issues/28),
[#29](https://github.com/fy-agent/fyagent/issues/29),
[#32](https://github.com/fy-agent/fyagent/issues/32).

This contract is **not** the Codex Desktop MSIX installer. That locked surface
stays on `codex_desktop_*` plus `fyagent-user-helper.exe codex-msix-install`.
The six-agent catalog lives in a sibling domain: `agent_install_*`.

## Locked decisions

1. Four independent layers: source/license, integrity, preflight, plan.
   Never merge them into one green “installable”.
2. Layer states: `ok | warn | fail | unknown`. Preflight items:
   `pass | warn | fail | unknown`.
3. `warn` may continue with a visible warning. `fail` and `unknown` block
   package install.
4. `redistribution_allowed=false` or `null` fails hosted, cached, or mirrored
   install. Official landing / guide may still be shown.
5. Plan drift that requires reconfirm: source kind, version, hash, actions,
   signer identity, revocation status.
6. `agent_install_start_install` accepts only `{ snapshotId }`.
7. Renderer never sends URL, path, hash, or a shell script.
8. Frozen product boundaries: do not rewrite #35, #41, #49, #50, #51.
9. #32 has three gates. Unauthenticated is `installed_healthy_pending_auth`,
   not an install failure. No headless model request.
10. #22 catalog fact contract stays a separate OPEN issue. This registry is an
    install-source table for six first-wave agents, not a claim that #22 is done.
11. #110 (source-only one-click, drop hash) is **rejected** for this chain.
    #26 remains fail-closed.
12. This PR does not migrate the FyAgent Windows host from
    `requireAdministrator` to `asInvoker`. #29 here means: Agent user-scope
    work stays in the main app; machine-scope work uses a second typed helper
    verb. Native Windows HIL is residual (`code_audit`).

## First-wave agents

| ID | Mode | Redistribution | First-wave execute |
| --- | --- | --- | --- |
| `qoderwork-cn` | official_guide | false | open official landing |
| `dingtalk-wukong` | official_guide | false | open official landing |
| `workbuddy` | official_guide | false | open official landing |
| `trae-work` | official_guide | false | open official landing |
| `codex-cli` | package_manager | true | pinned npm/brew; integrity `warn` |
| `claude-code` | native_verified | false | official manifest + SHA-256; no cache |

## IPC

```
agent_install_list_catalog()
agent_install_get_contract(agentId)
agent_install_refresh_preflight(agentId)
agent_install_create_plan(agentId)
agent_install_reconfirm_plan(snapshotId)
agent_install_start_install({ snapshotId })
agent_install_get_job()
agent_install_cancel_install(jobId)
agent_install_probe_health(agentId)
agent_install_open_official_guide(agentId)
```

## Evidence levels this PR can claim

| Surface | Level |
| --- | --- |
| Registry, source gate, integrity matrix, preflight TTL, plan hash | unit |
| Claude Code verifier on fixtures | unit |
| Codex CLI / Claude Code executor | unit with mocked runner |
| Helper second verb | unit / `code_audit` |
| Windows UAC / PackageManager / live download | residual, not claimed |

## Local checks

```bash
mise run rust:test -- agent_install
mise run rust:test -- user-helper
mise run typecheck
mise run test:unit -- agent-install
mise run test:i18n
```
