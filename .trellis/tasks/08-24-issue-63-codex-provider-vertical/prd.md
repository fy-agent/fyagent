# Issue 63 Codex Provider vertical

## Goal

Route the V2 Codex quick-setup create/edit operation through the same UCP
preview, one-confirmation, five-phase execution and readback surface already
used by existing-Provider switch, while persisting only SecretRef metadata in
FyAgent-owned storage.

## Locked contract

- One plan has the explicit business steps `save_provider` then
  `set_current_provider`; the user confirms that concrete plan once.
- Preview may add one UCP ledger row. It writes no Provider, Keychain, live
  Codex file, job/event, or network target.
- The submitted API key is converted to zeroizing process-private material.
  The public plan and SQLite contain only a random `secretRef`, opaque version,
  OS-backend code, and safe display/status fields.
- Apply writes and verifies the new OS-keyring entry first, calls the existing
  quick-setup Provider writer once with distinct persisted and transient-live
  Provider projections, then readbacks DB current, device current, sanitized
  Provider definition, and Codex live projection.
- The Provider row contains `secretRef/version`, never the raw API key or a
  secret-derived digest. Codex's external native files may contain plaintext;
  preview names that limitation without exposing an absolute path.
- Edit rotates to a new reference. Failure preserves the old reference; after
  successful dependency switch, old-reference deletion failure is a warning
  with manual recovery truth.
- Apply performs no connectivity check, model request, or proactive network
  validation. Usage evidence remains `not_observed`.

## Acceptance criteria

- [ ] Create and edit preview are side-effect-free outside one UCP ledger row.
- [ ] Public/DB/event/log/export surfaces contain no submitted API-key canary,
      secret hash, or private proof.
- [ ] Plan preview shows `save_provider` and `set_current_provider`, a shortened
      SecretRef projection, external plaintext limitation, restart and recovery.
- [ ] Exactly one confirmation admits one execution and one existing Provider
      writer call; duplicate apply returns the same job without another write.
- [ ] Provider/Codex drift, API-key material drift, expiry and lost process
      proof all stop before Keychain or Provider writes.
- [ ] DB persists only SecretRef metadata while live Codex readback contains the
      resolved material expected by the approved plan.
- [ ] DB/live failure accurately reports rolled back, warning, or
      recovery-required state; old credential remains usable until success.
- [ ] The existing switch path can resolve a SecretRef-backed Provider and
      retains its previous legacy-provider compatibility.
- [ ] V2 uses the shared preview/job UI; Codex quick setup no longer calls the
      direct mutation command.
- [ ] Focused tests, V2 gates, native macOS UAT, full `mise run check`, and
      final-head Required CI pass. Issue #63 stays OPEN until dependency HIL and
      merge evidence are complete.

## Closure checklist

1. Freeze safe backend/IPC DTO and v20 persistence delta.
2. Implement process-private draft, keyring admission, existing writer reuse,
   readback and failure classification.
3. Route the V2 Codex form through the shared plan controller and add closed
   parser/component tests.
4. Run canary, fault, idempotency, frontend, full-repository and native gates.
5. Push a narrow stacked Draft PR, attach exact evidence, and leave #63 OPEN.
