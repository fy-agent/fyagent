# Frontend Development Guidelines

The product has one renderer. This index routes to contract owners; it does
not duplicate DTOs, state machines, filenames or implementation behavior.

## Reading order / Pre-Development Checklist

Read [Directory Structure](./directory-structure.md),
[Modular Boundaries](./modular-boundaries.md), [Type Safety](./type-safety.md),
[State Management](./state-management.md) and [Reuse](./reuse.md), then the
focused feature owner. Apply [Component Guidelines](./component-guidelines.md),
[Hook Guidelines](./hook-guidelines.md) and [User-Facing Copy](./user-facing-copy.md).

## Shared contracts

| Contract                                                    | Owns                                                                        |
| ----------------------------------------------------------- | --------------------------------------------------------------------------- |
| [Directory Structure](./directory-structure.md)             | Single entry, role placement and test environments.                         |
| [Modular Boundaries](./modular-boundaries.md)               | Domain, renderer layers, native ports and import constraints.               |
| [Type Safety](./type-safety.md)                             | Unknown input, guards, closed DTOs and exhaustive states.                   |
| [State Management](./state-management.md)                   | URL/query/draft/secret and native authority ownership.                      |
| [Reuse](./reuse.md)                                         | Adopted primitive/shared owner registry and anti-clone rules.               |
| [Security Boundaries](./security-boundaries.md)             | Structured input, production entry, dependency graphs and test boundaries.  |
| [Quality Guidelines](./quality-guidelines.md)               | Correctness, browser/production measurements and evidence limits.           |
| [Localization](./localization.md)                           | Current Chinese UI, manual languages and future locale admission.           |
| [Visual Language](./visual-language.md)                     | Typography, density, focus and shared hierarchy.                            |
| [Blue Appearance](./appearance.md)                          | Paired themes, preference, native synchronization and radial reveal.        |
| [Surfaces and Container Response](./surfaces-responsive.md) | Material, contrast, roundness and library-backed stable panes.              |
| [Shared Motion](./motion-system.md)                         | Time units, press, media preferences, disclosure and notification motion.   |
| [Dialog Lifecycle](./dialog-lifecycle.md)                   | Source geometry, content resize, session teardown and focus/scroll release. |

## Shell and feature owners

| Contract                                              | Owns                                                                            |
| ----------------------------------------------------- | ------------------------------------------------------------------------------- |
| [Navigation](./navigation.md)                         | Hash routes, literal loaders, keep-alive lifetime, blockers and return context. |
| [Window Shell](./window-shell.md)                     | Chrome, native overlay boundary, selection and shared interaction.              |
| [Change Plan Workspaces](./change-plan-workspaces.md) | Preview/apply, source switching, job observation and reconciliation.            |
| [Agent Directory](./agent-directory.md)               | Catalog, scan/readiness, cards, installation and capabilities.                  |
| [External Agent Auth](./agent-auth.md)                | Native auth observations, session ownership and safe handoff.                   |
| [Managed Auth](./managed-auth.md)                     | Accounts/connections/request sources, login and impact confirmation.            |
| [Models](./models.md)                                 | Drafts, connectivity, native save and existing model workflows.                 |
| [Managed Grok Subscriptions](./grok-subscription.md)   | Explicit account/model binding, native readback and subscription scope.         |
| [Assignments](./assignments.md)                       | Shared seven-target selection and serialized mutations.                         |
| [Skills](./skills.md)                                 | Discovery, installed items, backups and assignment.                             |
| [MCP](./mcp.md)                                       | Catalog/launch validation, CRUD, installation and assignment.                   |
| [Prompts and Memory](./prompts-memory.md)             | Native content CRUD, editor/dirty state and directory operations.               |

## Historical discovery routers

[Shell](./shell.md), [Agents and Models](./agent-models.md) and
[Skills and MCP](./skills-mcp.md) only point to focused contracts. They are not
alternative implementations. New work cites the focused owner.

## Quality Check

Every changed owner supplies its required behavior tests. `mise run check`
includes the single strict type/lint pipeline and renderer/domain/contract
unit tests. Run `mise run test:browser` for production boot and browser behavior,
and `mise run test:performance` serially for actual motion/navigation costs.
Task/SPEC-only edits still run `mise run check:contracts`; active tasks use the
exact task exclusion at prearchive, then validate effective context references.
Required owner documents must fit `context_injection.max_file_bytes`: use
`task.py validate` to detect truncation and split a cohesive feature contract
rather than raising the limit or silently losing the end of a required spec.

Migrations preserve native commands and persisted identities, update effective
source/SPEC/CI/test references and explicitly account for retired UI assertions.
Historical prose/commit hashes are evidence, not live paths to mass-rewrite.
Browser/mock fixtures never become native, signature or real-account evidence.
