# Local Proxy Pipeline Contract

## 1. Scope / Trigger

Read this contract before changing local proxy routes, the internal listener,
request context construction, Provider selection, circuit-breaker permits,
retry/failover classification, request/response transformation, SSE handling,
or usage attribution. The implementation owner is `src-tauri/src/proxy/`;
configuration persistence remains owned by
[Database Persistence](./database-persistence.md).

Tauri command admission, `ProxyService` start/stop orchestration, takeover,
Provider switching, live-config backup/restore, and crash recovery are owned by
[Local Proxy Service and Takeover](./proxy-runtime.md). This file owns the HTTP
engine after that service has admitted and configured it.

[Managed Account Proxy](./managed-account-proxy.md) owns which explicit
OpenAI/xAI account, model and Agent target may create the managed Provider and
how its local route appears in the account overview. This pipeline consumes the
admitted Provider/account lineage; it does not select an account or publish
connection summaries.

Provider-specific adapters may add narrower authentication or wire-format
rules, but they must enter and leave through this pipeline rather than creating
a parallel HTTP server, retry loop, health ledger, or usage logger.

## 2. Signatures

The local server exposes health/status plus closed protocol route families:

```text
GET  /health
GET  /status
POST /v1/messages and /claude/v1/messages
POST chat-completions route family
GET  /models and /v1/models
POST responses/compact route families, including Codex and Grok Build aliases
POST /opencode/v1/responses (isolated OpenCode application context)
ANY  /v1beta/*path, /gemini/v1beta/*path, /gemini/v1/*path
```

Core Rust boundaries are:

```text
ProxyServer::start() / stop() / status()
RequestContext + UsageParserConfig

ProviderRouter::select_providers(appType) -> Result<Vec<Provider>, AppError>
ProviderRouter::allow_provider_request(providerId, appType) -> AllowResult
ProviderRouter::record_result(providerId, appType, usedHalfOpenPermit,
                              success, errorMessage)
ProviderRouter::release_permit_neutral(providerId, appType,
                                       usedHalfOpenPermit)

ProviderAdapter::extract_base_url(provider)
ProviderAdapter::extract_auth(provider)
ProviderAdapter::build_url(baseUrl, endpoint)
ProviderAdapter::get_auth_headers(...)
ProviderAdapter::transform_request(...) / transform_response(...)
```

Proxy control commands/services remain registered, but the current renderer
has no general Proxy settings Port or panel. Managed Auth's Local Proxy account
connection is a separate feature, not that retired UI. Any command caller must
not supply an arbitrary adapter implementation, retry classifier, upstream
socket, database handle, or raw response-success override.

## 3. Contracts

### Server and request context

- `ProxyServer` owns one listener lifecycle and one shared `ProxyState`. The
  shared `ProviderRouter` retains circuit state across requests; per-request
  construction must not reset health or half-open counters.
- Bind/listen failures are authoritative. Status becomes running only after a
  listener has bound successfully, and stop uses the owned shutdown channel;
  repeated lifecycle calls must not spawn orphan accept loops.
- Route aliases normalize into an explicit application/protocol context. Query
  strings, model resources, streaming intent, session identity, timeout policy,
  and usage parsers are carried deliberately; handlers must not infer an app
  only from an untrusted substring.
- `/opencode/v1/responses` uses OpenCode's effective Provider, configuration,
  breaker and usage scope. It reuses the Responses/Codex protocol adapter and
  never borrows Codex's current Provider. Generic OpenCode API-key takeover
  remains outside this dedicated managed binding surface.
- Request bodies and headers are bounded and normalized before forwarding.
  Protected routing/authentication headers cannot be replaced by generic local
  override configuration.

### Provider selection and permits

- With automatic failover disabled, routing uses only the effective current
  Provider and does not silently append the failover queue.
- With automatic failover enabled, routing uses only the persisted ordered
  queue and skips unavailable circuits. Missing Providers are ignored; an empty
  usable result is distinguished between no configured Provider and every
  queued Provider being circuit-open.
- `select_providers` checks availability but does not consume a half-open
  permit. The forwarder acquires a permit immediately before an attempt and
  must resolve it exactly once through `record_result` or
  `release_permit_neutral`.
- Circuit keys are scoped by `appType:providerId`. Health, thresholds, resets,
  and hot configuration updates must not leak across applications.

