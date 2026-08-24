# Implementation plan

1. Inspect ProviderService lock/current/live boundaries and DB DAO transaction patterns.
2. Define compact secret-free domain DTOs and error/reason enums.
3. Implement DAO SQL methods assuming tables supplied by integration.
4. Implement service create/apply/get/list/reconcile seams and writer-count-safe tests.
5. Add lexical sanitization and negative serialization tests.
6. Report required shared registration/schema changes to the integration task; do not perform them.

## Progress log

- 2026-08-24: claimed by `gpt-5.6-sol-high`; baseline and ownership frozen by root.
- 2026-08-24: implementation completed; external/Trellis P1 fixes were folded back into the same domain and all focused/full Rust gates passed.

## Deliverables

- [Change Plan service](../../../src-tauri/src/services/change_plan/service.rs)
- [Change Plan DAO](../../../src-tauri/src/database/dao/change_plan.rs)

## Acceptance evidence

- Fresh `mise run check` passed: Rust fmt/check/clippy and 2838 library tests passed, 5 ignored.
- Secret-blocked, stale/replay/expiry, concurrency, takeover parity and read-only recovery regressions are part of the passing suite.
