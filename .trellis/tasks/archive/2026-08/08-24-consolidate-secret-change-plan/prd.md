# Secret and Change Plan consolidation

## Goal

Implement the canonical Change Plan domain from #114 and the fail-closed Secret admission conclusion from #112, without touching shared schema/command/lib registration.

## Requirements

- Add a single Change Plan domain/service/DAO with a 15-minute immutable plan, append-only job events, separate DB/device baselines and read-only reconcile.
- Create Plan performs no network or Provider mutation. Apply reuses the existing Provider mutation lock and calls the existing writer at most once.
- Only existing Codex Provider switches proven to require no new credential material are admitted; otherwise return `secret_dependency_unavailable` and write nothing.
- Store and serialize no API key, raw settings/live config, SecretRef, absolute path or credential-derived value.
- Pure lexical display-name sanitization works identically on every host.
- Do not add shared schema version, Tauri command registration, ACL, FeaturePorts or Git changes.

## Acceptance Criteria

- [x] Domain and DAO compile independently after integration wiring.
- [x] Create Plan has zero business side effects.
- [x] Normal Apply writer count is exactly one; invalid/expired/replay/stale/secret blocked count is zero.
- [x] Concurrent Apply permits one consumer only.
- [x] Mixed/unknown readback yields `recovery_required`; reconcile never replays writer.
- [x] Serialization and persistence contain no secret material.
- [x] Windows/Unix/file URI/control/Unicode/length sanitization matrix passes.

## Notes

- Source PRs: #112 and #114. Schema and shared registration are integration-worker ownership.
