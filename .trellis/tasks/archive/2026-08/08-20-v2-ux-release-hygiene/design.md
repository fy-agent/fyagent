# Design

## Boundaries

| Child | Owns | Must not touch |
|---|---|---|
| `08-20-skill-mcp-agent-ui` | Skill/MCP header chrome, Agent intros | models IPC, release YAML |
| `08-20-install-model-gates` | Codex installer hash path, Claude v1 warn, draft URL stream_check | Skill/MCP layout, release eligibility |
| `08-20-release-ci-speed` | `release.yml`, eligibility, CI/Release cache pins, 0.4.2 notarize-once port | V2 pages |
| `08-20-repo-hygiene` | docs-contract tests, gitignore, safe untrack | installer hash semantics, eligibility policy |

## Cross-child contracts

- Agent intro is page-local Chinese copy (`src/v2/pages/agents/intros.ts`), not Rust `description`.
- Connectivity is a new IPC `stream_check_url({ baseUrl })` plus V2 `providers.checkReachability` / equivalent on WorkBuddy and OpenCode ports. Qoder/TRAE panels stay unchanged.
- Release frozen identity may drop required `ciRunId` or make it optional; tag target SHA remains `sourceSha`.
- Installer: keep exclusive create / no-follow / OS installer / post-install existence. Remove `revalidate_artifact` / `VerifiedFilePin` full-file SHA reread loops.

## Tradeoffs

- Dropping prior CI as a Release gate means Release itself is the compile proof. Accepted: user asked to stop waiting for a second full CI.
- Allowing unpublished tag moves is a retry hatch, not in-place overwrite of a published GitHub Release.
- Registry cache is lockfile-keyed and never includes `src-tauri/target`.
