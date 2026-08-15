# UCP contract gap audit at `ca552f4d`

Evidence level: `source_report + code_audit`. No tests or runtime actions.

## Result

`BLOCKED / NOT CONTRACT-COMPLETE` for the full Issue #55 contract.

## P0 gaps

1. Only `codex_provider_switch` exists; create/edit still directly mutate.
2. Public Plan DTO lacks `schemaVersion`, independent intent identity,
   `affectedResources`, ordered actions, warnings, secretRefs, preconditions,
   and recovery hints.
3. `planDigest` covers operation, target ID, baseline digest, and contract, but
   not the complete executable semantics.
4. No secretRef missing/version drift contract.
5. Baseline precheck and writer are separated by a moving-target window:
   `ProviderService::switch` reloads the Provider by ID after admission. A target
   can drift after precheck, be written, and only then be reported as failed.
6. The old writer can affect original Provider state, current/settings/live and
   managed MCP projection, while the plan does not enumerate all affected
   resources or preserve switch warnings.
7. Product PRDs conflict: the parent requires create/edit/switch while the old
   child explicitly excludes create/edit.
8. There are no formal product, architecture, and detailed-design review results,
   no P0/P1/P2 closure record, and no `DESIGN_FREEZE=PASS` artifact.

## P1 gaps

- Expired/drift/consumed/digest failures are collapsed into one stale UI.
- Unsupported and generic create errors are conflated.
- No secret-missing state; preview does not render risks/actions/resources.
- Expiry depends on incidental React rendering rather than a timer.
- No public `get_change_plan`; renderer reload cannot rediscover an in-memory job
  naturally from the current dialog path.
- Visual reference, high-fidelity prototype, and usability review are absent.

## P2 evidence gap

Static code shows no direct Provider/model request in preview, but the old test
surface has no explicit outbound-call counter. This task adds a side-effect spy
covering network adapters as well as local writers.

## Reusable foundation

- Unique plan ID and stable same-baseline digest.
- Baseline fingerprints for current, target definition, and live projection.
- Additive v16 plan/job/event persistence.
- Atomic one-time consume and replay rejection.
- Existing Provider switch writer reuse and readback-based terminal truth.
- Thin Tauri/TypeScript/query/dialog boundary and monotonic events.

The follow-up extends these owners; it does not build a second ledger, job state
machine, or Provider writer.
