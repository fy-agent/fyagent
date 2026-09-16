# FDE MCP Catalogue Recipes

## 1. Scope / Trigger

Read before changing FDE discovery membership or China-oriented cloud/data
recipes. [MCP](./mcp.md) owns installation, targets, secret display and native
mutation behavior; this document owns the catalogue-only additions.

## 2. Signatures and owners

`src/pages/mcp/catalog.ts` remains the single recipe owner. Its existing
`McpCatalogItem.build(values, apps, platform): McpServer` validates required
fields/targets and uses `npxSpec` for Windows/macOS launch differences.
`McpCatalogCategory` and `McpCatalogFilterId` admit `fde`; Discovery intersects
this category with ordinary search. No new installer, Port or endpoint exists.

## 3. Contracts

- The FDE filter includes relevant existing domestic collaboration, mapping,
  API and deployment entries plus the new cloud/data entries; do not clone
  existing IDs to create a second catalogue. Other install-mode filters retain
  their existing meaning. No-field configuration does not imply no login.
- New FDE recipes are `provenance: official`, `maturity: verify`, linked to
  upstream setup documentation and carry actual runtime/auth/risk notes.
  Configuration review is not authentication, availability, audit certification
  or a guarantee of mainland connectivity. Revalidate upstream when editing.
- Credentials/DSNs use password fields and `env`, never argv, search or card
  text. `CONNECTION_STRING` for DMS identifies an authorized database rather
  than carrying its password. Optional STS values are omitted when blank.

| ID / launcher                                                                        | Required configuration and conservative defaults                                                                                                                                                  |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cloudbase` / npx `@cloudbase/cloudbase-mcp@latest`                                  | Vendor login and environment selection occur later; cloud privilege, no automatic login/deploy.                                                                                                   |
| `aliyun-dms` / uvx `alibabacloud-dms-mcp-server@latest`                              | `ALIBABA_CLOUD_ACCESS_KEY_ID`, `ALIBABA_CLOUD_ACCESS_KEY_SECRET`, required `CONNECTION_STRING`, optional `ALIBABA_CLOUD_SECURITY_TOKEN`; write privilege, not a SQL firewall.                     |
| `aliyun-dataworks` / npx `alibabacloud-dataworks-mcp-server@latest`                  | Alibaba AK pair, `REGION`, fixed `TOOL_NAMES=ListProjects`; read-only tool selection, not general DataWorks write access.                                                                         |
| `aliyun-ack` / uvx `alibabacloud-ack-mcp-server@latest`                              | `ACCESS_KEY_ID`, `ACCESS_KEY_SECRET`, `KUBECONFIG_MODE=ACK_PRIVATE`; no `--allow-write`, requires cluster network/RAM/RBAC.                                                                       |
| `aliyun-rds` / uvx `alibabacloud-rds-openapi-mcp-server@latest`                      | Alibaba AK pair, optional STS, `SERVER_TRANSPORT=stdio`, `ENABLE_WRITE_TOOLS=false`; still cloud privilege, not guaranteed read-only.                                                             |
| `aliyun-cloudops` / uvx `alibaba-cloud-ops-mcp-server@latest`                        | Alibaba AK pair; fixed `--transport stdio --env domestic --services ecs --visible-tools ECS_DescribeInstances`; do not add broad/local execution tools.                                           |
| `dbhub` / npx `@bytebase/dbhub@latest --transport stdio`                             | Required secret `DSN`; Node >=22.5. Do not invent legacy `--readonly`; database account permissions must enforce read-only.                                                                       |
| `starrocks` / `uv run --with mcp-server-starrocks mcp-server-starrocks --mode stdio` | Required secret `STARROCKS_URL`; optional `STARROCKS_SSL_CA` sets both `STARROCKS_SSL_VERIFY_CERT=true` and `STARROCKS_SSL_VERIFY_IDENTITY=true`. Missing CA does not guarantee secure transport. |

Tool-selection flags complement server-side permissions; instructions and
privilege labels do not enforce authorization. DMS, DBHub and StarRocks can
write SQL; RDS and CloudBase can change cloud resources. The catalogue does not
execute these packages to validate connectivity or request customer secrets.
Package resolution remains upstream-owned; no SDK/runtime dependency is added
to FyAgent's own lockfiles by adding a recipe.

## 4. Validation & Error Matrix

| Condition                                                    | Required behavior                                                             |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| Required field missing / no target                           | Existing builder throws `UserFacingError` before native installation.         |
| Optional STS or CA blank                                     | Omit optional env keys, do not create fictional credentials/TLS state.        |
| Platform is Windows                                          | npx uses existing `cmd /c` adapter; uv/uvx retain their documented argv.      |
| Database account can write                                   | Keep write/cloud risk even when the intended workflow only queries.           |
| Upstream only provides source/binary or publisher is unclear | Do not invent an installable npm/PyPI recipe or mark community code official. |
| FDE filter + search                                          | Both apply; unrelated catalogue entries are excluded.                         |

## 5. Good / Base / Bad Cases

Good: a restricted CloudOps inventory configuration is built for one chosen
Agent, then validated by the customer in a test account. Base: a DBHub DSN
stays in env, with database permissions and TLS independently configured.
Bad: claim a prose “read only” instruction prevents SQL writes, or equate a
successfully saved CloudBase config with a successful login/deployment.

## 6. Tests Required

`tests/renderer/features/mcpCatalog.test.ts` covers all new IDs, category,
official/verify metadata, nonempty docs/risk, required fields, zero targets,
macOS/Windows identity generation and no private values in argv/search.
Assert exact env keys, restricted tool selectors, STS omission and conditional
CA verification. `featurePages.test.tsx` checks FDE/search intersection while
retaining install-mode and ordinary install/assignment regressions.
These deterministic tests do not replace authenticated customer-side smoke.

## 7. Wrong vs Correct

Wrong: DSN in `--dsn` argv, an unverified `--readonly` flag, or an unrestricted
CloudOps toolset labeled read-only. Correct: DSN in env, accurately stated
database permissions, and the documented inventory-only CloudOps selector.
