# Recovery ownership

Reuse the existing rolling backup, path-bound receipt, atomic writer and per-target proxy lock. Admission must explain the complete current projection from the saved backup/provider, and the actual file writer must reject a changed postimage at publication. Do not infer ownership from a loopback URL or placeholder alone. Historical backup data retains its existing format.

Managed restoration accepts each file's owned postimage or exact preimage only in the exit path. Activation/rebinding retains strict postimage admission. This allows retry after partial restoration without authorizing unrelated edits.

Codex native auth is never a proxy-owned restoration target. Keep its current bytes. File recovery remains a separate explicit user operation and cannot clear proxy state automatically. Existing native readback remains the source of truth for completion.

Ordinary direct API proxy controls are not exposed by the current renderer. Managed subscription binding, OpenCode subscription restoration and the shared FileRecoveryButton are the actual UI surfaces to assess. Avoid creating a new settings surface solely for historical native commands.
