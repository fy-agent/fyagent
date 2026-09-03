# Codex official-auth writer inventory

> Captured 2026-09-03 from a read-only scan of `dev/laiyongjie`.
> Preserve is still an optional setting (default **false**). There is **no**
> OS keyring writer; `cli_auth_credentials_store` does not stop third-party
> `auth.json` overwrites.

Hub in `write_codex_live_for_provider`:

```text
src-tauri/src/codex_config.rs:734-754
should_write_auth =
  (official && has login material)
  || (not official && !preserve_codex_official_auth_on_switch())
  && (!oauth_native_projection || native_file_store)
```

Third-party branch sets `oauth_native_projection=false`, so keyring/auto/unset
still overwrite `~/.codex/auth.json` when preserve is false.

## Must-change writers (third-party switch)

| Location | Preserve? | Required invariant |
| --- | --- | --- |
| `codex_config.rs:734-737` hub | optional | Drop `\|\| (category != official && !preserve)`. Only official+login+file-store may write auth. |
| `services/provider/live.rs:791,853` Quick Setup | independent | Always `should_write_auth = false` (Quick Setup is third-party). |
| `services/provider/mod.rs:4184` `quick_setup_write_targets` | yes | Never `push(get_codex_auth_path())`. |
| `services/proxy.rs:2894` placeholder path | half | Placeholders always config-only; remove preserve shell. |
| `services/proxy.rs:2978` `write_codex_live_verbatim` | **no (bug)** | Non-official restore/cleanup must be config-only even if `auth` is non-empty. |
| `services/proxy.rs:2111` `cleanup_codex_takeover_placeholders_in_live` | no | Use `write_codex_live_config_atomic`, not full live rewrite. |

Indirect callers of the hub (`write_codex_provider_live_with_catalog`,
`write_live_snapshot`, `sync_codex_live`, takeover fallback) are fixed once
`:734` is hard-preserve. Takeover official/placeholder path is already
config-only.

## Official switch (keep)

- Official+login may still write `auth.json` when store is `file`.
- `clear_stale_codex_live_auth_after_official_switch` (`codex_config/auth.rs:112`)
  deletes third-party-only auth after switching to official without login
  material. Keep, but do not use it as a third-party switch tool.

## V1 toggle to remove/disable

- `src/components/settings/CodexAuthSettings.tsx:98-106`
- Mounted on Settings **General**, not Auth Center
- Default false: `settings.rs:521`, `useSettingsForm.ts`

Read old JSON for compatibility; behavior is always preserve. Do not add the
toggle to V2.

## Tests that currently require overwrite (must invert)

- `services/proxy.rs:4988` `codex_custom_provider_live_write_can_overwrite_auth_when_preserve_disabled`
- `tests/provider_service.rs:712` `provider_service_switch_codex_default_overwrites_official_auth_when_preservation_off`
- `tests/provider_service.rs:800` bearer token still writes `OPENAI_API_KEY` into live auth.json
- `services/provider/mod.rs:1805` Quick Setup asserts `live_auth["OPENAI_API_KEY"]=="new-key"`
- `services/provider/mod.rs:1682` concurrent Quick Setup asserts live auth key

Already-preserve tests should become the default. Official-to-official file
store writes remain allowed
(`provider_service_switch_codex_official_accounts_write_auth_json`).

## Keyring

No keyring crate / OS vault Codex writer exists. Do not pretend `auto` or
`keyring` can be silently implemented as file writes.
