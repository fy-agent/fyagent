# Auth and proxy acceptance review: #42 / #43 / #64

Reviewed 2026-09-21 against integration HEAD `07519b37` plus the shared tree's
current changes. Scope: the original completion conditions in
`research/issues-initial.json`, auth commit `94ab3244`, proxy commit `e18d9501`,
and their existing production entry points. This was a bounded source and test
assertion review, with no external research, product edits or test execution.
The root agent owns final combined compilation and runtime validation.

## Decision

- **#42: retain pending acceptance.** The implemented per-connection preview,
  result retention and fresh retry are real, and existing native subscription
  tests exercise supported Grok subscription paths. However, the missing
  target-specific proxy exit entry below also leaves this Issue's independent
  recovery condition incomplete for Claude/Grok subscription connections.
- **#43: no new code blocker found in the reviewed paths.** The three Grok/xAI
  concepts are named separately, unsupported direct credential projection is
  withheld, and official CLI actions remain explicitly unverified handoffs.
  Final integrated tests remain the root's responsibility; source/mocks are
  not proof of a real vendor login, expiry, logout or keyring behavior.
- **#64: blocking UI completion gap confirmed.** Owned restore checks are
  substantially covered, but only OpenCode exposes the actual target-specific
  proxy exit command in the current renderer. Generic file undo does not clear
  proxy takeover state and is not a substitute.

## Blocking finding: target-specific proxy exit is absent for Provider targets

**Priority: P1. Conditions affected: #64 current supported targets can exit;
#42 each target has its own recovery.**

Evidence chain:

1. `src/pages/models/Page.tsx::ProviderPanel` renders
   `XaiSubscriptionSection` for Claude, Codex and Grok Build. The subscription
   section's `confirmBind` invokes `providers.bindManagedProxy`; Claude/Grok
   activate, while Codex saves a draft and hands off to the existing source
   Change Plan workspace.
2. `src/shared/platform/tauri/feature-ports/managedSubscriptions.ts` exposes
   `restoreManagedProxy` only on `managedOpenCodePorts`. It invokes
   `set_proxy_takeover_for_app` with `appType: "opencode", enabled: false`.
   A renderer search found no equivalent Provider-target exit call and no
   renderer call to aggregate `stop_proxy_with_restore`.
3. `src/pages/models/OpenCodeSubscriptionRestore.tsx::restore` has an actual
   confirmation, native exit mutation, OpenCode snapshot and managed-auth
   reread, uncertainty handling and file/backup conflict instructions.
4. ProviderPanel instead offers `FileRecoveryButton`.
   `src-tauri/src/services/config/recovery.rs::ConfigService::restore_file_recovery`
   takes the app mutation lock and calls `restore_at`; it does **not** clear
   `proxy_config.enabled`, delete the app's proxy backup, or update listener
   ownership. Those operations belong to
   `src-tauri/src/services/proxy.rs::set_takeover_for_app`'s disable branch.
   File undo therefore cannot be presented as exiting the proxy.

Minimal follow-up agreed with root: reuse the OpenCode confirmation/recovery
presentation pattern in a Provider subscription restore control, integrated
inside `XaiSubscriptionSection`; closed typed target payload; native exit;
target-specific proxy state + actual Provider live summary + managed-auth
reread; independent target results and explicit retry after uncertainty. Do not
edit `Page.tsx` or create a second recovery executor.

**Native disclosure dependency:**
`src-tauri/src/services/proxy/managed_recovery.rs::paths` includes both Codex
`config.toml` and the model catalog. The section currently receives only root
Quick Setup `writeTargets`, normally the main config file. Per-source metadata
also depends on the Provider snapshot and does not prove the current proxy
backup's restore set. A native-owned read-only restore preview must disclose
the actual restoration targets before this UI can be considered complete.
Do not fabricate the catalog/auth paths or treat generic receipt listing as the
proxy backup's exact restoration plan. Root was notified of this small native
surface dependency before implementation.

## #42 condition-by-condition evidence

