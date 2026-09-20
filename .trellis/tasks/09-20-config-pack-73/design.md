# Design

Native strict config-pack/v1 DTO -> native preview store -> atomic SQLite draft import -> real readback. Frontend only invokes a typed port and presents validated DTOs. The service is independent of the live Change Plan adapters, whose current operations always include activation; this import deliberately has no live-file write. SQLite transaction is the recovery owner.

Portable projection allowlists name, app, endpoint, model and Codex wire API. Unknown source settings are omitted, not copied. Credentials and device fields never enter the pack; all imported entries require credentials before later separate activation.

The native preview stores immutable final entries and the full local inventory fingerprint for 10 minutes, at most 16 previews. Conflicts default to skip. Rename/add must be unique per app; overwrite is limited to an unselected, credentialless, pack-created draft. Apply accepts only preview ID and digest. It rechecks expected inventory and local current selections under Provider switch guards and holds the SQLite transaction through readback, so failures roll back the complete batch. No schema migration.

A narrow config_pack DAO is necessary for one atomic multi-row portable-only operation. #35 owner confirmed this integration boundary; its provider_credentials table, when present, protects pending/ready/revoked records from overwrite. This DAO never accepts Provider/metadata/auth input.

Files use native pickers, bounded regular-file reads and create-only export to *.fyagent-config.json. No renderer path authority, archive or network. UI archetype: existing restrained developer tool; reuse semantic surfaces, fonts, Button/Dialog/Checkbox/Input and existing color/border/radius tokens.

Files: new domain/config-pack, native services/config_pack, dao/config_pack, commands/config_pack, typed port, shared ConfigPackDialog and tests; minimal Models header and registration/ACL/facade wiring only.
