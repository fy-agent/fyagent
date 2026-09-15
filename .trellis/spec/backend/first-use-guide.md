# Device-local First-use Guide

## 1. Scope / Trigger

Read before changing first-install eligibility, guide persistence or its Tauri
commands. `settings/first_use_guide.rs` owns the state; `lib.rs` supplies startup
data-directory evidence. [Renderer First-use Guide](../frontend/first-use-guide.md)
owns presentation and recommendations. This is not software installation,
authentication, model configuration, telemetry or a database migration.

## 2. Signatures

```text
settings.json: firstUseGuideState?: "pending" | "dismissed"
settings.json: firstRunNoticeConfirmed?: boolean  # legacy compatibility
get_first_use_guide_state() -> "pending" | "dismissed"
dismiss_first_use_guide() -> Result<"dismissed", String>
```

Commands accept no paths, settings snapshots, software IDs or purpose choices.
Register both in `generate_handler!` and the active application permission set.
The Tauri SettingsPort parses unknown responses; a `pending` dismissal response
is an error rather than successful completion.

## 3. Contracts

### Admission and restart authority

- Initialize pending before database creation/seeding, only when the database,
  legacy `config.json` and device `settings.json` are confirmed absent. An
  existence-check error is not absence. A present, malformed or unreadable
  settings file is an existing/unknown installation and must not be overwritten
  just to initialize this guide.
- No marker on an existing installation means dismissed. Do not infer first
  install from an empty providers table, a missing renderer localStorage key,
  application version or the absence of detected third-party software.
- Existing pending survives restart; dismissed and legacy
  `firstRunNoticeConfirmed: true` suppress the guide. Closing the app without
  finishing or skipping does not manufacture user acknowledgement.
- The device file, not the synced database or renderer cache, is the restart
  authority. Removing all local application data creates a new installation
  context; replacing the app binary while retaining data does not.

### Mutation authority

- Dismissal uses the existing settings write lock and persistence owner to
  merge only `firstUseGuideState: dismissed` and
  `firstRunNoticeConfirmed: true`. Persist before changing the in-memory value.
- Both fields are native-owned for this workflow. Ordinary `save_settings`
  preserves their latest locked values exactly: a stale payload cannot reopen
  dismissed state, and an arbitrary payload cannot dismiss pending state through
  either the current marker or the legacy acknowledgement. Returning those
  fields in a compatibility settings snapshot does not grant write authority.
- `dismiss_first_use_guide` is the only Renderer-reachable transition from
  pending to dismissed. It is idempotent for an already dismissed installation.
  Purpose choice is component-local and is never persisted or transmitted.

## 4. Validation & Error Matrix

| Condition                                                                 | Result                                                           |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Confirmed new local data, no acknowledgement                              | Persist pending before DB initialization.                        |
| Existing DB, JSON or settings and no marker                               | Return dismissed without onboarding initialization writes.       |
| Settings exists but is malformed/unreadable, or any existence check fails | Treat as existing/unknown; do not create pending.                |
| Pending with DB now present                                               | Continue pending.                                                |
| Dismissed or legacy confirmed                                             | Do not reopen.                                                   |
| Initialization persistence failure                                        | Log safely; do not claim pending was saved.                      |
| Dismissal persistence failure                                             | Return failure and retain prior in-memory state.                 |
| Ordinary settings payload changes either first-use field                  | Ignore both incoming values and preserve the latest locked pair. |
| Unknown wire state or pending dismissal acknowledgement                   | Reject at renderer adapter.                                      |

## 5. Good / Base / Bad Cases

Good: a completed guide remains dismissed after a settings panel saves an old
snapshot, while a pending guide cannot be acknowledged through that same generic
save path. Base: an interrupted first launch resumes its pending guide. Bad:
checking providers after built-in providers were seeded or submitting the
entire renderer settings object merely to dismiss onboarding.

## 6. Tests Required

`settings/first_use_guide.rs` tests cover the all-absence admission matrix,
existing markers, legacy acknowledgement, closed serialization and pending
restart. The commands/settings merge test protects both directions: stale input
cannot reopen completion or acknowledge pending state. Renderer port and ACL
tests cover exact no-argument calls, response rejection and
registration/permission closure. Run Rust format/check/Clippy/tests and Renderer
type/lint/unit gates. Browser persistence fixtures are mock evidence, not
fresh-install HIL evidence.

## 7. Wrong vs Correct

Wrong: `save_settings({ ...cachedSettings, firstRunNoticeConfirmed: true })` or
trusting an incoming `firstUseGuideState`. Correct: ordinary settings saves copy
both fields from the latest locked state; `dismiss_first_use_guide()` alone
merges the completed pair and acknowledges only after persistence.
