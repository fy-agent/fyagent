# Design

The integration keeps existing owners: `config` owns file replacement/backup primitives; Managed Auth owns account and connection operations; Provider/Change Plan owns explicitly selected model routes; Tooling/Agent lifecycle owns CLI installation; Agent Auth owns Claude's official CLI handoff.

The two children are independently implemented and checked. The safety child establishes the backing-file contract first. The Claude child reuses it only for FyAgent-managed configuration; an official CLI's own credential/keychain behavior is disclosed as vendor-owned rather than falsely promised as a FyAgent undo transaction.

Credential state and request route are independent. The unsafe current Codex coordinator calls the whole Provider switch from an auth action. Remove that coupling. Source changes remain separately previewed operations and must preserve unowned TOML, not serialize a minimum provider snapshot over the live file.

No new dependency is planned: reuse the existing atomic writer, hashing, serde, toml_edit, npm registry policy, subprocess boundary and shared UI/dialog/query owners. Backup/undo metadata contains no credential bytes in IPC. Native paths are display-only, resolved by Rust; the renderer never supplies paths to write/restore.

Rollback is by normal commits on the isolated branch. Do not merge into or clean another checkout. Product recovery restores only the retained immediate preimage after current-state verification; external drift is a refusal, not permission to overwrite.
