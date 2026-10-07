# CC Switch v3.20.4 Upstream Provenance

This ledger records the source identity and ancestry of the CC Switch v3.20.4
integration. It is not a FyAgent Release Note and does not change FyAgent's
product version from `0.4.10` to the upstream version.

**标签祖先关系已集成；行为移植按组列状态；Rust 验证待完成。**
2026-10-07 fix1 仅解除数据库/备份的已知源码阻断并明确数据库增量，
不是 S02/S03/S04/S23 的功能验收，也不表示整个 Rust 工程已可编译。

## Verified source and graph

| Field                       | Verified value                                         |
| --------------------------- | ------------------------------------------------------ |
| Authorized FyAgent baseline | `5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798`             |
| Merge first parent          | `5b1a334bbf6a8e3df59d2d5b8b3dd03eb11bd798`             |
| Upstream repository         | `https://github.com/farion1231/cc-switch.git`          |
| Annotated tag               | `v3.20.4`                                              |
| Tag object                  | `93994110505d4d4aae3a7a3582797aae48c2dc74`             |
| Peeled commit               | `43e1d99084ed9b2f5dc252fd35c5adaf29d6876e`             |
| Merge base                  | `43eaf07355af145aebfee301801779e824d4c221` (`v3.19.2`) |
| FyAgent two-parent merge    | `3d1ee4fe5f7d6b6da437cb89315393c510dcf0b7`             |
| Merge second parent         | `43e1d99084ed9b2f5dc252fd35c5adaf29d6876e`             |
| Integration date            | 2026-10-07 (Asia/Shanghai)                             |
| Database schema             | 27 after fix1; FyAgent migrations through 26 preserved   |

The annotated tag object and peeled commit match the approved v3.20.4 identity.
The merge commit has exactly those two parents, with the peeled upstream commit
as the second parent, and that commit is an ancestor of the merge. The merge
was created with explicit `--no-ff --no-commit` semantics. Conflict
resolution remains incomplete; ancestry does not prove behavioral integration. It was not squashed, rebased, or resolved with a repository-wide
ours/theirs strategy.

The shared repository remotes were not rewritten. 因账号原因，origin 暂为
`https://github.com/junshi-fy/fyagent.git`（fetch/push）；upstream fetch 实际为
`https://github.com/fy-agent/fyagent.git`，push 为 `DISABLED_no_push_to_main_repo`。
这与 upstream-sync.md 的 canonical remote 角色不一致；按本轮授权仅记录现状，
不修改配置、不推送、不创建 PR。 The local clone at
`/workspace/fy-maint-1007/migration/cc-switch` is a partial clone and could not
supply the tag's blobs, so `refs/tags/v3.20.4` was fetched from the upstream
URL above after `ls-remote` confirmed the tag object and peeled commit.

## Provenance and license boundary

CC Switch-derived code, history, notices, and attribution retain their MIT
ancestry. The upstream v3.20.0–v3.20.4 release-note bodies were not kept in the
FyAgent tree: they advertise unsupported desktop packages and distribution
install commands, which the supported-platform scanner rejects. They remain
reachable from the second parent. This ledger and the existing FyAgent
CHANGELOG preserve provenance without presenting upstream release marketing as
a FyAgent release.

FyAgent-owned components and modifications remain under the repository's
published PolyForm Noncommercial terms. See [LICENSE](../../LICENSE),
[LICENSING.md](../../LICENSING.md), and
[THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). No upstream partner,
sponsorship, affiliate, or tracking metadata became a FyAgent product claim.

## FyAgent contracts preserved through the merge

- product/runtime identity: `FyAgent`, `fyagent`, `fyagent_lib`,
  `com.fyagent.desktop`, and `fyagent://`;
- version only from `[workspace.package].version = "0.4.10"`;
- persistence correction: the merge kept `SCHEMA_VERSION = 26` but already
  added `skills.enabled_mcode`, `session_log_sync.last_byte_offset` /
  `last_tail_fingerprint`, and `session_usage_dedup`. It also mistakenly
  duplicated FyAgent v16→17/v17→18 method names with upstream migrations.
  Fix1 keeps those storage additions in one explicit FyAgent **v26→v27**
  migration; all historical FyAgent migration bodies through v26 stay intact.
  Keeping the fields supports session readers already present in the merge;
  removing them would require reverting their consumers too. No new Mcode
  skill/MCP product integration is enabled;
