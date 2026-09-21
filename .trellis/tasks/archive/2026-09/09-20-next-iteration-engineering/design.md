# Design

## Baseline and isolation
Use immutable candidate 2c09c4be2c5f50b4060fd6f7a13a7be31c364c54 while the concurrent release owner merges 0.4.6. Reconcile against final main before integration. Preserve the original dirty checkout and other branches. One writer per worktree; root owns integration and tracker.

## Workstreams
1. Source links/About and first-use entry (#25 #70 #34) in existing renderer/catalog owners.
2. Production SecretRef/provider lifecycle (#35), followed by shared presets/reuse/auth (#40 #42 #43) to avoid competing Provider writers.
3. Installation preflight and ordinary-user/helper recovery (#27 #29).
4. Existing configuration, exact preview and write/proxy recovery (#47 #56 #61 #64).
5. Versioned selected configuration import/export (#73), reusing existing parser, Change Plan, and native persistence.
6. Current fixture demo artifacts (#92) after UI freezes.
7. Release roles and Authenticode (#67 #68): inspect code gates, implement genuine code gaps if any; retain external evidence dependencies.

## Contracts
Use existing domain/port/Tauri/service layers. No new orchestration framework, inferred device entitlement, weakening of signing or path protection, request execution during passive discovery, or plain secret exports. Renderer uses existing design tokens and interaction components.

## Integration and rollback
Each worker supplies a scoped commit, exact checks, unresolved requirements, and an evidence report. Root reviews the delta and checks dependencies before cherry-pick. Revert only this task's accepted commits when necessary; never reset shared state. Model and job identity plus lifecycle and acceptance are separate.
