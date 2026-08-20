# Validation and evidence guide

Validation must match the claim. Start with the tests nearest the changed code,
configuration, script, or workflow, then add the smallest higher-level gate
that crosses the changed boundary.

## Local evidence

- `mise run tasks:docs:check` checks the generated task reference.
- `mise run tasks:validate` checks task metadata and task-runner contracts.
- `mise run check:contracts` covers repository tasks, maintained docs, Python
  locks, versioning, and release policy.
- `mise run check` is the complete current-host local gate.

Retained `.trellis/` tasks and specs are optional AI assistance. The local
gate neither requires nor validates a project-specific Trellis wrapper,
overlay, task, PRD, or spec before it can pass.

Targeted unit and Rust tests are useful while iterating, but a passing narrow
fixture is not evidence for an unrelated layer.

## Native and remote evidence

Local structure or policy tests cannot prove:

- a Windows x64 or ARM64 setup executable was built and packaged successfully
  on the matching architecture;
- an install/uninstall lifecycle result, when an operator elects to run that
  manual Windows diagnostic;
- an Authenticode state or certificate/timestamp policy;
- a macOS bundle's native identity and packaging;
- another supported architecture's native packages;
- a GitHub required check, attestation, or published Release.

Those claims require the matching native CI/release job and exact remote
source identity, except that an optional manual lifecycle claim requires the
recorded result from the matching Windows machine where it was run. The Release
workflow does not execute that lifecycle diagnostic. A public release claim
additionally requires the release workflow's re-download/digest checks,
metadata, attestation, public state, and Latest verification.

## Windows Codex PackageBridge A1 evidence boundary

This delivery intentionally does not run HIL, either on a local Windows machine
or through GitHub Actions. Its present evidence is limited to static contract
tests, scoped Windows-target compilation checks, and code/security review.
Those checks exercise source and protocol invariants but do not prove the
protected local-file deployment boundary at runtime.

The following behaviors therefore remain explicit, unverified residual risks:

- the fixed
  `FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}\v1`
  root ACL gives Authenticated Users only stable `FILE_GENERIC_EXECUTE`
  directory semantics (traverse/read-attributes/`READ_CONTROL`/synchronize,
  never list/create/write/delete/delete-child), while each operation ACL names
  the exact Alice SID;
- the actual ProgramData volume passes capacity admission for the accepted extra
  full copy, and the sealed bridge SHA/size/object identity matches the verified
  source on real Windows 10/11 and x64/ARM64 systems;
- `Hello` → parent authentication → bridge control → `Started` identity →
  parent admission occurs before current-user `AddPackageByUriAsync`;
- in a real Bob-elevated/Alice-standard-Explorer-Shell case, Alice reads the
  protected DOS file URI, PackageManager consumes/registers the expected package
  and supplies the native signature-chain decision, while effective ACLs reject
  create/write/rename/replace/delete/hard-link/reparse/ACL-owner mutations;
- an authenticated non-`Started` WinRT terminal status, matching valid terminal
  frame, and clean pipe close permit only application-owned normal cleanup;
  ambiguous termination leaves an immutable orphan, and the next elevated
  bridge creation cleans only known conforming objects through held handles;
- NSIS does not touch PackageBridge and there is no HTTP, proxy, network, Temp,
  cwd, or install-root package fallback.

These checks must not be reported as proof of native compatibility or native
runtime verification. A2 is not a runtime fallback and is never selected by
HRESULT, ACL, disk, timeout, or missing validation. Only future independent
native validation plus an explicit, separately authorized design decision may
start a separate A2 implementation and review. The product minimum supported
Windows version remains unchanged; existing OS/package `MinVersion` preflight
owns unsupported-host rejection before helper launch.

## Semantic scans

Historical cleanup scans need interpretation:

- retain real wire/schema/API/toolchain versions;
- retain negative tests that forbid retired assets or behavior;
- leave historical release notes and archived task evidence unchanged;
- reject operational references to removed versioned development packages or
  any external planning directory.
