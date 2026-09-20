# Execution and verification

1. Read FDE delivery evidence and verify remote branch, original dirty checkout,
   PR status, failing job logs and relevant repository contracts.
2. Open the FDE PR after source/evidence checks; monitor its actual GitHub run.
3. Merge FDE into the subscription branch. Assign non-overlapping backend and
   frontend conflict resolution; main session owns release/docs/contracts.
4. Implement the combined migration and meaningful compatibility regressions.
5. Fix workstation-path failure and reconcile exact source/ACL/startup contracts.
6. Run canonical prearchive checks, browser integration and independent review.
7. Archive this task and run postarchive contracts, commit/push only owned changes.
8. Read both final-head GitHub checks and reviews, fix failures and repeat affected
   checks. Explain two-PR dependency and retain explicit real UAT/release boundaries.

Rollback means reverting an integration commit in the isolated branch; it never
means changing shared checkout state or applying a candidate to real user data.
