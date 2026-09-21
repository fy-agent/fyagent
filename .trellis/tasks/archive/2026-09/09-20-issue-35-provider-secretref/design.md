# Design

## Change boundary

Codex Provider API keys currently persist in `settings_config.auth` and sometimes
the TOML bearer field. Reuse the existing SecretBackend/SecretService for these
keys. Preserve the Provider save signature as a compatibility facade owned by
the Provider service; the DAO retains only SQL writes. Other Provider kinds keep
their current behavior. No navigation, version, proxy-recovery, vendor-preset,
managed-OAuth or hardware implementation changes belong here.

## Data and ordering

- A local-only credential table durably admits opaque handles before native
  creation. Provider JSON stores an opaque credential ID, never native locators.
- Reserve -> admit pending record -> native create/readback -> reference-only
  Provider save. Old rows remain recoverable until the whole operation commits.
- Resolving a reference checks provider ownership and the exact current binding;
  missing, locked, denied and revoked credentials never fall back to live/env.
- Blank/masked edits retain the existing binding; replacing a key allocates a new
  handle so rollback can still select the previous one.
- Legacy plaintext is read compatibly and migrated on write/startup. Failed
  migration keeps the original row and external files unchanged.
- Native projection resolves only at existing writer/proxy boundaries. Renderer
  DTOs and ordinary exports scrub all Codex credential-bearing fields. External
  Codex config remains a disclosed plaintext projection with private backups.

## Ownership and checks

Affected: Provider credentials owner, provider DAO/schema/composition, native
writer/router seams, command redaction, export sanitization and Models disclosure.
Use isolated memory backend and temporary filesystem fixtures; no real accounts.
Run canonical mise bootstrap, focused Rust tests and renderer/contract checks.
Native keyring HIL, external Codex process pickup and Windows acceptance remain
separate evidence and are not inferred from these tests.
