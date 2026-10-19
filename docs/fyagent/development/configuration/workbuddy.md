# WorkBuddy configuration flow

Current WorkBuddy services, typed renderer flows, and tests define URL
normalization, remote model discovery, document parsing, overwrite/revision
behavior, backup and atomic replacement, errors, and IPC. Retained WorkBuddy
notes under `.trellis/spec/` are optional AI-assistance review material.
WorkBuddy remains independent of Codex provider configuration even when the
renderer presents both as configuration experiences.

## Data flow

```text
WorkBuddy page input
  -> typed renderer API/query layer
  -> bounded URL and credential validation
  -> remote model projection
  -> current ~/.workbuddy/models.json snapshot
  -> revision/overwrite decision
  -> backup and atomic replacement
  -> refreshed typed status
```

The third-party API's `/v1` path and the WorkBuddy document shapes are real
compatibility contracts. They remain in current code, spec, and tests; they
are not product-stage documentation labels.

Backend tests under `src-tauri/src/services/workbuddy/` own parsing,
normalization, write, concurrency, and failure fixtures. Renderer tests under
`tests/renderer/pages/models/` own form, query, conflict, and accessible-state
behavior.

## Experimental account subscriptions

API Key configuration remains the default. Account subscription mode explicitly
selects one existing OpenAI or xAI account and one model. OpenAI models are
entered manually; xAI suggestions do not guarantee account availability.

The renderer reads `get_workbuddy_subscription_source`, then creates a zero-write
`create_workbuddy_subscription_plan` with the exact account/model and current
revision/binding version. Preview shows the frozen local service address and
same-model replacement. Closing before confirmation does not apply the plan.
Confirmation sends only `{planId, planDigest}` to `apply_change_plan`.

Only a successful binding job plus a fresh matching source readback permits
saved copy. Conflicts, failed reads and mismatched account/model/route/version
remain unconfirmed. WorkBuddy uses a dedicated managed-proxy route and local
credential; upstream secrets remain in managed auth. FyAgent must stay running.
This flow does not switch Codex, Claude Code, Grok Build or OpenCode sources.

Renderer/unit and Playwright fixtures verify UI and command contracts only.
Native rollback/listener/file isolation requires native tests. Real OpenAI and
xAI acceptance requires WorkBuddy itself to make requests on the tested device;
a saved message, model discovery or fake upstream cannot substitute for it.
