# Ownership and save-path audit

Worktree: /Users/serendipity/.codex/worktrees/fyagent-next-secretref/fyagent
Branch: codex/next-secretref-35; base: 2c09c4be2c5f50b4060fd6f7a13a7be31c364c54.

Database::save_provider and update_provider_settings_config keep their public
signatures, now implemented by the Provider credential owner. SQL primitive
save_provider_record is crate-private. Its non-fixture consumers are the owner,
the exact outer activation rollback, and the explicit non-Codex/non-API branch.
The backup importer restores exact locally owned rows in SQL; legacy DB import
and historical schema migration remain import sources and are covered by later
startup/save migration. No Provider API writer bypass was introduced.

The separate provider_secret_guard covers native mutation/import serialization;
no SQL connection guard is held while invoking NativeSecretBackend. Database
composition depends only on the pre-existing secret leaf. Export helpers live in
provider/redaction.rs, not in ProviderService, avoiding a reverse orchestration
dependency. Hardware trait and native backend implementations are unchanged.

Parallel ownership respected: no navigation, version, installer, vendor presets,
Grok forwarder/handler/Codex adapter, WorkBuddy recovery, proxy stop/dynamic-port,
universal subcard metadata or proxy recovery state-machine edits.

Actual executor: native trellis-implement sub-agent in the specified worktree.
Exact runtime model identifier and reasoning effort are not observable in this
subagent tool context; do not infer them from global defaults or labels.
