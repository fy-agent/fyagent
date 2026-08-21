# Design

## Boundary

- Recover the real-request path from `a5903d86^` `services/stream_check.rs` into a new `model_probe` service.
- Keep current `stream_check.rs` as URL reachability. Do not merge the two, and do not reset the circuit breaker.
- V2 Models UI stops calling `checkReachability` / `stream_check_url`.

## Protocol

| App | Request |
|---|---|
| Claude | Anthropic Messages `POST .../messages`, `x-api-key` |
| Codex | OpenAI Responses `POST .../responses`, Bearer; origin-only URLs try `/v1/responses` first |
| Grok Build / WorkBuddy / OpenCode | OpenAI Chat Completions `POST .../chat/completions`, Bearer |

Draft-only input: `{ app, baseUrl, apiKey, modelId }`. Empty API key is allowed (WorkBuddy/OpenCode 「不使用 API Key」); the auth header is omitted.

Timeout 30s, one retry on timeout/abort only. First-token TTFB over 6000ms is degraded. Prompt is a short `ping`. First SSE chunk without an error event is success.

Error `message` is `HTTP {status}: {truncated body}` or the transport error, with the API key redacted. `errorCategory` may be `modelNotFound` or `quotaExceeded`.

## UI

`src/v2/pages/models/ModelConnectivityTest.tsx` owns the button, dialog, search, group filter, chip picker, and result `FieldFeedback`. Parents pass `modelIds` and `onProbe(modelId)`.

Codex and Grok Build gain the same 「拉取模型」 + chips as Claude so they can produce a selectable list.

## Out of scope

Qoder/TRAE, V1 provider-card reachability, restoring OpenClaw/Hermes/Gemini/Copilot probes, circuit-breaker reset.
