# Design notes

The integration base is `codex/ucp-integration-35-41`: exact #41/#56 head plus
the four commits from Draft PR #132. This is a dependency composition branch,
not a `main` merge.

The UCP public operation becomes `codex_provider_upsert_and_switch`. Its
business-step list is derived exhaustively from the operation, while executor
phases remain the five stable shared phases. Safe credential projection is
persisted in schema v20; raw material and desired transient live Provider stay
inside the process-private proof map and disappear on restart.

Provider mutation keeps one owner. A new lock-held quick-setup entry point uses
a sanitized Provider for SQLite and a transient materialized Provider for
Codex/native projection. It reuses the current snapshot, atomic write,
readback, compensation and mutation lock rather than introducing a second
writer.
