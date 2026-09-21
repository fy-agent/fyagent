# Export/import compatibility review

This is the proposed #35 behavior; verification results belong in final validation evidence.

## Ordinary SQL / sync export

Allowed: app/id/name identity; whitelisted Provider model identifiers, routing
shape, HTTP(S) primary base URLs (no userinfo/query/fragment), wire API and known
feature booleans. All apps use the same pure whitelist; unknown scalar extensions
are omitted. Known secret value collisions in identity cause a bounded export
error rather than exporting a modified identity. Native store is never read.

Omitted: auth/API keys/SecretRef/credential ID; arbitrary env/header values;
usage scripts and metadata; notes/website/icon; endpoint history; full profiles,
universal provider source and common-config snapshots. Exported Provider current
and failover flags are false. Non-Provider arbitrary user tables/content are not
given a whole-database no-secret guarantee by this package.

Private binary backups keep the existing lossless representation and must stay
private. Reference-only rows in such backups still depend on the same device's
native key availability; the backup is not a native credential export.

## SQL / sync import

- New ordinary exports create inactive credentialless drafts, without writing
  Codex live files or changing the local settings current pointer.
- Device-local credential records are preserved for both ordinary and sync
  imports. A local Provider containing credentials keeps its complete existing
  route and endpoint list; remote changes never receive its key implicitly.
- Legacy import compatibility remains: unbound plaintext rows can be imported;
  startup or the next canonical Provider save attempts lossless migration.
- Existing trigger/unsafe SQL rejection remains. Imported SQL cannot inject a
  credential ledger over the destination's native ownership records.

## Test cases

- Codex old plaintext + reference-only rows: ordinary/sync output lacks canary/ref.
- Claude, Gemini, OpenCode, OpenClaw, Hermes, GrokBuild, Claude Desktop shapes:
  API key, headers, unknown extensions, TOML comments and snapshot canaries absent.
- Snapshot sanitizer disables triggers before updating rows.
- Sync retains a local bound Provider when remote exports the same ID with a
  different endpoint; native resolution still returns local material.
- Fresh ordinary import is a credentialless inactive draft and leaves live bytes
  unchanged.
