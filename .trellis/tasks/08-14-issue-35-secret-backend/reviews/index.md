# Issue #35 review index

`DESIGN_FREEZE=PENDING`.

## Initial static snapshots

These files preserve the first working-tree review and remain immutable historical evidence. They are `REQUEST_CHANGES` inputs, not the final review authority:

- `product-review.md`
- `architecture-review.md`
- `detailed-design-review.md`

`review-disposition.md` maps their findings to revised authority but does not close them.

`v6-working-tree-audit.md` and `v7-working-tree-audit.md` record later stable-hash pre-commit audits that still returned `REQUEST_CHANGES`; both are revision input only. `v9-working-tree-audit.md` records the final stable working-tree hash set and three independent `P0=P1=P2=0` closures. It authorizes creation of a design candidate commit only and cannot substitute for same-commit authoritative rereviews.

## Authoritative rereviews

Candidate `D=f2f26b8b6b5aa4acf8bbd257cee9ee22713aebaf` was read immutably:

- `product-rereview.md` — `APPROVE`, `P0=P1=P2=0`
- `architecture-rereview.md` — `REQUEST_CHANGES`, `P0=0/P1=3/P2=0`
- detailed rereview was not run after architecture rejected `D`.

`D` is not design authority and cannot be frozen. Its receipts remain immutable history. The next candidate `D2` must receive three new files:

- `product-rereview-d2.md` — `PENDING`
- `architecture-rereview-d2.md` — `PENDING`
- `detailed-design-rereview-d2.md` — `PENDING`

All three must name the same immutable design candidate SHA and report `P0=0/P1=0/P2=0`. Any design correction creates a new candidate SHA and invalidates all three prior rereviews. The freeze receipt must point only to the latest same-SHA rereviews while retaining every earlier snapshot for provenance.
