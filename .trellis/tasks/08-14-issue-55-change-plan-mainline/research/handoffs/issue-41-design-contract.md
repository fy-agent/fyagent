# Issue #41 design-contract handoff — docs only

```text
schemaVersion=1
handoffKind=design_only_non_consumable
status=delivery_pending_readback_pending
DESIGN_CONTRACT_HANDOFF_SHA=d158b27690d897e8e9f2ece7d8887da6423b899c
producerBranch=codex/issue-55-change-plan-mainline
baseMainSha=4b4e17540ad8ddd564bb7ef7c5ca2a31b7c36287
ucpHandoffSha=6859e9ce04970008f4cf8b3d4883b4f70316291a
ucpSourceImplementationSha=ca552f4d918cacc734f81f7efdef70619da139b8
designFreezeManifestSha256=2c1b753acf470699b428ae8a9eb401be82dbb0f1f19d9c13a6268350d4edfc5f
designFreezeReceiptSha256=0f1ea11d1fa96f81d48860684ed44678125bbc5506878cf8aef179bcad80967b
consumerIssue=41
consumerThreadId=01a0004d-52f1-7a30-a137-730bd102c0a1
createdAt=2026-08-15T10:11:47+08:00
lastDeliveryAttemptAt=2026-08-15T10:11:47+08:00
deliveryResult=thread_transport_timeout_no_success_claim
readbackResult=not_received
```

## Consumption boundary

This receipt lets Issue #41 plan against the frozen product and design
contract. It is deliberately **not** a source-integration handoff. The SHA does
not yet contain the reachable v2 Rust/TypeScript/DAO/worker/registration seam,
and #41 must not compile, cherry-pick, or integrate against it.

The first consumable authority will be a different immutable SHA recorded in
`issue-41-consumable-contract.md`. It is valid only when that one SHA contains
the strict Rust DTO and canonicalization, Rust-authored fixtures, v1/v2
persistence and read APIs, admission/worker/CAS/event/recovery APIs, Provider
guard, strict TypeScript decoder/query, exact command registration, focused
green gates, producer seam review, and #41 exact-SHA compatibility readback.

The Codex thread transport timed out during the recorded delivery attempt. No
message receipt or consumer acknowledgement is claimed. Delivery/readback is a
non-blocking administrative open item for #55; it remains a blocking item for
calling this notification delivered.

## Frozen contract for #41 planning

- **Identity and lifetime:** a Plan is a saved, immutable, side-effect-free
  snapshot with stable `planId`, `planDigest`, `schemaVersion`, `createdAt`,
  `expiresAt`, intent, baseline fingerprint, affected resources, ordered
  actions, risks/warnings, opaque secret references, preconditions, and
  recovery hints.
- **Canonical digest:** one schema-dispatched canonical JSON encoder owns intent,
  baseline, private-envelope, and Plan digest construction. Unknown fields,
  unknown enum values, non-canonical numbers, and non-finite values fail
  closed. No UI/DAO consumer reconstructs digest defaults.
- **Baseline and resources:** baseline identity binds the exact app/provider,
  resource key, source/version/epoch, target structural projection and ordered
  mutation set. Provider row, endpoint set, DB current, device current, Codex
  catalog/auth/config, common config, managed MCP, and conditional source
  backfill remain independently fingerprinted/read back when applicable.
- **Persistence/read:** #55 owns the only Plan/job/event/coordination ledger.
  It provides schema-dispatched v1/v2 reads, scope discovery, lifecycle and
  revision CAS, retention/purge, and durable job reconciliation. #41 consumes
  these APIs and must not create a shadow ledger or parallel admission record.
- **Invalidation:** expiry, target/source/baseline/resource/precondition or
  secretRef metadata change invalidates the saved Plan and forces a new
  preview. Closed reasons include `expired`, baseline/resource fingerprint
  mismatch, target/source changed, precondition changed, secret reference
  missing/version changed/backend unavailable, unsupported mode, consumed,
  abandoned, and stale/replayed identity.
- **One confirmation:** the renderer confirms exact `planId + planDigest` once.
  Admission re-decodes the stored private envelope, verifies its digest and
  current baseline, then atomically consumes the ready Plan and creates one
  owning durable job. Apply never recomputes form intent or asks for a second
  product confirmation.
