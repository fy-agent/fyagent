# Design

Start from the stable B3 commit. Keep the Raft A/B writer and the omp writer in their own worktrees. Cherry-pick their stable commits into this integration branch.

ConnectionResults already owns persistent feedback by target. Suppress duplicate connection toasts at the existing runMutation boundary, while preserving account-level notifications. Reuse all existing retry and error state.

A and C native/renderer contracts remain owned by their implementation tasks. Resolve integration conflicts without broadening product scope. Actual Windows GUI acceptance remains distinct from mocked and headless checks.
