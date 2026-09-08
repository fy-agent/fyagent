# Codex Request-Source Selection

## 1. Scope / Trigger

Read before changing the top-level `model_provider` selector or the pure
projection of a saved Provider into live Codex TOML. Two callers share this
configuration logic but have different write permissions:

- [Managed Auth Consumers](./managed-auth-consumers.md) connects an official
  account and may comment the selector, or disconnects and restores it.
- [Codex Provider Configuration](./codex-provider-configuration.md) selects a
  saved request source and patches its owned model/provider fields. Its writer
  and Change Plan remain config-only.

Owners are `src-tauri/src/codex_config/model_provider_line.rs` and
`src-tauri/src/codex_config/source_switch.rs`. This contract does not own
credentials, consumer status, file recovery, or Provider persistence.

## 2. Signatures

```rust
comment_top_level_model_provider(config: &str) -> Option<String>
uncomment_top_level_model_provider(config: &str) -> Option<String>
top_level_model_provider_is_active(config: &str) -> bool

validate_codex_source_config(
    category: Option<&str>, auth: &serde_json::Value, desired_config: &str,
) -> Result<(), AppError>
patch_codex_source_config(
    current_config: &str, category: Option<&str>, auth: &serde_json::Value,
    desired_config: &str, config_dir: &Path, unify_sessions: bool,
) -> Result<String, AppError>
```

The last two names are the `codex_config.rs` re-exports of `validate_source`
and `patch_source`. These are native helpers, not renderer commands. Paths,
auth bytes and full TOML never become new IPC arguments through this boundary.

## 3. Contracts

### Selector edit versus complete operation

The line helper changes the first matching bare-key `model_provider = ...`
assignment before the first table header. Comment inserts `#` after the
indentation; uncomment removes one leading `#` and following whitespace.
It retains line endings, the assignment value, trailing comments and other
lines. `None` means this helper found no matching edit, not that the caller's
whole operation is a no-op. The active-selector predicate is false for both
a commented selector and no recognized selector.

This is a narrow lexical helper, not a general TOML parser or revision guard.
Do not infer support for quoted keys, multiline-string content or ambiguous
multiple selectors from these helpers. Source projection separately parses
complete TOML with `toml_edit`; account projection retains its own observation
and admission. Extending the lexical grammar requires parser-aware validation
and preservation tests, not a broader text replacement.

| Operation | Permitted effect |
| --- | --- |
| Official account connect/switch | Compute the auth delta separately; comment an active selector to use the built-in default route. Do not edit provider tables, model, MCP or features. |
| Codex disconnect / restore source | Uncomment the recognized saved selector when present; keep `auth.json`. Connection metadata and restart evidence belong to the consumer. |
| Saved Provider/source selection | Comment/uncomment as preparation, then patch the selected source's owned TOML fields. Preserve `auth.json` and unrelated configuration. |

An auth delta of `Noop` can therefore still produce a config write for a
matching official account with an active third-party selector. Conversely,
switching a saved request source does not replace an official account.

### Targeted Provider projection

`source_switch.rs` validates the desired source before catalog/config side
effects, edits the existing selector, then parses and patches the live
document. It is not limited to toggling one comment:

- Root source-owned fields are `model_provider`, `model`,
  `experimental_bearer_token`, `base_url` and `wire_api`. A missing desired
  field removes that owned value while retaining attached user comments.
- Optional model choices (`review_model`, reasoning effort/summary, verbosity,
  context/compaction limits and `web_search`) change only when present in the
  desired source. `model_catalog_json` is removed on absence only when its
  path is recognized as FyAgent-owned.
- Replace only the selected custom provider table. Preserve other provider
  tables, MCP, features, profiles and the credential-store choice. Do not claim
  byte-for-byte preservation of the selected table that this operation owns.
- An empty official template uses the default route. An explicit compatible
  official template may supply a selector or native-capability table; the
  unified-session projection retains its existing ownership checks. Do not
  apply the account operation's selector-only restriction to this writer.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Matching auth plus active selector | Auth bytes can remain unchanged while the selector is commented; not a whole-operation no-op. |
| Selector already commented or absent | Comment helper returns `None`; independently evaluate any auth/source changes. |
| Selector inside a provider/MCP/other table | The line helper does not edit it. |
| Desired TOML invalid or present model/selector is not nonempty text | Reject source projection before file side effects. |
| Official desired source explicitly selects an incompatible auth route | Reject; a display name alone is not official-source evidence. |
| Custom third-party source lacks its selected table or supported authentication | Reject; do not replace the live file with an empty template. |
| Nonempty built-in-default configuration has an API key | Validate through the existing built-in source path; do not invent a custom-table requirement. |
| Live TOML cannot be parsed | Reject Provider projection; never reconstruct unrelated user settings from a minimal form. |

Custom source admission accepts the existing API-key, native-auth, `env_key`
or auth-table alternatives. Reserved built-in routes retain their own auth
contract; this helper does not invent a bearer-key requirement for them.

## 5. Good / Base / Bad Cases

Good: an official account already matches, so its auth bytes stay intact while
`model_provider = 'custom'` becomes `#model_provider = 'custom'`; provider and
MCP tables remain unchanged. Base: a saved third-party source restores the
selector and patches its selected model/table without a second selector.
Bad: describe both operations as either “auth-only” or “comment-only”, replace
the entire live TOML, or treat a pure patch result as proof of a successful
file write, connected account or running-process pickup.

## 6. Tests Required

Run `mise run rust:test -- model_provider_line` and
`mise run rust:test -- source_switch`. Their tests assert table isolation,
LF/CRLF preservation, comment/uncomment round trips, no duplicate selector,
desired-source rejection, and preservation of unrelated live configuration.

`consumers/codex/project.rs` tests independently prove matching-account config
changes without auth rotation, already-commented no-op, and stale auth/config
revision rejection without writes. Consumer tests own backup/readback and
restart outcomes; a lexical unit test proves none of those effects.

## 7. Wrong vs Correct

Wrong: `delta == Noop` means no file can change; every official source operation
must only comment one line; `patch_source` returning text proves connection.

Correct: evaluate auth and selector deltas independently, select the caller's
bounded write set, validate the complete desired/live source where required,
then let the existing native writer, readback and consumer observer determine
the operation's outcome.