- the deleted production renderer stays deleted. New files that import the old
  module graph were not restored;
- capabilities stay on the FyAgent list. `process:allow-exit`,
  `process:allow-restart`, and `dialog:default` were not added;
- Pi command handlers are present as source but are not registered on the
  invoke surface. Pi/Mcode skill and MCP integration is incomplete; the retained Mcode
  storage column alone does not provide a usable target;
- native Fetch stays. `cross-fetch` was not reintroduced;
- new implicit broad-family cfg sites were rewritten to explicit macOS or
  Windows. The supported-platform structure manifest was refreshed after that
  rewrite.

## Conflict resolution

The merge started with 274 unmerged paths: 82 content conflicts, 143 paths
deleted by FyAgent and modified upstream, and 49 upstream additions inside
directories FyAgent had renamed. The original report counted 417 hunks in 82 files. These are historical
resolution counts, not behavioral acceptance evidence:

| Resolution | Hunks |
| --- | ---: |
| Overlapping region kept FyAgent data, identity, or behavior | 262 |
| Whole-file rule kept FyAgent (docs, CI, changelog, identity) | 58 |
| Upstream-only addition taken | 27 |
| FyAgent-only addition kept | 17 |
| Partner `isPartner` additions dropped | 14 |
| `utm_` tracking rejected | 10 |
| Unsupported-platform additions rejected | 10 |
| Partner `isPartner` edits rejected | 7 |
| Unsupported-platform additions dropped | 3 |
| `partnerPromotion` rejected | 1 |
| Unions (store, MCP module declaration, command and service modules) | 8 |

Deleted-by-us paths stayed deleted, including the old renderer, old manuals,
and root `vitest.config.ts`. File-location moves into the old UI or partner
assets were not accepted. Clean auto-merges were still scanned: partner banner
assets were removed, and `utm_`, `aff`, `affiliate`, `ic`, `ref`, and
`referral` query keys were stripped from presets. Campaign parameters `ac` and
`rc` on the Volcengine coding-plan URL were stripped because they were not on
the FyAgent baseline.

After the mechanical pass, these contract repairs were applied:

- OpenCode Go declares `apiKeyField: "ANTHROPIC_API_KEY"`, matching the env key
  the kept preset already writes.
- The duplicate AtlasCloud card (`zai-org/glm-5.2`) was removed. The kept card
  remains `zai-org/glm-5.1` with context window 200000. The Codex chat test
  expectation follows that kept card, and the Volcengine preset name stays
  `火山Agentplan`.
- Codex model-catalog helpers that had been copied back into
  `codex_config.rs` were removed. `codex_config/catalog.rs` remains the owner,
  including `resolve_fyagent_catalog_path`.
- The facade copy of `ToolInstallationReport` and `probe_tool_installations`
  was removed. `services/tooling/discovery.rs` remains the owner. The copied
  planner called a subsystem shell helper that does not exist here.
- Diagnostic strings no longer interpolate `{account_id}`.
- `session_usage_mcode` is `pub(crate)`.
- The newly added nightly workflow under `.github/workflows/` was not kept.
  The change classifier has no class for that path, and CI modernization is
  outside this merge.
- Helpers whose names or tests contain the forbidden subsystem token were not
  kept. The platform scanner rejects that token, and the version probe called
  a distro-name checker that is not defined. The Windows `ReplaceFileW`
  fallback for `ERROR_NOT_SUPPORTED` was kept without naming a subsystem.
- Qianwen API-key URLs that contained the retired home-directory path token
  were pointed at `https://platform.qianwenai.com/`.

## Fix1 behavior status and remaining work

Evidence level: `code_audit` plus Python SQLite execution of source DDL;
no Rust build, test execution, native runtime, or installer acceptance.

