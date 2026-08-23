# Harden Issue 55 approval binding

## Goal

Close PR #130 review gaps: immutable approval rebinding, safe live baseline classification, and frozen V2 status enums.

## Requirements

- Recompute the stored plan's non-secret approval binding immediately before
  admission. A mutation to any immutable execution or preview field must make
  the plan stale and call the Provider writer zero times.
- Distinguish a known-missing Codex live baseline from an unreadable or invalid
  baseline. Missing may be planned and applied if it remains missing;
  unreadable/invalid must not create a plan or reach the writer.
- Reject proxy-takeover state before plan persistence until this slice has a
  proxy-aware target projection and readback authority.
- Freeze `cancelled`, `warning`, and `not_started` wire enum values now so the
  stacked V2 client does not invent incompatible local states.
- Preserve process-private secret-bearing proofs and schema v20. Do not add a
  second writer, schema revision, frontend implementation, or generic engine.
- Fast-forward PR #130 only if its remote head is still the reviewed
  `9b4599db7f2ae79d957cdf257f44111e9dd7377d`.

## Acceptance Criteria

- [x] Tampering `expiresAt`, `sourceVersion`, target display metadata, or
      contract identity after preview yields `stale`, creates no job, and
      calls the writer zero times.
- [x] Wrong caller digest, expiry, replay, and ordinary baseline drift remain
      zero-writer rejections.
- [x] A known-missing live baseline is bound distinctly and can be applied only
      while still missing; malformed/unreadable live state is rejected before
      plan persistence or apply admission.
- [x] DTO fixture contains the frozen `cancelled`, `warning`, and
      `not_started` spellings.
- [ ] Focused Rust tests, format, clippy, full Rust tests, repository contracts,
      full `mise run check`, and Required CI pass at the final PR head.
- [ ] PR #130 has a complete recovery/salvage description and Issue #55 remains
      open for the broader #56/#57/#58/#41 acceptance surface.

## Notes

- This is a review-hardening increment on top of PR #130, not a replacement
  branch or an independent product slice.
