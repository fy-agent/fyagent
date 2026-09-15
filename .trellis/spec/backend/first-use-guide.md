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
get_first_use_guide_state() -> "pending" | "dismissed"
dismiss_first_use_guide() -> Result<"dismissed", String>
```

Commands accept no paths, settings snapshots, software IDs or purpose choices.
Register both in `generate_handler!` and the active application permission set.
The Tauri SettingsPort parses unknown responses; a `pending` dismissal response
is an error rather than successful completion.

## 3. Contracts

- Initialize pending before database creation/seeding, only when the database,
  legacy `config.json` and device `settings.json` are confirmed absent. An
  existence-check error is not absence. Existing settings, even malformed,
  must not be overwritten just to initialize this guide.
- No marker on an existing installation means dismissed. Do not infer first
  install from an empty providers table, a missing renderer localStorage key,
  application version or the absence of detected third-party software.
- Existing pending survives restart; dismissed and legacy
  `firstRunNoticeConfirmed: true` suppress the guide. Closing the app without
  finishing or skipping does not manufacture user acknowledgement.
- Dismissal uses the existing settings write lock and persistence owner to
  merge only `firstUseGuideState: dismissed` and
  `firstRunNoticeConfirmed: true`. Persist before changing the in-memory value.
- Ordinary `save_settings` preserves the native-owned guide marker and a true
  legacy acknowledgement against stale renderer snapshots. Purpose choice is
  not persisted or transmitted.
- The device file, not the synced database or renderer cache, is the restart
  authority. Removing all local application data creates a new installation
  context; replacing the app binary while retaining data does not.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Confirmed new local data, no acknowledgement | Persist pending before DB initialization. |
| Existing DB, JSON or settings and no marker | Return dismissed without onboarding initialization writes. |
| Pending with DB now present | Continue pending. |
| Dismissed or legacy confirmed | Do not reopen. |
| Initialization persistence failure | Log safely; do not claim pending was saved. |
| Dismissal persistence failure | Return failure and retain prior in-memory state. |
| Stale ordinary settings save | Preserve guide completion. |
| Unknown wire state or pending dismissal acknowledgement | Reject at renderer adapter. |

## 5. Good / Base / Bad Cases

Good: a completed guide remains dismissed after a settings panel saves an old
snapshot. Base: an interrupted first launch resumes its pending guide. Bad:
checking providers after built-in providers were seeded or submitting the
entire renderer settings object merely to dismiss onboarding.

## 6. Tests Required

`settings/first_use_guide.rs` tests cover eligibility, old settings, legacy
acknowledgement, closed serialization and pending restart. The commands/settings
merge test protects completion from stale saves. Renderer port and ACL tests
cover exact no-argument calls, response rejection and registration/permission
closure. Run Rust format/check/Clippy/tests and renderer type/lint/unit gates.
Browser persistence fixtures are mock evidence, not fresh-install HIL evidence.

## 7. Wrong vs Correct

Wrong: `save_settings({ ...cachedSettings, firstRunNoticeConfirmed: true })`.
Correct: `dismiss_first_use_guide()` merges the two native-owned fields against
the latest locked settings and acknowledges only after persistence.
