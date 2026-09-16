# Research and selection review

Reviewed 2026-09-16. Sources are public official documentation or upstream project repositories. Original prompt text is authored for FyAgent; OpenAI has not endorsed these templates. This is source/configuration review, not authenticated MCP smoke testing or a model benchmark.

## Prompt design evidence

- https://developers.openai.com/api/docs/guides/prompt-engineering — distinguish role, instructions, examples and task context; use clear structure rather than a rigid oversized role-play script.
- https://developers.openai.com/api/docs/guides/evaluation-best-practices — task-specific datasets, metrics, edge cases, human calibration and regression evaluation; never substitute “looks good” for evidence.
- https://developers.openai.com/api/docs/guides/agent-builder-safety — treat external text as untrusted, limit data flow, retain tool approval and defense in depth. Only general safety principles are used, not a dependency on Agent Builder.
- https://openai.com/careers/forward-deployed-engineer-%28fde%29-sf-san-francisco/ — FDE scope runs from discovery/scoping to implementation and production rollout. Chosen scenarios focus on this delivery lifecycle, not generic expert personas.

## Accepted MCP additions

| Entry     | Primary source                                                                        | Recipe / review decision                                                                                                                                                                                               |
| --------- | ------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CloudBase | https://github.com/TencentCloudBase/CloudBase-AI-Toolkit                              | Official @cloudbase/cloudbase-mcp; separate vendor login; cloud changes/local upload risk, not “no authentication”.                                                                                                    |
| DMS       | https://github.com/aliyun/alibabacloud-dms-mcp-server                                 | uvx alibabacloud-dms-mcp-server; AK/SK/optional STS in env; required single CONNECTION_STRING; upstream explicitly supports DML/DDL.                                                                                   |
| DataWorks | https://github.com/aliyun/alibabacloud-dataworks-mcp-server                           | Published npm package; REGION and AK/SK env; documented TOOL_NAMES starts with ListProjects, not every API.                                                                                                            |
| ACK       | https://github.com/aliyun/alibabacloud-ack-mcp-server                                 | Published uvx entry; ACCESS_KEY_ID/ACCESS_KEY_SECRET (not Alibaba-prefixed keys); private cluster access; omit --allow-write; RAM/RBAC still required.                                                                 |
| RDS       | https://github.com/aliyun/alibabacloud-rds-openapi-mcp-server                         | Published uvx entry; AK/SK/STS env, explicit stdio; conservative cloud privilege because instance/account operations exist.                                                                                            |
| CloudOps  | https://github.com/aliyun/alibaba-cloud-ops-mcp-server/blob/master/README_mcp_args.md | Published uvx entry; --env domestic and --visible-tools ECS_DescribeInstances; exclude local shell/deploy/default broad toolset.                                                                                       |
| DBHub     | https://github.com/bytebase/dbhub ; https://dbhub.ai/config/command-line              | Official npm package; Node >=22.5; stdio, DSN in secret env; current docs do not list legacy --readonly. Require DB-enforced limited account; advanced readonly/max_rows belongs in reviewed TOML, not invented flags. |
| StarRocks | https://github.com/StarRocks/mcp-server-starrocks                                     | Official published package via uv run --with; database credentials in env; DDL/DML and local export tools mean write privilege; optional CA verification must not be mislabeled as always secure.                      |

Product review: these add data integration/governance, warehouse analytics, cloud inventory, database/container operations and WeChat/web delivery. Existing Feishu, DingTalk, Yunxiao, Gitee, Tencent Docs, TAPD, Yuque, Apifox and Amap cover collaboration/location without duplicated recipes. This is China-oriented integration selection, not a quantified market-share ranking or a guarantee of mainland network availability.

Security review: official ownership does not imply read-only, safe-to-run or successfully connected. Each newly reviewed recipe has explicit runtime/auth/risk and awaits customer environment verification. Do not execute third-party MCP tools or request actual customer secrets during this repository change.

Maintenance review: upstream setup must describe an executable/package and env keys; no additional FyAgent runtime dependency is vendored. Existing launch/secret/assignment owners remain authoritative. Package availability and supported Python/Node environments still need customer-side validation; @latest means upstream can change after this review.

## Deferred / rejected

- Tencent COS: official README places secrets in argv; do not copy without proving a safer supported credential path. https://github.com/Tencent/cos-mcp
- Milvus: official source and an independently maintained/pre-release PyPI project must not be conflated. Official repo https://github.com/zilliztech/mcp-server-milvus ; package https://pypi.org/project/mcp-server-milvus/ . Defer until publisher/release provenance is reconciled or add a separately designed self-hosted flow.
- Alibaba observability: current official repo uses a Go binary/config.yaml, not an invented uvx command. https://github.com/aliyun/alibabacloud-observability-mcp-server . Binary provisioning is outside this task.
- Volcengine monorepo, OceanBase, payment/ERP integrations: considered for coverage but not admitted without a fully verified, bounded recipe. No fabricated enterprise endpoint or unofficial package relabeled as official.

## Verification limits

No customer bank/health/production credentials are involved. Prompt structure and deterministic regression tests are not evidence of a measured model success rate. Sector legal/medical/financial decisions remain with authorized professionals; templates require current sources and local policy rather than asserting compliance.
