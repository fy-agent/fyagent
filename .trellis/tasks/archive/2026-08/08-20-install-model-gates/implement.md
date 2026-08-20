# Implement

1. Strip installer full-file SHA rereads; update installer tests/spec `codex-desktop-installer.md`.
2. Add Claude v1 warning helper + UI; tests in `quickSetup.test.ts` / models Page tests.
3. Public `probe_reachability` / `check_url`; command `stream_check_url`; register in `lib.rs`.
4. V2 ports + tauri/browser adapters; buttons on WorkBuddy, ProviderPanel (claude/codex/grokbuild), OpenCode. Not Qoder/Trae.
5. Spec `v2-agent-models.md`.

```bash
mise run typecheck
mise run rust:test
pnpm exec vitest run tests/v2/pages/models/Page.test.tsx tests/v2/pages/models/quickSetup.test.ts tests/v2/pages/models/workBuddyModels.test.ts
```
