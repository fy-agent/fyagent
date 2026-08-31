# Phase 0 baseline

Date: 2026-08-31

```text
commit:  1d0aeecc5b4cff9dc914907f24a7ed321daff75b
branch:  dev/laiyongjie
status:  predecessor 08-31-macos-agent-install-update-experience archived
         helper-facing Agent/Codex contracts are in tree
```

## Settled helper-facing owners

- Download: `codex_desktop::download::DownloadedArtifact`
- DMG transaction: `codex_desktop::platform::macos::dmg::install_managed_exact`
- Agent adapter: `agent_install/macos.rs` rejects
  `InstallationScope::AllUsers` and `MacSystemApplications` with
  `AuthorizationRequired`
- Inventory: `FreshDestinationCapability::MacSystemApplications` is visible
  and non-actionable
- Product identity: `agent_install/desktop.rs` `DESKTOP_PRODUCTS` plus Codex
  `com.openai.codex` / `ChatGPT.app` with historical `Codex.app`
- Release: `scripts/release/macos-developer-id.sh` signs the top-level app
  only; nested helper signing is the gap this task fills

## Production enablement

Keep system actions disabled. Formal Developer ID / notarized HIL is not
assumed available in this session. Archive language, if HIL is missing, is
“implementation and portable tests complete; `/Applications` one-click remains
disabled”.
