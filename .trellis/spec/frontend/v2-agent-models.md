# V2 Agents and Models Compatibility Router

This path is retained because archived Trellis tasks and reviews reference it.
It is a reading map, not a second implementation authority. New work must cite
the smallest focused contract below instead of adding detailed behavior here.

## Read by concern

| Concern | Authoritative contract |
| --- | --- |
| Agent directory scan, ordering, cards, capability projection, lifecycle UI and return links | [V2 Agent Directory](./v2-agent-directory.md) |
| Agent Auth observation, sessions, polling and evidence-correct status UI | [V2 External Agent Auth UI](./v2-agent-auth.md) |
| Models target selection, drafts, tests, typed preview/apply, quick setup, WorkBuddy and Codex model flows | [V2 Models](./v2-models.md) |
| Route registration, keep-alive visibility, sidebar and closed Agent return query | [V2 Navigation and Persistent Route](./v2-navigation.md) |
| Shared shell, native-overlay chrome, selection material, motion and external opening | [V2 Window Shell and Interaction](./v2-window-shell.md) |
| Native Agent catalog/runtime observation and generic launch | [External Agent Catalog and Runtime](../backend/external-agent-catalog-runtime.md) |
| Native install/update/launch jobs and inventory capabilities | [External Agent Installation and Lifecycle](../backend/external-agent-lifecycle.md) |
| Native Agent authentication sessions | [External Agent Authentication](../backend/external-agent-auth.md) |
| TRAE/OpenCode model-native boundaries | [External Agent Model Integration](../backend/external-agent-models.md) |
| Typed preview/apply/idempotency/compensation | [Change Plan Typed Executor](../backend/change-plan-executor.md) |
| Codex and WorkBuddy persistence transactions | [Codex Provider Configuration](../backend/codex-provider-configuration.md) and [WorkBuddy Configuration](../backend/workbuddy-configuration.md) |

## Shared invariants

- The Renderer displays native catalog/readiness/inventory/auth/readback; it
  does not infer installation, authentication, target identity, or mutation
  success from local component state.
- Pages submit closed Agent/target/action IDs and typed bounded payloads through
  feature Ports. Native code owns paths, process launch, package/application
  identity, credentials, live files, backups, and side effects.
- Preview and apply use the same typed plan/revision authority. Stale preview,
  target drift, partial result, rollback and recovery-required states remain
  explicit rather than becoming a generic success toast.
- Cross-page return state is the closed Agent ID/section tuple owned by the
  navigation contract, never a caller-provided return path.
- Keep this router short. A change that needs a new invariant belongs in the
  focused owner and its tests, followed by a link update here only when needed.
