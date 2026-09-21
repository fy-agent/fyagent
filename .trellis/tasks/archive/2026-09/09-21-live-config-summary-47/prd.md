# Actual target configuration summary #47

## Goal

Expose a bounded read-only summary of the target configuration files independently
from saved DB Providers for first-use keep/replace decisions.

## Requirements

- Return actual model, endpoint and explicit protocol when safely readable,
  together with target identity, file existence and read/configuration state.
- A missing, malformed, unsafe or unreadable live target must not discard the
  saved Provider list.
- Use existing native read/target owners; never resolve secrets, call a model
  endpoint, write configuration or infer live state from DB currentId.
- Runtime parser rejects secret/unknown fields inside the live projection while
  retaining an independently valid saved summary.
- Cover external state differing from DB, missing/empty/malformed file and
  credential collisions with native temporary-file tests and focused port tests.

## Acceptance

- [x] Native projection and independent failure semantics implemented.
- [x] Closed DTO and runtime parser implemented with negative tests.
- [x] Focused renderer/architecture/type/lint checks recorded.
- [x] Native compilation/testing explicitly handed to root.

Implementation completion is not native/runtime or UI acceptance. See
`validation.md` for the pending integration checks.
