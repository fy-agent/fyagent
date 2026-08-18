# Issue #35 implementation entry

The only execution authority is `execution-plan.md`; production implementation has not started.

Current gate: `DESIGN_FREEZE=PENDING`.

Do not report the presence of PRD/design files as implementation. After freeze, every worker receives the immutable contract SHA plus the exact file-owner list from `detailed-design-overview.md`, runs only its module tests, and returns to the main thread for integration.