### Forwarding, retry, and semantic success

- The adapter owns URL/auth/wire projection for one Provider. The forwarder
  owns attempt order, timeouts, failover, transport, success admission, and
  circuit accounting. Neither layer duplicates the other's responsibility.
- Client-invalid history, local validation failures, and definitive credential
  failures are terminal. Only explicitly classified transport/upstream or
  semantic failures may advance to the next Provider.
- A non-streaming response is not recorded successful until its bounded body
  has been read and provider-specific success semantics pass.
- A streaming response is not recorded successful merely because headers are
  2xx. The first meaningful chunk/event is primed and validated; it is then
  replayed to the client exactly once.
- Provider-specific 2xx error envelopes are failures when their adapter/parser
  can prove failure. Unknown or malformed bodies are not rewritten into a
  fabricated success.

### Managed subscription Responses policy

- OpenAI/xAI managed credentials pin vendor origin and Responses protocol before
  editable metadata. API-key and official native-client auth paths retain their
  own adapters. Resolve the actual bound upstream model before final headers.
- `providers/managed_responses::prepare_request(provider, endpoint, body)` runs
  after editable overrides. OpenAI generation requests share the idempotent
  `prepare_openai_generation` policy with Claude conversion: force
  `store=false` and upstream `stream=true`, remove `max_output_tokens`,
  `temperature` and `top_p`, supply missing instructions/tools/parallel defaults,
  and deduplicate encrypted-reasoning inclusion. Explicit supported tool,
  instruction, parallel, include and service-tier values remain intact.
  Compact requests have a separate schema and must not receive generation-only
  fields. Both Claude conversion and native Responses handlers aggregate the
  forced upstream SSE response when the downstream caller requested JSON;
  stream=true retains native SSE/tool events. Reuse the bounded completed-event
  parser and do not fabricate success from a failed or incomplete stream.
- xAI reuses namespace/sanitize owners, normalizes the route model, hoists
  text-only system/developer content into instructions, removes unsupported
  reasoning/encrypted replay and retention fields, and maps a legacy response
  format only when no explicit text policy exists. Preserve function calls,
  call IDs, arguments and empty function-result outputs; reject unsupported
  instruction content instead of silently deleting it.
- Native xAI compatibility also applies to API-key Responses routes whose
  effective parsed upstream hostname is exactly `api.x.ai`. Codex, Grok Build,
  and OpenCode share this owner; Chat/Anthropic routes and unrelated hosts do
  not enter it. Resolve model mapping before model-specific request cleanup.
- Request-time vault resolution retains the admitted account lineage. A first
  upstream 401 drops its response before a single same-account refresh/replay
  inside the existing attempt and permit. Reuse the buffered request, replace
  protected bearer/routing headers and bound response-header waits. A second
  401 is terminal. Do not create another failover loop or retry a different
  account/API-key balance after subscription 401/403 or credential failure.
- Shared OpenAI compatibility headers apply to all consuming Agents. Final
  xAI token-auth, model-override and client headers replace caller copies.
  Neither upstream credentials nor raw OAuth/upstream error bodies may escape
  into renderer results, Agent configuration or diagnostics.

### Response, streaming, and usage

- `ProxyError::status_code` supplies the HTTP status for general responses,
  Codex error responses, and usage error logs. Oversized upstream bodies and
  invalid upstream status codes resolve to `502 Bad Gateway`.
- Rebuilt bodies remove hop-by-hop headers and stale entity headers. Streaming
  keeps bounded UTF-8/SSE framing across chunk boundaries and does not lose or
  duplicate the primed bytes.
- Codex-to-Chat conversion keeps commentary followed by tool calls in one
  assistant turn when safe, preserving user, tool-result, media, and existing
  tool-batch boundaries. Reasoning deduplication must retain original text and
  indentation. An empty reasoning delta opens no block; nonempty whitespace
  deltas remain meaningful and are preserved.
- Native xAI response cleanup restores tool namespaces and handles complete
  arguments even without a namespace map. Numeric normalization must prove
  losslessness from the original decimal token, preserve other tokens, and
  leave partial argument deltas untouched. Rewriting SSE data must preserve
  non-data fields, comments, and framing; it does not change usage ownership.
