# GitHub Issue #35 authority snapshot

Evidence class: `source_report`. Initially captured 2026-08-14 Asia/Shanghai with the authenticated GitHub connector and independently read back with `gh api`; revalidated 2026-08-15 before the next design candidate.

## Source identity

| Object | GitHub time / ID | SHA-256 of `gh --jq .body` stdout (including trailing LF) |
| --- | --- | --- |
| Issue [#35](https://github.com/fy-agent/fyagent/issues/35) | created `2026-08-12T10:36:51Z`; updated `2026-08-13T02:29:27Z` | `27a47682e2bc082e301529869cf77656ae140dbee8823148c3205619fdf5e08b` |
| [comment 5267413157](https://github.com/fy-agent/fyagent/issues/35#issuecomment-5267413157) | created/updated `2026-08-12T13:24:20Z` | `5cb4cb42d66de595a0a7ea5ba2689aba23010f20eb18860e1242ed27e4dd4156` |
| [comment 5272601797](https://github.com/fy-agent/fyagent/issues/35#issuecomment-5272601797) | created/updated `2026-08-12T20:45:56Z` | `f762cfabe2d04112e4f2a359adc5d5a74c32aa81006c7366684098e1ce0ba1f5` |

At readback the issue was OPEN, milestone 1, labels `enhancement` and `priority:P0`, with exactly two comments.

## 2026-08-15 pre-freeze revalidation

The GitHub connector returned the same Issue #35 title/body. Independent authenticated `gh api` readback returned `state=open`, `updated_at=2026-08-13T02:29:27Z`, `comments=2`, milestone 1 and the same two labels. The issue and comment IDs/timestamps were unchanged, and fresh stdout digests (including the trailing LF) remained exactly:

- issue body: `27a47682e2bc082e301529869cf77656ae140dbee8823148c3205619fdf5e08b`;
- comment `5267413157`: `5cb4cb42d66de595a0a7ea5ba2689aba23010f20eb18860e1242ed27e4dd4156`;
- comment `5272601797`: `f762cfabe2d04112e4f2a359adc5d5a74c32aa81006c7366684098e1ce0ba1f5`.

No authority delta was found, so product scope did not reopen for a GitHub-source change. This is source readback only, not design approval or runtime evidence.

## Authority requirements and disposition

| Authority requirement | #35 disposition |
| --- | --- |
| Provider/Agent/future hardware reference one credential without treating value as ordinary DB/log/export/frontend state | Freeze random `secretRef`, generic owner wire model and value-free public surfaces. MVP runtime is Codex Provider; Agent owner is wire-reserved until its stable owner registry exists and must fail typed rather than pretend support. |
| Provider/config stores only ref + non-sensitive state | Device-local non-sensitive secret state/binding authority; Provider DB is scrubbed and never becomes a fallback material store. |
| OS software backend and future hardware share an interface without flattening hardware value | Explicit backend instance/per-record capabilities; physical confirmation, device generation, hardware-only residency, central revocation and persistent-projection prohibition remain observable. |
| Frontend states present/missing/locked/denied/stale/unavailable and never returns/serializes value | Stable owner/ref summaries plus operation-scoped readiness; native capture accepts no value IPC. Revoked is added as an explanatory state while deletion still makes dependencies unusable. |
| Backend unavailable fails closed without SQLite/config/log/cloud fallback | Exact backend-instance lookup; no fallback loop. |
| Add/replace/validate/rotate/lock/delete; rotate new → verify → switch dependencies → delete old | Durable operation journal and binding-set CAS; old binding survives failed verification. |
| External tool plaintext limits visible in Change Plan; FyAgent DB keeps no copy | #55 plan only contains ref/non-sensitive projection; #41 resolves only after approval. Secret-bearing recovery bytes become ref placeholders and are rehydrated only inside controlled restore. |
| macOS/Windows real CRUD; hardware contract + separate device acceptance | Native host evidence remains mandatory; no mock/cross-build substitution. Hardware runtime is out of MVP and its UI selector is hidden when no adapter exists. |
| DB export/diagnostic/crash log/frontend IPC/Workspace Pack scan no original value | Acceptance claim is explicitly `codex_feature_runtime`, enumerated by call graph. Existing unrelated WebDAV/S3/non-Codex credential debt prevents a repository-global claim. New Codex exports/backups are scrubbed; historical/user-owned artifacts are v1 scan/report-only and are never rewritten/deleted by #35. |
| Delete/revoke makes dependencies missing/stale, never connected | Binding is retained for impact/explanation; backend missing, user delete and central revoke have distinct non-sensitive cause/error/audit. |
| No cloud hosting/full cross-device/hardware protocol/commercial product | Remains non-goal. Device-local state is intentionally excluded from DB sync. |
| Windows choose Generic + LocalMachine; macOS non-sync Keychain; one ref per secret | Adopted. Windows entry must set explicit target and `persistence=Local`; no Enterprise default. |

## Review rule

Before design freeze, refresh this report. If issue `updated_at`, comment count, IDs or any digest differs, reopen product review and record the changed requirement disposition.