- **Execution boundary:** preview performs no Provider/model outbound request,
  no file/live-config/current/tray/cache/event/job/backup mutation, and does not
  execute a Plan. After admission, only a private one-use Provider effect permit
  can reach the existing writer; readback and recovery classify outcomes
  without replaying ambiguous effects.
- **Dependency boundary:** Issue #35 remains un-frozen. Until an owner-declared
  immutable handoff is compatible, only a narrow material-free `SecretRefPort`,
  synthetic fixtures, and typed-disabled secret-bearing operations are allowed.

## Frozen byte authority

The manifest below is copied from the signed freeze receipt. Each digest is the
SHA-256 of the exact file at `DESIGN_CONTRACT_HANDOFF_SHA`.

| SHA-256 | Repository-relative path |
| --- | --- |
| `dce6a6273025341e6ce14d7b67c42a9de012d2871d9df4258f512bad6c0952c7` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/prd.md` |
| `0e76ace18e937b7901a54acdabf64a4875d037009fdfe31cf06820b289477c3a` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/process-state-machine.md` |
| `c32072a1431d7d3c740f17a32f13547ccca4159f3c911190e652656d69676ce8` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/design.md` |
| `7284e5a734ed9c9317fba2be9168c4be9518a1d6cb37ef8d996a646b5a198c83` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/detailed-design.md` |
| `e3fc521ee0be99a76349b7387c4ffdbbb5f70e2f11289ed56b02da3752f97f51` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/implement.md` |
| `02f977a55080a902a54597fc74f75ed800ea6599f7c9744f1220953b37e056ea` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/reviews/product-review.md` |
| `205f5dc061138a9f1be591ef79216d96e76712215faf2a1ff62f04079b50b7dd` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/reviews/architecture-review.md` |
| `34f63a6267717dbf374b1f4b493abea43ff7a44af39f81480228ecfd8e8b5cdf` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/reviews/detailed-design-review.md` |
| `be2bdea8bdcfe843fd5095a73a3b8afdf76500fa15096cdb823ed3a31fc605e6` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/research/ownership-and-handoff-audit.md` |
| `7b370c617615c36bf3203c9707390490215ade8ab7072badd3cc43531717c9c6` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/research/ucp-contract-gap-audit.md` |
| `30ab0dd7f90720e56a3396be5270868feaadbefb9eb4cfb4336b09979e4ff449` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/research/dependency-contract-audit.md` |
| `fcaffa4e438aa78cab0649a7f9fd267a0de841ca18d7befbc73a8b896771072e` | `.trellis/tasks/08-14-issue-55-change-plan-mainline/research/provider-create-edit-code-map.md` |
| `f38f006aaf70ec436717e6c38321d43b96bad5683373a96c01d1ac6caca69730` | `.trellis/spec/backend/unified-change-plan.md` |
| `6a561cc32d0e18ce1ea3017b2b92648622dd0884eef2e4d73fa14a2e545ca8fd` | `.trellis/spec/backend/codex-provider-configuration.md` |
| `6026141e3a8f66d66f18a7ad2022ca28b0f7e75219616bd6f3415cfd4522fdc0` | `.trellis/spec/backend/deeplink-import-security.md` |
| `62c4c68cb0490209f972ad299c0df7d247f2f1d230b42ce73d71e2763eebe5e3` | `.trellis/spec/backend/task-runner-contract.md` |
| `de551f5c5e2cda6bf1ce75f83f53e61e99495ebaf2a6d2f5fd380f789ec72b18` | `.trellis/spec/backend/index.md` |
| `9d84937783247c97003e6550f4b65e204fa00fa1a022dc99bff2975cd3606f19` | `.trellis/spec/frontend/index.md` |

## Readback fields

These remain unset until the exact consumer thread returns them:

```text
ackSha=PENDING
consumerBranch=PENDING
consumerBaseSha=PENDING
compatibilityStatus=planning_only_not_evaluated
producerReview=0_p0_0_p1_0_p2_at_design_freeze
consumerReview=PENDING
seamFindings=PENDING
```
