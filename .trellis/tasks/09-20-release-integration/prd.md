# FDE and subscription delivery integration

## Request

The same delivery includes yesterday's completed FDE work and subscription reuse.
The user authorizes two clearly explained PRs, fixing conflicts and failed remote
checks, and following GitHub checks through completion rather than handing off a
failing PR. This continues the authorized implementation and delivery scope.

## Acceptance

- FDE has its own PR with accurate scope, validation and remaining UAT boundaries.
- PR #192 includes the FDE prerequisite and subscription changes without losing
  either implementation; its description explains order and combined scope.
- Existing FDE and subscription schema-22 databases upgrade safely to the combined
  schema, preserving data; schema 21 and fresh databases are also covered.
- Existing CI failure is reproduced and fixed without weakening checks.
- Both PRs have no unresolved merge conflicts and all required GitHub checks pass
  at their final heads. Review findings and meaningful non-blocking risks are read.
- Shared dirty checkout, installed applications and real account data are preserved.
  No merge to main, formal release or real-account success is implied.
