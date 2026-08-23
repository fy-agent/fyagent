# Issue 55 approval binding hardening implementation

1. Add failing tests for stored-row tampering and malformed/missing live
   baseline behavior.
2. Introduce a complete approval-binding input and recheck it before admission.
3. Replace the live availability boolean with a closed three-state baseline.
4. Freeze the V2-facing status variants and update the shared fixture/spec.
5. Run focused and full gates, then fast-forward the existing PR branch only
   after remote-head readback.