| Completion condition                           | Actual implementation and assertions                                                                                                                                                                                                                                                                                                                                                                                                                                    | Assessment                                                                                                                         |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Offer only targets with a real writer          | `managed_auth/connection_actions.rs::observe_connection_action` checks current revision, account health/provider, credential purpose and returned allowed actions. `consumers/grok.rs` exposes refresh only while native projection gates are false; `providers/xai.rs::start_xai_login` rejects unsupported consumer-purpose Grok login. Auth Page tests explicitly reject a Grok connection/device-code reuse button.                                                 | Supported within the native capability boundary; no fabricated direct Grok login writer.                                           |
| Independent preview/save/readback/recovery     | `ConnectionActionPreviews::issue/consume` binds request, one-use UUID, lifetime and resolved paths. `apply_connection_action` re-observes revision and paths before the consumer owner. `MutationDialogs` uses a fresh preview. `AuthPage::runMutation` records each connection ID independently and consumes the returned overview. `ConnectionResults` exposes target-specific retry and file recovery for Codex/OpenCode.                                            | Real implementation, with the subscription-exit gap above.                                                                         |
| Same Grok source reaches supported save paths  | `XaiSubscriptionSection` targets Claude/Codex/Grok Build plus its OpenCode variant, using the typed managed subscription ports. Codex deliberately remains draft → source preview/apply. `subscription_tests.rs::verify_subscription_cli_roundtrip` seeds an xAI vault source, writes temporary Claude/Codex/Grok configs, exercises the real bind/service path and preserves native auth. `subscription_opencode_tests.rs` covers OpenCode revision and restore paths. | Existing production paths and native test bodies support the scope. A vault fixture is not a real vendor entitlement/login result. |
| A failed target preserves others and can retry | `Page.test.tsx::preserves target A success when B fails and retries B with a fresh preview` really performs A, makes B throw `external_change_detected`, verifies A remains, retries B with a different preview, and asserts A was invoked once. `subscription_tests.rs::check_concurrent_targets` injects A compensation while B waits, asserts A's config/rows/flags restored and B's listener/current source still committed.                                        | Meaningful rejection/partial-failure assertions, not only happy-path mocks; not executed in this review.                           |

The new Auth result-retention test covers OpenAI Codex/OpenCode account
connections. It is not itself evidence of all xAI subscription targets. The
native subscription test families provide the additional target/rollback
coverage; the missing Provider exit UI still needs its own renderer tests.

Readback boundary worth retaining in acceptance: subscription `confirmBind`
currently checks Provider saved row/current ID and a fresh managed-auth overview;
it does not consume the newly added `summary.live` observation. The native
service tests inspect actual files, while the renderer assertion alone proves
DB source identity rather than a second actual-file observation. Do not claim
the mock's current ID independently proves live file state.

## #43 condition-by-condition evidence

| Completion condition                                      | Actual implementation and assertions                                                                                                                                                                                                                                                                                                                                  | Assessment                                                                                              |
| --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Real current login, expiry, exit and recovery operations  | `AccountView` gates reauthentication/removal by native actions; `presentation.ts` separately maps `requires_reauth`. Shared `useAgentAuthSession` recovers the existing session rather than starting a duplicate. Auth Page tests inspect resumed sessions and reason-specific recovery copy.                                                                         | Source contract is coherent; real-provider lifecycle evidence is outside this review.                   |
| Accurate Grok official/xAI device/API Key names and state | `GrokOfficialLogin.tsx` calls `ports.agentAuth` for `grokbuild` login/logout, explicitly says the outcome is unverified, and says xAI accounts/API Key are separate. `LoginDialog.tsx` names an xAI device-code account; model subscription account labels also use xAI device-code wording. Grok native summary clears unsupported request/preserved-session claims. | No conflation found in the changed entry points.                                                        |
| Clear official terminal/app handoff                       | `GrokOfficialLogin` exposes login, separately confirmed logout, stop-waiting and refresh; its copy gives `grok login` / `grok logout`. `src-tauri/src/agent_install/auth_actions.rs` uses the closed Grok tooling handoff. Auth Page's handoff test asserts login/logout target intent and zero `managedAuth.startLogin` calls.                                       | Meaningful separation of CLI handoff from OAuth; does not claim a terminal opening is successful login. |
| New vendors are independent integrations                  | The diff does not add a new vendor or broaden the Grok file-projection gate. xAI start validation still enforces provider/method/purpose.                                                                                                                                                                                                                             | No scope expansion observed.                                                                            |

