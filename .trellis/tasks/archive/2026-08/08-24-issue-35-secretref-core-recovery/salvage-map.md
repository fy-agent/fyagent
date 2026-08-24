# PR #112 salvage map

PR #112 and branch `codex/issue-35-secret-backend` are behavioral references only. No old commit is rebased or cherry-picked into this recovery branch.

## Preserved behavior and test intent

| Old input | Preserved intent | New narrow implementation |
| --- | --- | --- |
| `a338ee18` / `2f83f34f` | Frozen D2 invariants: unpredictable refs, no-value public shape, fail closed, no SQLite ownership | `types.rs`, `error.rs`, task PRD/design |
| `d57957e7` | Material validation/zeroization, closed backend facade, source-free errors | `material.rs`, `backend.rs`, `error.rs` |
| `b28f5f35` | macOS Keychain create-not-upsert/readback and OS-store boundary | `platform/macos.rs`; Windows stub replaced by actual `platform/windows.rs` Credential Manager leaf |
| `4771a083` | Runtime-generated canaries and focused SecretRef/CRUD contract tests | `tests/secret_service_contract.rs` |
| `4ef73d75` | Renderer-facing objects must be material-free and strict | output-only camelCase `SecretSummaryDto`/error/delete DTO tests; old V2 decoder/UI not copied |
| `eaab3091` / `ec54eb71` | Secret-surface scanning and contract registration intent | focused DTO/key/debug/canary tests now; shared registration and repository-global scanner remain integration work |
| `b5c7013b` | Deterministic projection integrity concern | no secret-bearing projection digest is implemented or persisted; long-lived authority is random `secretRef/version` |

## Explicitly discarded

- The 5k-line lifecycle operation algebra, 2k-line backend broker, and unfinished `todo!` authority constructors.
- Device journal/store, candidate activation, hardware adapter product surface, credentials panel prototype, and old V2/legacy `App.tsx` wiring.
- Prompt/Memory commits, archived task bulk, schema v17/v20 claims, old AppState/command/lib registration, and any direct Provider writer coupling.
- Windows compile-only stub and every old statement that treated static design, mock tests, or one host as cross-platform runtime proof.
- RFC8785 or ordinary/HMAC digests over secret-bearing Provider/live projections.

## Deferred before Issue #35 can close

- Serial `services/mod.rs`/AppState/command registration after UCP shared-file handoff.
- Device-local owner binding and lifecycle authority, Provider create/edit integration, legacy migration, V2 Credentials UX, and full repository canary inventory.
- Windows matching-host Credential Manager CRUD/readback/cleanup HIL and Required CI at the replacement PR exact SHA.
