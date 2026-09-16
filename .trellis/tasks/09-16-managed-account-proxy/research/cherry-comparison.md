# Cherry Studio comparison and research

## Input identity

User-supplied `cherry-studio-2.0.14.zip`, root `cherry-studio-2.0.14/`.
SHA-256: `b69bc513d97c4fced35a05c4ac5dbca299cc56be8d13636c061d967453c14fbe`.
The supplied source, not a changing main branch, is the architectural reference.

The archive's `LICENSE` is GNU AGPL v3. FyAgent's root license distinguishes
PolyForm Noncommercial additions from retained MIT-derived portions. No Cherry
implementation file or runtime dependency is imported: this work independently
implements the required protocol behavior in existing FyAgent Rust owners.
Do not describe an architectural comparison as a compatible license grant.

## Reference owners

| Cherry source | Useful pattern | FyAgent decision |
| --- | --- | --- |
| `src/main/services/oauth/runtime/OAuthRuntimeService.ts` | Per-session refresh singleflight, expected-token compare-and-set, one forced refresh after 401, release old response before replay | Extend existing per-credential lock/generation/SecretRef resolver; no second cache/store |
| `src/main/services/oauth/runtime/OAuthTokenStore.ts` | Reject stale refresh after logout/new login; distinguish terminal vs retriable | Preserve purpose/owner/generation through proxy refresh and fail closed |
| `src/main/ai/provider/codex.ts` | Request-time bearer, ChatGPT routing header; Responses store=false and supported fields | Share managed OpenAI adapter for both Codex/Grok Responses and Claude conversion |
| `src/main/ai/provider/grokCli.ts` | CLI subscription origin, native Responses, model routing header, instructions/reasoning normalization | Switch managed xAI only to native Responses and extend existing sanitizer |
| `src/main/ai/provider/config.ts` | Reuse SDK/transport with a small authenticated fetch adapter | Reuse existing reqwest/Axum/Provider/SSE stack; no Electron/AI SDK runtime migration |
| `src/main/features/apiGateway/ApiGatewayService.ts` | Desired enablement vs actual listener state; serialized reconciliation | Reuse ProxyService lifecycle/activation guard; avoid connected-from-login claims |
| `src/main/features/apiGateway/middleware/auth.ts` | Local gateway key is separate from upstream OAuth and checked by protocol header | Do not export upstream credentials. Existing loopback managed-proxy boundary retained; no new public gateway/per-client auth product |
| `src/main/features/apiGateway/utils/models.ts` | Route identity separate from upstream raw model; catalog matches routable providers | Keep existing per-Agent Provider namespaces and explicit account/model binding |
| `src/main/features/apiGateway/proxyStream.ts` | Shared dialect conversion and cancellation/terminal handling | Keep existing converters, usage and circuit accounting rather than clone stream parser |
| `src/main/services/codeCli/CodeCliService.ts`, `src/shared/utils/cliConfig*` | Per-tool configuration, gateway mode vs direct mode, validate model/target | Reuse target writers, confirmation/readback/rollback; direct account projection unchanged |

## Primary online references, checked 2026-09-16

- Cherry official repository and source provenance: https://github.com/CherryHQ/cherry-studio . The uploaded release snapshot takes precedence over mutable main.
- OpenAI custom provider configuration: https://developers.openai.com/codex/config-file/config-advanced . `wire_api="responses"`; custom provider IDs cannot reuse reserved `openai`, `ollama`, `lmstudio`.
- OpenAI config reference: https://developers.openai.com/codex/config-reference . Authentication and source configuration are distinct; preserving official native auth is essential.
- Grok Build official overview: https://docs.x.ai/build/overview . Custom model entry uses `~/.grok/config.toml` / GROK_HOME, explicit base URL and model identity.
- Grok Build settings, updated 2026-08-31: https://docs.x.ai/build/settings . `api_backend="responses"` is an explicit supported setting; MCP and unrelated settings must be preserved.
- Grok official source: https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-shell/README.md . Custom provider credentials are separate from first-party session auth; failing custom auth must not fall back to xAI session credentials.

## Concrete gaps

1. `managed_auth/service.rs::upsert_proxy_connections` writes Connected from a default Ready credential and skips missing defaults, so it is neither listener nor consumer proof and can be stale.
2. `provider/managed_xai.rs::parse_request` excludes Grok Build and only accepts xAI; managed OpenAI accounts cannot follow the same target binding workflow.
3. `proxy/providers/codex.rs` forces managed xAI to Chat and does not pin managed OpenAI auth/base in the Codex adapter. The existing xAI native namespace/sanitizer branches are unreachable under that forced route.
4. `ManagedAuthService::resolve_credential_access` reuses a newer generation after stale refresh rather than rejecting the in-flight lineage, and lacks same-token 401-triggered refresh. Terminal refresh status needs authoritative persistence.

## Reuse assessment

No additional framework, crate, daemon, token store or parser is needed. The language/runtime differ; adapt responsibilities and narrowly specified protocol behavior while preserving the existing Rust owners. Existing xAI binding identities, native account flows, API-key providers and per-target backup ownership are compatibility constraints, not migration casualties.
