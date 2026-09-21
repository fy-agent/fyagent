# Provider SecretRef production integration (#35)

## Goal

Connect the existing native SecretRef store to Codex Provider API-key saves,
reads, updates and deletion. Migrate existing Provider keys without damaging a
working configuration when native storage or persistence fails.

## Requirements

- Codex create, edit, source switch, backfill and native proxy/usage consumers
  share one credential owner. Missing, locked, foreign and revoked refs fail closed.
- Blank/masked edits retain the current binding; explicit replacement rotates it.
- Durable create admission precedes backend I/O; interrupted migration preserves
  exact legacy material and can resume without creating another native item.
- Logs, errors and renderer projections contain no key. Ordinary SQL exports
  whitelist Provider connection fields for all supported app shapes; binary
  backups remain private recovery data.
- Show Codex's plaintext config.toml projection and backup implications at save.
- Keep the SecretBackend hardware extension unchanged. No real secrets or
  production configurations may be read during implementation/testing.

## Acceptance

- [ ] Save/read/blank-edit/rotation/missing/locked/foreign-ref/delete regressions pass.
- [ ] Migration interruption and DB failure preserve old material; retry succeeds.
- [ ] Native temporary-file create/edit/switch/delete preserves auth.json and unrelated settings.
- [ ] Schema fresh/v23/rollback, export-negative and local sync-binding tests pass.
- [ ] Renderer blank-edit validation and file disclosure pass.
- [ ] Formatter, focused Rust/renderer checks and code-spec updates complete.

Native OS credential-store HIL, real-account UAT, Windows acceptance and release
acceptance are separate evidence; this package uses explicit fixture backends.
