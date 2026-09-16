# Design

## Boundary

只扩展代理用途链路。继续使用一个 `ManagedAuthService`、一个 `ProxyService/ProxyServer`、一个 Provider 路由与现有协议转换。上游授权固定来源，配置文件仅接收 loopback endpoint 与本机代理凭据/占位符，不接收 OAuth token。原生 `managed_auth/consumers/*` 与登录投影不作为重构目标。

## Data flow

```text
Managed Auth proxy-purpose login (existing)
  -> credential identity + OS SecretRef + FyAgent refresh ownership
  -> generalized managed-proxy binding (public account ID + target + model)
  -> existing Provider transaction / Codex Change Plan
  -> one loopback ProxyService -> target-specific config writer + readback
Agent Responses / Messages
  -> existing app-scoped router and circuit permit
  -> provider-pinned request shaping + current credential resolution
  -> official subscription endpoint
  -> existing response/SSE/tool/usage pipeline
```

## Owners and changes

- `services/managed_auth`: general proxy account admission, safe refresh reconciliation, no native refresh-owner changes; connection summary distinguishes credential availability from listener readiness.
- `services/provider/managed_xai.rs` and a narrow shared managed-proxy owner: keep old command façade and stable xAI IDs; generalize pure source construction and existing transaction, not a second writer. OpenAI IDs are separate from reserved Codex built-in `openai`.
- `services/proxy.rs`: existing guarded managed activation also accepts Grok Build; snapshot/backup/readback/rollback are still target-owned.
- `proxy/providers`: managed OpenAI support in Codex adapter, xAI native Responses selection, shared final request shaping. Adapt architectural patterns, not a wholesale TypeScript copy.
- `proxy/forwarder.rs`: request-time credential resolution and bounded same-account 401 replay inside the existing attempt, prior response dropped before retry; no parallel retry/failover loop or token cache.
- Existing Models subscription component and typed Ports: extend the current account/model/target workflow, keep modal confirmation, authoritative query reread and unknown-state protection. Native account projection controls are not replaced.

## Compatibility and risk

- Keep existing `bind_xai_managed_provider` DTO and IDs so old clients/saved sources work; generic façade resolves provider from the admitted public identity.
- New managed OpenAI sources must not be confused with the official Codex source that forwards native client authentication.
- The uploaded Grok CLI source is the implementation reference for subscription transport; public xAI API-key docs are not proof of subscription entitlement. The API-key route remains untouched.
- Preserve existing namespace/tool/SSE converters and extend them with deterministic request normalization only after editable overrides, so forbidden fields cannot be reintroduced.
- Do not add dependency or plaintext token storage. Existing loopback-only managed activation remains the exposure boundary; a new public/network gateway or per-client authorization product is not introduced.
- Target readback is local evidence, not a real upstream invocation. Production-account and non-host OS tests stay explicit unverified boundaries.

## Rollback

Provider/live-file/runtime mutations use existing compensation snapshots and lock order. Incomplete restore stays state-unknown. Code changes are one scoped work commit after validation; existing xAI compatibility façade remains available. Do not delete backups or overwrite user auth to force a green status.
