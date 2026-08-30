# Frontend Development Guidelines

This directory defines renderer architecture, state, UI quality, copy, reuse,
and the current V2 feature contracts. The index only routes readers; concrete
ports, DTO parsers, page behavior, failure states, and tests belong in the
linked spec.

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
4. For V2 work, read [V2 Shell](./v2-shell.md) and the owning feature contract.

## Foundation contracts

| Contract | Owns |
| --- | --- |
| [Directory Structure](./directory-structure.md) | Renderer directory roles and placement. |
| [Renderer Modular Boundaries](./modular-boundaries.md) | Renderer/host, V2/leftover, feature/platform, and import boundaries. |
| [Type Safety](./type-safety.md) | `unknown` parsing, DTO validation, exhaustive unions, and prohibition on scattered casts. |
| [State Management](./state-management.md) | Server, URL, local draft, secret, and derived-state ownership. |
| [Frontend Reuse](./reuse.md) | Reuse order, shared owner registry, component placement, dependency review, and anti-clone rules. |
| [Component Guidelines](./component-guidelines.md) | Component APIs, semantics, accessibility, composition, and presentation ownership. |
| [Hook Guidelines](./hook-guidelines.md) | Hook responsibilities, lifecycle, query/effect ownership, and stable return shapes. |
| [Quality Guidelines](./quality-guidelines.md) | Loading/error/empty states, test levels, deterministic behavior, and acceptance evidence. |
| [User-Facing Copy](./user-facing-copy.md) | Human-readable copy, evidence strength, errors, labels, and prohibition on internal/GPT-style narration. |

## V2 contracts

| Contract | Owns |
| --- | --- |
| [V2 Shell](./v2-shell.md) | Route registry, persistent left navigation, native title-bar chrome, layout, motion owner, and route lifecycle. |
| [V2 Agent Directory and Models](./v2-agent-models.md) | Agent catalog/configuration shell, lifecycle/Auth projections, Models setup, write confirmation, and Change Plan UI. |
| [V2 Skills and MCP](./v2-skills-mcp.md) | Skills/MCP ports, discovery, assignment, shared feature UI, secret-safe configuration, and authoritative reread. |
| [V2 Prompts and Memory](./v2-prompts-memory.md) | Prompt/native memory ports, CRUD/enable flows, directory operations, and Agent prompt delegation. |

## Maintenance rules

- A feature contract owns its exact UI/port behavior. Foundation specs should
  state reusable principles and link instead of copying feature matrices.
- Integrate the current rule into the body; do not accumulate dated override
  blocks above contradictory text.
- V2 production code must use the approved shared/platform boundaries. A test
  fixture or browser preview never becomes desktop authority.
- Keep secret, native-only, readback, rollback, accessibility, and responsive
  failure cases explicit even when they make a feature contract longer.

## Quality Check

Use every affected contract's **Tests Required** section. Focused renderer work
may start with `mise run check:frontend` or the owning V2 type/lint/test tasks;
the standard local implementation gate is `mise run check`. Documentation-only
spec/task changes still run `mise run check:contracts`, and active Trellis work
uses the exact prearchive exclusion before archival. Browser fixtures and mock
IPC prove only their declared scope; they do not become native desktop evidence.