| Group | Current code / fix1 | Remaining work (not implemented in fix1) |
| --- | --- | --- |
| S02 | REAL/NUL/non-UTF8 SQL formatting, sequence dump, staging auto-vacuum and incomplete-transaction rejection are present. Restored the missing FyAgent `validate_basic_state` definition. | Wire core-table validation **before** create/migrate; finish import protection and validate header-only/truncated/missing-core-table cases, including valid empty backups. |
| S03 | Restored `complete_backup`, locked backup wrappers, connection/protected-path/publish-hook parameters, test imports and import/restore hooks. Restore reuses its held lock. Existing temporary publish code now has its required inputs. | Validate restore candidate integrity/core tables; protect the selected source together with the safety snapshot; complete atomic backup/restore and validation-before-mutation behavior. Exercise corrupt/future DBs, publish failures/collisions, concurrency and retention=1 in Rust. |
| S04 | Shared sync mutex, Skills write locks and post-restore live/config/cache sync remain present. | Add `session_log_sync` and `session_usage_dedup` to both skip/preserve sets; recapture all local state under the final connection lock. Existing late-write tests remain pending and can expose these gaps. |
| S23 (with S22 / #210) | Byte observation is wired; fix1 supplies the missing schema-26 upgrade path. | Wire leading/trailing UUID acceptance into real validation; verify Windows same-mtime growth, unchanged partial-line skip and append retry. SQL migration evidence is not Rust/session behavior evidence. |

S20's four reviewed proxy fixes and S01's error-50 fallback are present in
reachable source; Rust/native validation remains pending. Other groups are
unverified, not implicitly completed by tag ancestry. Re-merging this same
tag cannot restore the discarded behavior.

Fix1 also found source blockers outside the database boundary: undeclared
Pi/Mcode config modules referenced by active code, missing Skills helpers/fields,
and missing Provider OAuth/preflight helpers. These must not be hidden by the
phrase “Rust verification pending”; a whole-project compilation pass is not
claimed.

## Fix1 validation (2026-10-07)

- Node 24.19.0: `pnpm typecheck` and `pnpm lint` passed.
- Focused session-migration, Rust-module-boundary, native-security-ordering,
  DEP0040, native-Fetch/MSW/Tauri and provider-promotion tests: **12 files,
  67 tests passed**. The separate version-consistency suite failed 3 tests
  with child-process `spawnSync ... node EPERM`; direct
  `node scripts/version.mjs check` passed (`0.4.10`).
- Python SQLite reproduced the old v26 missing-byte-column failure, then
  verified the source v27 DDL: old rows/flags preserved, NULL cursor defaults,
  dedup PK/index, idempotence, fresh/upgraded table shape parity and late-failure
  rollback. Five v27 Rust regression tests and the original v26 DDL fixture
  have been added but have **not** been executed by Rust.
- Static scan: no duplicate Database methods; no missing Database associated
  method definitions referenced by `backup.rs` / `schema.rs`. All 25 historical
  production migration/helper bodies and dispatch arms 0–25 match the pre-merge
  baseline. This is a bounded name scan, not type checking.
- Local `rustc` is 1.85.1; repository toolchain is 1.97.1; `rustfmt` is absent.
  No cargo build/check/test was attempted. Native validation and the additional
  source blockers listed above remain open. No stable-baseline acceptance.

## Original merge validation (historical, not a fresh fix1 run)

`pnpm typecheck` and `pnpm lint` passed. `pnpm test:unit` reported 207 files
passed and 6 failed (213 total); 2066 tests passed, 27 failed, and 3 skipped
(2096 total). The failing files are the known host-integration and governance
set: `miseTaskContract`, `windowsMsvcCross`, `developmentEnvironment`,
`systemCheck`, `taskDocs`, and `repositoryGovernanceScan`. The router-shell
load timeout did not fail on this run. Rust was not compiled on this machine.
`Cargo.lock` was not regenerated, so new upstream crates are not locked.

The long-term engineering contract is
[CC Switch Upstream Synchronization](../../.trellis/spec/backend/upstream-sync.md).
