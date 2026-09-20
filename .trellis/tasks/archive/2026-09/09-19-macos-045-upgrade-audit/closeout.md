# macOS 0.4.5 audit closeout

- Release verified at v0.4.5 / 64f4d8f; no 0.49 or 0.4.9 release/tag. Last dev/laiyongjie handoff was PR #190, then release PR #191.
- Installed local arm64 0.4.5 from official source plus two reviewed fixes. Ad-hoc resource signature verified; not a Developer ID release. Native About, final app startup and all eight page families exercised.
- Fixed Codex unmanaged OAuth Health false logout/missing-credential status and Agent-to-Prompts wrong-target navigation. Native regressions include target-specific disabled save, cancel/discard, re-entry and cleanup.
- Original checkout and existing data preserved; business tables and tracked configuration/memory fingerprints unchanged after temporary data cleanup. Final accessible app inventory has one FyAgent.app and same-version official recovery DMG.
- Health36/36 and managedAuth123/123; type/lint/fmt/Rust check/Clippy and final contracts pass. Full renderer/contracts run had1716 pass,1skip and2 platform-identity failures; single protected digest corrected and its24 tests reran successfully. Production native build exit0.
- Remaining: OpenCode builtin npm compatibility, OpenClaw object apiKey compatibility, Grok selected-profile/installation issues, Codex/Claude saved-vs-live configuration drift, official DMG embedded-app staple ordering, conditional Health overview database writes, Claude directory readiness disagreement.
- Real vendor OAuth/entitlements/inference, external MCP execution, third-party software updates, privileged helper commits and Windows HIL are not verified. No main merge, tag or Release created.
- Detailed private report: ~/Documents/FyAgent-0.4.5-本机验收-20260919/验收报告.md; raw local evidence: ~/fyagent/tmp/version-audit-2026-09-19/.
