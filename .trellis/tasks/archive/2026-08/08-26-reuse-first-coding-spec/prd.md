# Make reuse-first coding spec

## Goal

Make reuse the default engineering decision across FyAgent so future work
searches existing project owners and suitable open-source modules/components
before implementing a new local solution.

## Requirements

- Prefer an existing in-repository owner or an already-adopted dependency over
  introducing a parallel implementation.
- When the repository has no suitable owner, require an explicit search for a
  mature open-source module/component before choosing a bespoke implementation.
- Keep dependency adoption conservative: compatibility, maintenance, license,
  security/provenance, platform support, dependency cost, and project-boundary
  fit must be reviewed before adding a new dependency.
- When implementation or review discovers a capability that has multiple
  plausible consumers, promote or propose one shared/public project component
  at the correct boundary instead of allowing page/service-local copies to
  accumulate.
- Preserve current FyAgent architecture boundaries: frontend shared components
  stay in the owning shared layer; backend reusable owners stay crate-scoped by
  default and expose only the minimum stable facade required by callers.
- Do not force abstraction for genuinely one-off code, and do not add a large
  dependency when a current owner or small existing primitive already solves
  the problem.
- Do not elevate a personal development branch into a repository-wide concept
  in maintained SPEC, docs, CI descriptions, or operator-facing explanatory
  text. Historical archived Trellis records are out of scope.
- Release preflight must not use a personal development branch as its trusted
  authority. The trusted workflow remains on `main`, an explicit immutable
  candidate SHA selects what is diagnosed, and preflight candidate execution
  must not receive formal release signing/notarization secrets.

## Acceptance Criteria

- [x] The code-reuse thinking guide defines a repository-wide reuse decision
      order and an open-source candidate review step.
- [x] Frontend reuse SPEC makes existing/shared/open-source reuse the default
      before bespoke UI/helpers and requires early promotion of reusable chrome.
- [x] Backend SPEC has an executable reuse contract covering existing services,
      existing crates, external crates, shared-owner promotion, and validation.
- [x] Frontend/backend/guides indexes route future work to the updated reuse
      contracts.
- [x] The update does not weaken existing V2 isolation, security, dependency,
      or modular-monolith boundaries.
- [x] Maintained descriptive/project-guidance text does not present the
      personal development branch as a long-lived governance or architecture
      requirement; archived Trellis history is left untouched.
- [x] Release preflight is driven by the trusted `main` workflow plus explicit
      candidate SHA, keeps formal tag semantics unchanged, and does not expose
      Windows/Apple release secrets to preflight candidate execution.
- [x] Repository type, formatting, supported-platform, Release, and contract
      checks applicable to the final change pass.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
