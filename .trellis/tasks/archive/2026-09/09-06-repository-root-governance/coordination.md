# Execution ownership

Root governance is being implemented only on isolated branch
`refactor/round7-root-governance`, worktree `fyagent-df58b1c3`, from91b37c8c.
Do not modify root/config/task-runner files concurrently in the main checkout.
Main-checkout Dialog work is left intact. The isolated commits will be reviewed
and merged after both work sets are verified; parent archival waits for that.

The supplemental `scroll-ownership.spec.ts` changes in main belong to the scroll
reviewer. They extend all-seven-route physical wheel coverage and are still
being verified; do not include them blindly in a Dialog work commit.
