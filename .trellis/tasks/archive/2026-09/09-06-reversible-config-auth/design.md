# Design

## Change boundary

Extend the existing `config` file owner with backup-by-default mutation and explicit recovery operations. Keep the low-level atomic replacement private to the implementation/compensation path. Shared serializers and direct Codex/OpenCode auth writers consume this owner, preventing a caller from forgetting the backup. Keep one adjacent `.fyagent.backup` exact preimage and bounded, credential-free local recovery metadata. Atomic temporary files must be restrictive before bytes are written; flush/sync and permission failures are errors, not best effort.

Recovery verifies the recorded postimage and backup before touching a primary file. It restores exact bytes, or removes a newly created file, and refuses external changes. Backend-only closed target resolution exposes bounded path/backup labels but never accepts user-controlled paths. Feature writers preserve their existing serialization/domain locks. Multi-file writers retain all-or-compensate behavior and never claim filesystem-wide ACID.

Managed Auth separates admission/impact preview from applying a connection. Preview describes only the files the action can actually touch and binds to the displayed revision. The existing confirmation dialog loads that preview before enabling confirmation. Codex connects/switches only `auth.json`; explicit request-source selection stays in Provider/Change Plan. Login stores a purpose-compatible credential without automatic file projection. Official source projection must use lossless targeted TOML changes rather than a full snapshot.

## Reuse and risks

Reuse serde, SHA-256, existing atomic replacement, Provider guard, auth writer locks, typed ports, Query and shared Dialog/notice controls. Backup orchestration and disclosure are product contracts, not a new filesystem or OAuth library. Do not broaden native path authority. A persisted recovery marker is not proof a third-party process picked up credentials. Windows native behavior needs matching-host evidence; macOS tests cover only their declared scope.
