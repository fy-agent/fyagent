# Initial SPEC audit — 2026-09-14

## Baseline

`dev/laiyongjie` is six commits ahead of `origin/main` and four commits ahead of
`origin/dev/laiyongjie`. The worktree was clean before this audit task was
created. The six commits comprise two npm/tooling commits, the Renderer
refactor, a repository snapshot-test stabilization commit, the prior task
archive, and its journal record.

## Injection-size findings

Trellis defaults `context_injection.max_file_bytes` to 32768 bytes. Current
changed SPEC sizes include:

| File | Bytes | Finding |
| --- | ---: | --- |
| `backend/external-agent-lifecycle.md` | 40882 | Must split; product source/identity is separable from inventory/job orchestration. |
| `backend/task-runner-contract.md` | 50971 | Must split; supported-platform identity seals are a cohesive repository-governance owner. |
| `backend/windows-runtime-security.md` | 50781 | Must split; Agent helper and registry scenarios are separable from the frozen Shell-user core. |
| `backend/claude-code-cli.md` | 14155 | Safe size; current live npm details fit the mandatory seven-section form. |
| Largest changed frontend owner | 19954 | Safe size; review for duplication, not size pressure. |

Raising the configured limit would only hide truncation risk and is rejected.

## Duplication findings to verify

- Live npm resolution and Windows LocalProcess details are repeated in
  `claude-code-cli.md`, `external-agent-lifecycle.md` and
  `windows-runtime-security.md`. One file must own mechanics; the others should
  state only orchestration/platform boundaries and link to the owner.
- Concise secondary-surface rules appear in user copy, visual language,
  surfaces, Skills/MCP and Prompts/Memory. Feature owners should state their
  concrete projection; shared rationale belongs only in copy/visual/surface
  owners.
- The repository-wide snapshot timeout belongs to test/governance behavior,
  not product performance policy. Its SPEC must preserve zero-findings and
  inspected-file assertions while separating runner scheduling from scanner
  budgets.

## Review risks

- A live npm resolver must never turn a registry tag into npm argv; the install
  plan must carry a concrete version and matching root/platform integrity.
- macOS discovery must not walk manager internals and must select the PATH
  default when multiple copies exist.
- Development Windows must not execute the command-shape `1.2.3` fixture and
  npm 12 must receive the narrow Grok allow-scripts flag.
- React warning guards can be silently disabled by `restoreMocks`; setup tests
  must exercise consecutive cases against the real lifecycle.
- Browser/performance configuration must not retain traces in accepted timing
  runs or replace real motion/scroll evidence with unit mocks.

## Final decomposition

The review resolved the oversize/duplication findings without changing
`context_injection.max_file_bytes`:

| Owner | Final bytes | Responsibility |
| --- | ---: | --- |
| `external-agent-lifecycle.md` | 21425 | Action legality, normalized inventory, opaque capabilities, jobs, deployment and recovery. |
| `external-agent-sources.md` | 14050 | Product release sources, shared Claude/Grok exact npm admission, artifact bounds and closed Desktop identity. |
| `task-runner-contract.md` | 26189 | Public mise API, effects, validated transport, composition and canonical checks. |
| `trellis-prearchive-gate.md` | 5775 | Exact direct-session active-task exclusion before archive. |
| `supported-platform-governance.md` | 8246 | Platform-sensitive source/raster identities and one-snapshot repository scans. |
| `native-task-runner.md` | 13856 | Foreground trees, Windows MSVC child environment and macOS signed development runner. |
| `windows-msvc-cross-diagnostic.md` | 7656 | Optional macOS cross-compile diagnostic and its non-acceptance boundary. |
| `windows-runtime-security.md` | 24434 | Frozen Explorer-user authority, hidden paths/elevation and general HTTP COM launch. |
| `windows-agent-runtime-security.md` | 11154 | Trusted Agent EXE launch, closed Claude/Grok helper routes, LocalProcess parity and registry enumeration rights. |
| `github-ci-workflow.md` | 29922 | PR/merge-group Required CI classification, domains and aggregation. |
| `github-push-commit-policy.md` | 5588 | Push-only unreachable-base fallback and topology-aware subject policy. |

All affected owner documents are below 32768 bytes with durable routing from
`backend/index.md`. A local Markdown link scan reported zero missing targets;
the curated task implement/check contexts validate without truncation.

Shared live npm source authority is now singular: `external-agent-sources.md`
owns metadata/mirror/integrity/exact-plan mechanics, Claude/Grok focused owners
own product-specific detection/execution/post-verification, and the lifecycle
owner consumes only admitted capabilities/outcomes.
