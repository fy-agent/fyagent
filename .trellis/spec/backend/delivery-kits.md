# Delivery Kit Native Contract

## Scope and owners

`services/delivery_kits/{schema,validator,mod}.rs` owns text packages, strict
validation, local immutable storage and the weekly-report validator.
`commands/delivery_kits.rs` owns eight narrow commands and trusted file pickers;
`permissions/delivery-kits.toml` admits them only in the main capability.
Projects own binding transactions; evidence owns persistent verification.
There is no database migration, global Agent projection, credential access,
network request, shell/SQL executor, or background job in this owner.

## Contract

- `.fyagent-kit.json`, exact `fyagent-delivery-kit/v1`; Rust structs use
  camelCase and deny unknown fields. `UniqueValue` rejects duplicate members
  recursively before typed decoding, including JSON resources. Unknown enum,
  reference, resource hash, identity or SemVer fails closed with `KitError`.
- Input is at most 4 MiB, depth 32, each text at most 256 KiB, ID collections
  at most 100. IDs are lowercase ASCII letters/digits/hyphen, begin with a
  letter, at most 64 characters. Stable versions have no prerelease/build tag.
- Resource types are Markdown/JSON text only; no locator, script, env, header,
  activation or executable field exists. Text is untrusted data, never executed.
- SHA-256 covers recursively sorted object keys, original array order and
  compact UTF-8 JSON. Resource SHA covers exact text bytes. The shared golden
  fixture is `tests/fixtures/deliveryKitContract.v1.json`. Hashes prove content
  identity, not publisher authenticity.
- `KitIdentity { kitId, kitVersion, manifestDigest }` travels across boundaries.
  Neither native paths, credential slots' values nor SecretRefs travel in it.
- Native library root is the app config directory's `delivery-kits/`. Files are
  `<id>--<version>.json`, holding canonical manifests. Native construction and
  ancestor/symlink checks own paths; renderer cannot supply a filesystem path.
  The flat immutable file scheme replaces the early nested-directory proposal.
- Preview holds bounded bytes in memory, at most 16 previews, monotonic TTL ten
  minutes. No library/external writes. Catalogue reads also cap file count at 100 and aggregate package bytes at 64 MiB. Cancellation removes the preview;
  restart invalidates it. Apply accepts only preview ID + matching digest.
- Applying a compatible package stages a private file, syncs it and publishes
  create-only via `persist_noclobber`; same id/version/digest replays, changed
  digest conflicts. Readback must match. Concurrent processes cannot overwrite
  another version's file. No destructive delete/update API is exposed.
- Library root ownership is application-level, not an OS process sandbox.
  Same-user hostile parent-directory replacement is not an OS isolation claim.
  Picker paths reject symlink ancestors; exports use create-only files, never
  overwrite an existing selected path. Temporary incomplete writes are not
  published packages. Read errors are explicit, not silently skipped rows.

## Sharing and validation evidence

Exact built-in content and strictly validated, already imported immutable
versions can be exported after an explicit content preview. Both preview and
save resolve the exact native identity; a missing or changed imported version
cannot be exported using an old preview. Export contains only the canonical
manifest, never attached project state or local credential references. Imported
text retains its unverified origin and requires the user to confirm that it is
safe to share. A conservative secret-pattern check rejects known forms; it is
not an anonymization guarantee. An imported synthetic label does not prove
provenance. Only exact built-in content is admitted to `local_fixture` machine
checks. No automatic upstream package download/pinning is implied by a recipe ID.

`KitLibrary::confirm_identity` reads the exact native id/version/digest and
checks host compatibility without writes. Project binding adapters use this
authority instead of trusting renderer catalogue fields.

`run_delivery_kit_demo` runs the native validator, independently totals bounded
integer cents, computes rounded basis points and reports source row IDs.
Missing fields, duplicate rows, period/currency mismatch, missing period,
invalid amount and zero previous total are closed business failures. Expected
data are compared with computed results, never substituted as results.
Only `weekly_report` has a machine validator in v1; other packages are explicit
manual-review starter kits. Results carry kit identity, validator version,
input digests, UTC check time and `sourceClass=local_fixture`, not project or
customer acceptance. Root's evidence adapter must re-read native project
dependencies and call this service; it must not accept renderer-calculated facts.

## Required checks

Focused Rust `delivery_kits` tests cover immutable round-trip/restart/concurrency,
strict/duplicate input, path/secret/script rejection, preview cancellation/TTL,
compatibility/conflict, imported sharing round-trip and native identity
confirmation without writes, built-in-only machine checks, golden digest and real
fixture calculations with expected-output tampering. All files live in temporary
directories. Test results are host-native service evidence, not desktop picker,
Windows, online MCP, or customer acceptance evidence.
