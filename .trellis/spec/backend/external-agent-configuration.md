# External Agent Configuration Compatibility Router

This stable path preserves archived references to the former combined Qoder,
TRAE, and OpenCode configuration contract. It is a reading map only; new work
must cite the focused owner below.

## Read by concern

| Concern | Authoritative contract |
| --- | --- |
| QoderWork Hooks snapshot, closed event/command projection, revision conflict, overwrite capability, backup, atomic replacement and reread | [QoderWork Hooks Configuration](./qoderwork-hooks.md) |
| TRAE endpoint preflight, TRAE Work CN observed model IDs, and OpenCode model snapshot/fetch/save | [External Agent Model Integration](./external-agent-models.md) |
| WorkBuddy revisioned model/config writes | [WorkBuddy Configuration](./workbuddy-configuration.md) |
| Codex/Provider auth/model transaction | [Codex Provider Configuration](./codex-provider-configuration.md) |
| Renderer model composition and Change Plan UI | [V2 Models and Change Plan UI](../frontend/v2-models.md) |

## Shared invariants

- Qoder Hooks and OpenCode model writes remain separate fixed-target native
  transactions. Both use revision checks, bounded one-use overwrite
  authorization, backup/atomic replacement and authoritative reread; neither
  accepts renderer paths or a generic `force` flag.
- TRAE model listing is observation-only. Endpoint testing is a bounded,
  cancellable, secret-safe network probe and does not prove vendor-side model
  configuration.
- Credentials stay in mutation/probe request lifetimes and never enter public
  snapshots, route/query state, logs or serializable diagnostics.
- Unknown or unsupported live document shapes fail closed instead of being
  normalized to a minimal template.
- Keep this router short. Detailed signatures, validation, cases and tests
  belong only in the two focused contracts.
