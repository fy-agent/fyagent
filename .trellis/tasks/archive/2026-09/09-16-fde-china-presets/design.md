# Design

## Boundary and reuse

Renderer-only. Keep PromptsPort, McpPort, native assignment and all persisted IDs unchanged. No dependency added. Use FeatureTabs/FeatureTabPanel, FeatureSearch, FeatureList, SplitPanes, Button and the existing ConfirmDialog owner.

## Prompt catalogue

Route-local typed original data in prompts/presets.ts; six categories and thirty scenario records. Compose a compact common delivery/safety contract with meaningful per-scenario inputs, approach, deliverables, evaluation and example. The catalogue is not native user data and can be browsed without a native connection.

PresetBrowser owns only local browsing/search/category/selection. PromptsPage remains the sole owner of application, editable draft, dirty blocker and writes. Two tabs distinguish the user's library from static presets. Switching to browse does not destroy a draft; applying a preset while dirty must explicitly discard through the existing confirmation. A selected preset creates a new disabled draft; Save allocates a fresh 128-bit opaque ID with crypto.getRandomValues and uses the unchanged upsert/readback path. Existing persisted IDs are preserved. Enable remains separate. Busy/native-unavailable gates apply to use, not reading static data. New/import header actions belong to the library tab.

## MCP catalogue

Extend the current catalogue factory, launch helper and filter model; no parallel installer or target table. Add the FDE category/filter and annotate relevant existing domestic entries. New entries remain marked as awaiting environment verification. Each recipe links its upstream documentation. Reuse env/password fields; never synthesize endpoints, claim public connectivity, or enable blanket write permissions.

CloudOps only exposes documented ECS_DescribeInstances; ACK omits --allow-write; DataWorks starts with ListProjects. DMS scopes CONNECTION_STRING; DBHub and StarRocks explicitly require a restricted database account because catalogue prose is not a SQL firewall. RDS is conservatively cloud-privileged even with ENABLE_WRITE_TOOLS=false. CloudBase requires its own login/environment; merely adding its configuration is not authorization.

## Rejected alternatives

No automatic seeding/enabling, no prompt substitution engine, no new global store, no bespoke modal or splitter. Do not use old DBHub --readonly flags absent from current docs. Do not silently substitute a similarly named third-party Milvus PyPI release for the official repository. Binary/source-only integrations are deferred rather than inventing npx packages.

## Failure and verification

Native reads/writes retain existing error and refresh-warning semantics. Empty searches show an actionable empty state. Keyboard names, local overflow, compact layouts and dirty transitions are tested. Static catalogue checks prove payload construction, not remote MCP authentication or LLM output quality. Research and review evidence stays with this task; durable contracts go into frontend SPEC before archive.
