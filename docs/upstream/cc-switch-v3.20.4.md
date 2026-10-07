# CC Switch v3.20.4 Upstream Provenance

This ledger records the source identity and ancestry of the CC Switch v3.20.4
integration. It is not a FyAgent Release Note and does not change FyAgent's
product version from `0.4.10` to the upstream version.

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
| Database schema             | 26 (unchanged; upstream schema 19 was not imported)   |

The annotated tag object and peeled commit match the approved v3.20.4 identity.
The merge commit has exactly those two parents, with the peeled upstream commit
as the second parent, and that commit is an ancestor of the merge. The merge
was created with explicit `--no-ff --no-commit` semantics and semantic conflict
resolution. It was not squashed, rebased, or resolved with a repository-wide
ours/theirs strategy.

The shared repository remotes were not rewritten. The local clone at
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
- persistence: schema **26**. Upstream schema-19 deltas (`enabled_mcode`,
  session byte cursor, migration rewrites v16–v19) were not applied and
  `SCHEMA_VERSION` was not bumped;
- the deleted production renderer stays deleted. New files that import the old
  module graph were not restored;
- capabilities stay on the FyAgent list. `process:allow-exit`,
  `process:allow-restart`, and `dialog:default` were not added;
- Pi command handlers are present as source but are not registered on the
  invoke surface. Pi and Mcode are not persisted skill or MCP targets;
- native Fetch stays. `cross-fetch` was not reintroduced;
- new implicit broad-family cfg sites were rewritten to explicit macOS or
  Windows. The supported-platform structure manifest was refreshed after that
  rewrite.

## Conflict resolution

The merge started with 274 unmerged paths: 82 content conflicts, 143 paths
deleted by FyAgent and modified upstream, and 49 upstream additions inside
directories FyAgent had renamed. Semantic hunk review covered 417 hunks in 82
files:

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

## Validation at this ledger

`pnpm typecheck` and `pnpm lint` passed. `pnpm test:unit` reported 207 files
passed and 6 failed (213 total); 2066 tests passed, 27 failed, and 3 skipped
(2096 total). The failing files are the known host-integration and governance
set: `miseTaskContract`, `windowsMsvcCross`, `developmentEnvironment`,
`systemCheck`, `taskDocs`, and `repositoryGovernanceScan`. The router-shell
load timeout did not fail on this run. Rust was not compiled on this machine.
`Cargo.lock` was not regenerated, so new upstream crates are not locked.

The long-term engineering contract is
[CC Switch Upstream Synchronization](../../.trellis/spec/backend/upstream-sync.md).
