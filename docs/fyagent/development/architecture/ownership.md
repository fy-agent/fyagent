# Current-state ownership

FyAgent keeps current engineering knowledge in three layers:

1. Code, configuration, tests, and workflows define executable behavior.
2. Development documents explain how responsibilities connect and where to
   operate, validate, or debug them.
3. Retained `.trellis/spec/` notes provide optional AI-assistance checklists
   and design context.

If these layers drift, executable behavior and its tests decide what the
repository actually does. Update the maintained explanation in the same change
when that behavior is intentionally changed. Optional AI notes never become a
contributor, build, CI, or release prerequisite.

## Cross-layer map

```text
Developer operation
  -> guarded mise task
  -> implementation or validation leaf
  -> targeted test evidence

Pull request / merge group
  -> repository change classifier
  -> affected domain jobs
  -> CI / Required

Formal release source
  -> stable vX.Y.Z tag at the intended commit
  -> native builds and evidence
  -> transactional public Release

Elevated Windows host
  -> freeze Explorer Shell SID and user directories
  -> immutable Shell-user context, including Bob/Alice UAC
  -> Codex Desktop ordinary-user lifecycle
  -> context-preserving restart or launch
```

Retained backend and frontend indexes under `.trellis/spec/` remain optional
prompts for reviewing these boundaries. The responsibility map in the
[development index](../README.md), current implementation, and executable
tests are the normal contributor route.

## Context routing

Normal implementation work reads, in order:

1. the user request, issue, or agreed planning artifact;
2. the nearest maintained development explanation;
3. current code, configuration, workflows, and tests;
4. optional AI-assistance notes when they help expose edge cases.

Archived tasks and Git history are consulted only when the work is explicitly
historical, forensic, or provenance-oriented. A prior decision never
overrides current implementation and executable evidence.
