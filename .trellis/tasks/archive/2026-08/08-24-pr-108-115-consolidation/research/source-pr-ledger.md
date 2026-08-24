# Source PR ledger

| PR | State at freeze | Keep | Rewrite / exclude | Issue relation |
|---|---|---|---|---|
| #108 | open, clean | Grok Official/xAI naming, four locales, selector/footer tests | Reapply surgically on current trees; do not import unrelated branch history | Closes #107 through replacement PR |
| #109 | closed, unmerged | Provenance only | Entire implementation excluded from replacement | No close action |
| #112 | open, conflicting | Future projectionDigest contract and fail-closed Secret admission conclusion | No SecretRef/Keychain/plaintext fallback/Secret UI or unconsumed secret subsystem | Refs #25–#29, #32, #35, #41, #55 only |
| #113 | open, conflicting | Apply preview/timeline/result presentation, accessibility and honest usage-evidence copy | Remove fake coordinator/runtime/scenarios, cancel, backup, restore and duplicate state machine | Migrated into real Change Plan UI |
| #114 | open, conflicting at freeze; closed unmerged during execution | Single Change Plan ledger, TTL/digest/baseline, once-only writer, append-only events and read-only reconcile | Rebase from schema v16 to v20; reuse existing Provider lock; strict DTOs; separate DB/device baseline | Core replacement implementation |
| #115 | open, conflicting | Four-layer read-only readiness semantics | Remove second catalog, old IDs, ten write IPCs, stores/executor/cancel/fake doctor/helper and unsupported claims | Read-only Agent detail only |
| #130 | opened during execution; clean, 7 commits | #114-derived backend provenance, async IPC offload, lock-held reconcile reread | Exclude schema-coupled v1/HMAC/process epoch, second lock, single-current baseline, weak event/DAO contracts; supersede after replacement CI | Derived replacement of #114; no new Issue close syntax |

## Replacement PR issue syntax

- `Closes #107`
- `Refs #25 #26 #27 #28 #29 #32 #35 #41 #55`

No other Issue is auto-closed by the replacement PR.

## Old PR close gate

Do not close any currently open source or derived PR until the replacement PR exists and its initial `CI / Required` is green. Each close comment must name the kept result, excluded result, replacement location and reason for superseding. Then read back `closed + merged=false`. PR #114 is already closed unmerged; add only a provenance comment after the gate. PR #130 is included because leaving its partial replacement open would violate the single-replacement objective.