- Cancellation, timeout, client disconnect, stream completion, and stream
  failure each settle permit/accounting guards once. Drop guards may finish
  logging, but must not turn an incomplete response into success.
- Usage parsing is protocol-specific. Deduplication identity, resolved model,
  Provider/application identity, token categories, pricing source, and cost are
  persisted as one attribution result; missing usage remains unknown.
- Logs use stable error/event codes and bounded sanitized diagnostics. Raw
  prompts, credentials, protected headers, complete upstream error bodies, and
  unsafe response headers do not enter routine logs.

## 4. Validation & Error Matrix

| Condition                                                          | Required result                                                                      |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| listener cannot bind or report its local address                   | Start fails and status does not claim a running server.                              |
| failover is off and current Provider is absent                     | `NoProvidersConfigured`; do not consult the queue as fallback.                       |
| failover is on and every queued Provider is open                   | `AllProvidersCircuitOpen`; do not try an unqueued Provider.                          |
| a half-open permit is acquired but an attempt exits early          | Settle it once with failure or neutral release; never leak the slot.                 |
| client request/history is invalid                                  | Return the mapped client error; do not retry another Provider.                       |
| upstream transport or reviewed retryable semantic failure occurs   | Record the failed attempt and try only the next selected Provider.                   |
| non-streaming body read fails after 2xx headers                    | Treat the attempt as failure before recording Provider success.                      |
| streaming first event proves failure or cannot be read             | Do not record success; apply the reviewed retry/failover rule.                       |
| response body is rebuilt                                           | Remove stale hop-by-hop/entity metadata and emit metadata for the rebuilt body only. |
| usage is missing, duplicated, malformed, or pricing is unavailable | Preserve the response; persist only parser-proved facts.                             |
| a diagnostic contains sensitive request or response data           | Redact/drop it and keep only bounded safe context.                                   |

## 5. Good / Base / Bad Cases

- Good: Provider P1 returns a reviewed retryable semantic failure in the first
  SSE event. Its permit and health are settled once, P2 is attempted, the
  validated primed event is replayed once, and usage is attributed to P2.
- Base: failover is disabled; one current Provider handles a valid
  non-streaming request, whose complete body is read before success is
  recorded.
- Bad: mark success on 2xx headers, acquire a half-open permit during candidate
  listing, retry invalid client history, expose protected headers in logs, or
  run a second Provider retry loop inside an adapter.

## 6. Tests Required

- Server tests cover route registration, successful bind before running state,
  duplicate start/stop, shutdown, configured address/port, and status updates.
- Context tests cover every route alias, query preservation, Gemini model path,
  application identity, streaming detection, and malformed/ambiguous paths.
- Router tests cover current-only and queue-only modes, missing queue rows,
  all-open versus unconfigured errors, app-scoped breakers, selection without
  permit consumption, and neutral release.
- Forwarder tests cover retryable/terminal classification, permit settlement,
  non-streaming body-read failure, first-stream-chunk priming/replay, semantic
  2xx failures, timeout/cancellation, and protected overrides.
- Managed subscription tests cover both providers and all four CLI targets,
  official host/path assertions before mock I/O, complete stream/tool terminal
  events, tool-result replay, final policy idempotence, preserved native auth,
  exact one-401 retry and no default-account/API-key fallback. HTTP fixtures
  must honor the actual outbound stream flag; a JSON mock is not valid evidence
  for an adapter expecting `response.completed` in SSE.
- Response/SSE tests cover LF/CRLF, split multibyte UTF-8, incomplete/trailing
  events, rebuilt headers, disconnect/drop settlement, and bounded diagnostics.
- Usage tests cover protocol parsers, deduplication, model resolution, cache
  token categories, pricing source, unknown pricing, and sensitive-data-negative
  serialization.
- Run `mise run rust:test`, the affected proxy integration tests, and
  `mise run check:contracts`.

## 7. Wrong vs Correct

Wrong:

```text
2xx headers -> record success -> begin reading body
select candidates -> consume every half-open permit
adapter -> own its own retries, logs, and usage writes
```

Correct:

```text
select ordered candidates without consuming permits
attempt -> acquire one permit -> validate transport and semantic response
       -> settle once -> retry next candidate or return one authoritative result
shared response/usage owners -> sanitize, stream, attribute, and persist facts
```
