# Frontend Development Guidelines

This directory defines renderer architecture, state, UI quality, copy,
localization, and the current V2 feature contracts. The index only routes
readers; concrete Ports, DTO parsers, page behavior, failure states, and test
assertions belong in the linked owner.

## Reading order

1. Read [Directory Structure](./directory-structure.md) and
   [Renderer Modular Boundaries](./modular-boundaries.md).
2. Read [Type Safety](./type-safety.md),
   [State Management](./state-management.md), and
   [Frontend Reuse](./reuse.md) before adding data flow or shared UI.
3. Apply [Component Guidelines](./component-guidelines.md),
   [Hook Guidelines](./hook-guidelines.md),
   [Quality Guidelines](./quality-guidelines.md), and
   [User-Facing Copy](./user-facing-copy.md).
4. For V2 work, read the focused navigation/window-shell contract plus the
   owning feature contract. Use compatibility routers only for archived broad
   references.

## Foundation contracts

For URL classification, dynamic text, configuration merges and standalone
HTML parsing, also read [Security Boundaries](./security-boundaries.md).
For the shared type scale, dialog sizing/focus and navigation shape, read
[Desktop Visual Hierarchy](./visual-language.md).
For material backing, semantic radii, contrast evidence and narrow containers,
read [Surfaces and Container Response](./surfaces-responsive.md).

| Contract                                               | Owns                                                                                                       |
| ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------- |
| [Directory Structure](./directory-structure.md)        | Renderer directory roles and placement.                                                                    |
| [Renderer Modular Boundaries](./modular-boundaries.md) | Renderer/host, V2/leftover, feature/platform, and import boundaries.                                       |
| [Type Safety](./type-safety.md)                        | `unknown` parsing, DTO validation, exhaustive unions, and prohibition on scattered casts.                  |
| [State Management](./state-management.md)              | Server, URL, local draft, secret, and derived-state ownership.                                             |
| [Frontend Reuse](./reuse.md)                           | Reuse order, shared-owner registry, component placement, dependency review, and anti-clone rules.          |
| [Component Guidelines](./component-guidelines.md)      | Component APIs, semantics, accessibility, composition, and presentation ownership.                         |
| [Hook Guidelines](./hook-guidelines.md)                | Hook responsibilities, lifecycle, query/effect ownership, and stable return shapes.                        |
| [Quality Guidelines](./quality-guidelines.md)          | Loading/error/empty states, test levels, deterministic behavior, and acceptance evidence.                  |
| [User-Facing Copy](./user-facing-copy.md)              | Evidence-correct copy, errors, labels, installer wording, and prohibition on internal/GPT-style narration. |
| [Frontend Localization](./localization.md)             | Leftover locale authority, exact key parity, detection, fallback, and V2 import boundary.                  |

## V2 architecture and shell

| Contract                                                 | Owns                                                                                                                                                 |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| [V2 Navigation and Persistent Route](./v2-navigation.md) | Hash route registry, literal lazy loaders, persistent page lifetime, hidden query isolation, blockers, sidebar state, and closed Agent return state. |
| [V2 Window Shell and Interaction](./v2-window-shell.md)  | AppShell/TopBar, native-overlay boundary, selection geometry, shared motion/collapse, external opening, and V2 architecture imports.                 |

## V2 feature contracts

| Contract                                                       | Owns                                                                                                                                       |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| [V2 Agent Directory](./v2-agent-directory.md)                  | Catalog/readiness scan, order, cards, capability projection, lifecycle UI, and Agent return links.                                         |
| [V2 External Agent Auth UI](./v2-agent-auth.md)                | Agent-card Auth summaries, Agent-owned session recovery/polling, desktop target selection, and managed-account routing.                    |
| [V2 Managed Accounts and Authentication](./v2-managed-auth.md) | Central `/auth` account/connection/request-source UI, strict ManagedAuthPort, login sessions, impact previews, and responsive interaction. |
| [V2 Models](./v2-models.md)                                    | Target selection, drafts/tests, typed preview/apply, quick setup, WorkBuddy, TRAE, OpenCode, and Codex model flows.                        |
| [V2 Shared Assignment](./v2-assignments.md)                    | Seven-target presentation order, shared AssignmentPanel API, serialized mutation, and authoritative reread.                                |
| [V2 Skills](./v2-skills.md)                                    | Installed/discovery views, SkillHub/repository sources, pagination, install targets, backups, and Skill assignments.                       |
| [V2 MCP](./v2-mcp.md)                                          | Installed/discovery views, launch parsing, security projection, presets, install, and MCP assignments.                                     |
| [V2 Prompts and Memory](./v2-prompts-memory.md)                | Prompt/native-memory Ports, CRUD/enable flows, directory operations, and Agent prompt delegation.                                          |

## Compatibility routers

- [V2 Shell](./v2-shell.md)
- [V2 Agents and Models](./v2-agent-models.md)
- [V2 Skills and MCP](./v2-skills-mcp.md)

These paths preserve archived references only. New work cites the focused
contract above and must not add detailed behavior back to a router.

## Maintenance rules

- A feature contract owns its exact UI/Port behavior. Foundation specs state
  reusable rules and link instead of copying feature matrices.
- Integrate the current rule into the owning body; do not stack dated override
  blocks above contradictory text.
- V2 production code uses approved shared/platform boundaries. A test fixture
  or browser preview never becomes desktop authority.
- Keep native-only, readback, partial-result, accessibility, responsive, and
  secret/error-redaction cases explicit even when a feature contract is longer.

## Quality Check

Use every affected contract's **Tests Required** section. Focused renderer work
may start with `mise run check:frontend` or the owning V2 type/lint/test tasks;
the standard local implementation gate is `mise run check`. Documentation-only
spec/task changes still run `mise run check:contracts`, and active Trellis work
uses the exact prearchive exclusion before archival. Browser fixtures and mock
IPC prove only their declared scope; they do not become native desktop evidence.
