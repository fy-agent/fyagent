# Issue #35 D2 working-tree closure

Status: `APPROVE_FOR_D2_CANDIDATE_COMMIT`. This is a targeted static closure receipt, not an immutable rereview or freeze receipt.

`D=f2f26b8b6b5aa4acf8bbd257cee9ee22713aebaf` remains rejected by `architecture-rereview.md`. The corrected working tree closes exactly `ARR-001..003`; no test, build, dependency resolution or runtime ran.

## Closure

- `ARR-001`: candidate discard/expiry has distinct `Delete` and `Validate` missing-readback slots, an operation-bound CAS reservation and durable `{deleteDisposition,backendCompletedAt,deleteAppliedCas}` checkpoint before fresh missing proof and terminal state.
- `ARR-002`: normal activation, durable failure and activation-cleanup recovery retain the complete role-specific old-record checkpoint; terminal supersession is atomic after fresh missing readback and uses `revokedAt=backendCompletedAt`.
- `ARR-003`: staged-resume CAS preimage binds immutable `operationId` and the exact cumulative five-phase algebra `intent|sourcesScrubbed|cutoverCommitted|liveOwnerMinted|localBindingFinalized`; each phase/fresh nonce/admission changes revision and digest, with five canonical fixture plans.

Independent targeted result: `ARCH_D2_WORKTREE_CLOSURE=APPROVE`, `P0=0/P1=0/P2=0`.

## Authority hashes

| File | SHA-256 |
| --- | --- |
| `secret-contract-v1.md` | `44da40384499df4e1936e12e7006cd89e5f0bc41e98343892df14c5e654e5041` |
| `device-local-secret-store.md` | `07fb3ea341a51ec92a5f50e1745fac1e3eb51037c0e173f5cea4cc4b06a62bb8` |
| `design.md` | `ec5a46de3c315f76160ad6426a1d7bd448afc14eb413c5cdc7fdf39c814619a2` |
| `technical-design-overview.md` | `2f5f13d006d3e20b50689e357438297dbac91e0e54e20f3f66be786c5f5fd69c` |
| `detailed-design-overview.md` | `ae4e768e1a2270600e1aa4fb95ed494b5f48aaf445a4147bc8afa7fb173124fe` |
| `execution-plan.md` | `6d476bf26010deb1548a4cc6fb8bec53bc93bada8a76f5bd4d7e3b5b1ad9deee` |
| `research/secretRef-contract-handoff.md` | `13efb1342360b22f2852229c207b129d90493dd43ebe0dcf783960d32bc8ea62` |
| `research/codex-secret-call-graph.md` | `af66de4fa8fb83a1c565ff3902dcb4eca71b1a5ee791036e11b8bfebc3554ea1` |
| `research/secret-surface-inventory.md` | `3d12125a5c279db01d44dbdf8210ebc2e9ce455af12e6d60202cb6b778736f11` |
| `research/os-keyring-options.md` | `c6a1e8cbbc6cd4691642e351a9ca6e8851347bfe32ba6d40b095a8fca644e4af` |
| `research/native-evidence-plan.md` | `127d75d5e31ada40cfd0dd6cef6a23d100d2bce21329df39257f738c491a9181` |

The next action is to commit this exact candidate as `D2` and run fresh product, architecture and detailed immutable rereviews on that one SHA.
