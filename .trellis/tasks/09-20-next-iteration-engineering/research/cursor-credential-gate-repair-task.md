# Credential and ChangePlan failures from full native gate

Continue in the same integration tree/branch. Your native gate writer stopped.
Root accepted the typed UsageTestInput direction and now explicitly authorizes
repair of the five remaining credential/ChangePlan failures. Root owns the
catalog ACL test (already added the existing config-pack manifest to the test
inventory; no permission expansion). Separate Cursor recovery conversation owns
the two proxy/provider_service failures. Do not edit its files.

Own `services/provider/credentials.rs`, its submodules/tests, `usage.rs`, and
`services/change_plan/service.rs` including narrow production fixes if a real
behavior regression is demonstrated. Read their contracts first, preserve all
existing task edits. No installer/helper/frontend/other proxy changes.

The narrow `provider/live.rs` legal-common-settings preservation path is also
in this package, as it is a direct cause of failure 3. If failure 5 proves an
existing ChangePlan adapter readback bug, its exact verification path and focused
regression are in scope; preserve strict target/digest/credential binding and
describe the evidence. There is no other writer on those paths. Remove all
temporary Debug instrumentation before final tests and handoff.

Failures in `artifacts/cursor-native-gate.log`:

1. `blank_legacy_edit_does_not_erase_the_only_copy_when_migration_is_locked`:
   unlocked save returns provider_secret_invalid.
2. `actual_usage_query_materializes_auxiliary_fields_before_script_validation`:
   test invokes Tokio-dependent async operation without reactor.
3. `common_config_keeps_legal_settings_but_redirect_requires_fresh_authorization`:
   live text lacks expected legal notifications=false setting.
4. `credential_capability_accepts_only_extractable_unmanaged_target_material`:
   inactive bearer fixture is rejected at the new save facade before plan.
5. `upsert_plan_is_side_effect_free_and_apply_writes_once`: admitted job Failed.

Reproduce and distinguish stale/invalid fixtures from real bugs. A negative
credential fixture must preserve its fail-closed assertion at the correct layer;
use an existing raw legacy insertion helper only if testing actual legacy data,
not to bypass production validation. Retain active-target-only extraction,
secret-free public DTOs, native resolver, target-bound reauthorization and no
plaintext fallback. Keep legal user config preservation. Upsert must actually
save/switch once after preview and stay idempotent; do not change expected success
to failure just to silence the test. Use a real Tokio test runtime where required.

Run only the five focused filters and relevant credentials/change-plan suites
after removing Debug instrumentation. Record actual failure causes and fixes in
`research/cursor-credential-gate-repair-result.md`. No full backend run while the
other recovery package writes; root will run the final gate after both stop.
Use rtk, canonical mise, and the existing root CARGO_TARGET_DIR. No commit/push/
Issue closure or real credential/network changes. Stop writer after delivery.
