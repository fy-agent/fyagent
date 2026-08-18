# Agent Install Contract

## 1. Scope / Trigger

Read this note before changing `src-tauri/src/agent_install/`,
`src-tauri/src/commands/agent_install.rs`, the first-wave registry JSON,
`src/lib/api/agent-install.ts`, or the Settings agent-catalog panel. This
surface is the six-agent G1 install chain (#25–#29, #32). It does **not** own
Codex Desktop MSIX (`codex_desktop_*`) or FyAgent’s own NSIS installer.

## 2. Signatures

Public Tauri commands are exactly:

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

`StartAgentInstallRequest` serializes as `{ "snapshotId": "<id>" }` with
`deny_unknown_fields`. No ordinary command accepts a URL, path, hash, shell
script, or validation-bypass flag.

The bundled helper keeps `codex-msix-install`. A second exact verb
`agent-machine-install --job-id <uuid> --pipe <64hex>` is the only new
machine-scope CLI. User-scope installs run in the main app.

## 3. Contracts

- Four layers stay independent. `unknown` is never pass. `fail` and `unknown`
  block package install. `warn` may continue with a visible warning.
- Hosted or cached install requires `redistribution_allowed=true`. Official
  landing remains available when hosting is blocked.
- Plan drift includes source kind, version, hash, actions, signer, revocation.
- Health probe: install / runtime / readiness. Unauthenticated is
  `installed_healthy_pending_auth`. No `codex exec` or `claude -p`.
- Do not claim #22 done. Do not adopt #110’s “drop hash” policy.

## Tests Required

```bash
mise run rust:test -- agent_install
mise run rust:test -- user-helper
```

Windows HIL is residual. Mock or unit success is not native installer evidence.
