# Integration and governance consolidation

## Goal

Integrate completed worker outputs into one reachable product chain: Schema v20, command/ACL/lib registration, V2 FeaturePorts/browser/Tauri composition, page wiring and cross-layer tests. Git/GitHub governance remains root-owned.

## Requirements

- Upgrade schema 19→20 and fresh/memory creation with the same idempotent Change Plan table helper.
- Add all Change Plan tables to sync skip and local-preserve boundaries.
- Register exactly four Change Plan commands and one Agent readiness command, with matching ACL union.
- Add `ChangePlansPort` and readiness port/query wiring with strict unknown-input parsers and browser native-only behavior.
- Connect Apply UI and Agent detail without widening Quick Setup, Settings or catalog contracts.
- Resolve compile/test conflicts across worker outputs and remove all stale/fake/old-ID residue.
- Do not commit, push, create/close PRs or edit GitHub.

## Acceptance Criteria

- [x] 0→20, 19→20, future reject, memory DB and sync boundaries pass.
- [x] Registered commands equal ACL union and no forbidden installer/fake commands exist.
- [x] Rust/TypeScript wires and exact parsers agree.
- [x] Real UI paths reach native commands; browser adapters fail native-only.
- [x] Focused integration tests and static stale-reference scans pass.

## Notes

- Starts only after first-wave workers finish and root verifies their artifacts.
