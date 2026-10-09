# Retired Legacy Cloud Sync

## Scope and decision

The 2026-10-07 approved Sprint decision retires WebDAV/S3 cloud sync.
The former scheduling contract is superseded; do not recreate transport
commands, workers, configuration editing or connection probes.

## Preservation boundary

- Startup registers no cloud database listener and spawns no cloud worker.
- No WebDAV/S3 cloud command is exported, handled or permitted.
- Legacy settings are opaque JSON values. Do not validate, normalize or use
  enabled/autoSync, credentials, remote locations or status fields.
- Renderer settings omit all legacy cloud values. A general settings save
  ignores incoming legacy values and preserves the latest native-owned
  values, including unknown nested fields and credentials.
- Retirement performs no settings-file rewrite, migration, deletion or remote
  operation. Ordinary later settings saves preserve opaque legacy values;
  they do not promise byte-identical formatting of the whole settings file.
- Preserve local DB export/import/backup/restore and connection-local listener
  tests. Their persistence and recovery contracts remain independently owned.

## Validation

Synthetic legacy enabled/autoSync configurations must roundtrip opaquely.
Stale or malicious renderer payloads must not replace preserved legacy values.
Architecture tests must prove commands, ACL, startup and service modules no
longer reach cloud transports. Native startup evidence and whole-file byte
preservation at startup remain separate from code audits and synthetic tests.