Useful negative coverage includes the gated Grok account action test, xAI
purpose rejection, and `consumers/grok.rs::helper_and_native_projection_stay_disabled`,
which asserts the disabled projection creates no auth file. This review did not
run those tests or verify vendor availability.

## #64 condition-by-condition evidence

| Completion condition                                         | Actual implementation and assertions                                                                                                                                                                                                                                                                                                                                                                                                   | Assessment                                                                                                                                                         |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Disclose target changes and necessary restart                | Subscription bind dialogs show account/model and native write targets; Codex uses its source Change Plan. OpenCode exit shows target paths and restart/new-session guidance.                                                                                                                                                                                                                                                           | Provider-target exit UI and accurate restore target disclosure are missing as detailed above.                                                                      |
| Backup before takeover and authoritative reread              | Proxy enable path takes its app lock, captures strict live backup, writes takeover, keeps backup if compensation fails, and only then updates enabled state. Native subscription roundtrip/port-conflict tests compare actual temporary files, rows and flags.                                                                                                                                                                         | Native assertions are substantive; new final build/runtime evidence still required.                                                                                |
| Restore on failure and preserve partial results              | `managed_recovery.rs::verify_managed_restore` accepts original bytes only for exit retry, while new takeover/rebind remains strict. `restore_verified_managed_config` uses `restore_file_preimage_if_owned` per file. `recovery_tests.rs::managed_exit_retries_partial_files_but_rebinding_stays_strict` injects failure after the first file, proves catalog still projected, rejects rebind, retries and proves both original bytes. | Good native partial-failure coverage. Aggregate stop still returns one joined error string, not a per-target UI result; target-specific UI is the smaller closure. |
| Detect later external edits and let the user handle conflict | `legacy_recovery.rs::restore_legacy_config_if_owned` validates regular-file/size, path-bound receipt, current postimage hash and explained takeover projection, then restores under the expected-write guard. Missing/corrupt receipt or intervening external edit refuses overwrite. OpenCode UI shows current/backup paths and manual conflict guidance.                                                                             | Guard is real. User recovery entry is incomplete for the other exposed Provider targets.                                                                           |

Reviewed rejection/restore tests directly (not just their names):

- `legacy_exit_guards_gemini_grok_and_codex_files_and_keeps_refreshed_native_login`:
  edits the actual file after takeover, expects rejection and byte preservation,
  explicitly resolves the fixture back to owned bytes, then verifies restoration
  and preserved refreshed Codex auth.
- `legacy_exit_preserves_external_file_and_original_backup_in_both_branches`:
  changes Claude permissions and asserts both strict/fallback restores reject
  without changing the file or original DB backup.
- `legacy_exit_restores_exact_bytes_and_retry_is_a_noop`: original bytes and
  backup remain unchanged on repeat restore.
- `legacy_exit_rechecks_external_edit_after_admission` and missing/corrupt
  receipt tests exercise refusal around the admission/write seam.
- `subscription_port_conflict_restores_current_files_flags_and_provider_rows`:
  binds a real occupied local port, expects rollback, and compares file bytes,
  DB/current Provider, flags and listener state.
- `subscription_runtime_rollback_tampering_reports_unknown`: DB triggers
  interfere with compensation; result must be `RollbackPartialStateUnknown`.

These are local native test designs using temporary files and synthetic
credentials. They are not real-account, Windows-host, release or UAT evidence.

## Follow-up handoff

Root received the #64 blocker immediately and authorized a narrow follow-up
implementation after this report. Assigned surface: Provider subscription exit
UI inside the existing section, closed typed ports and focused tests. Root owns
the small native restore-preview dependency and all combined native runs. This
review report records the pre-follow-up state; it must be superseded by the
follow-up's final checks before closing #42/#64.

Renderer follow-up (2026-09-21): the missing Provider-target exit surface now
has an implementation and 137 passing focused renderer/port assertions. See
`.trellis/tasks/09-20-proxy-recovery-64/renderer-exit-followup.md`
for the exact boundary and checks. This supersedes the missing-renderer
finding only; native command integration, full checks, runtime and final issue
acceptance remain parent-owned and are not established by this review.
