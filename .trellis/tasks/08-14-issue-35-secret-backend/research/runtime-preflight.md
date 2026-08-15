# Runtime evidence preflight

Status: design-stage preflight only. No test, build, browser, server or native runtime command has been executed.

## Required environments

| Evidence | Required host | Path | Current readiness |
| --- | --- | --- | --- |
| Rust/frontend static + module/integration | current macOS worktree | canonical `mise run` tasks | repository versions identified; execute after freeze |
| renderer/browser | current macOS worktree | V2 renderer + repository browser task | page route exists; execute after source freeze |
| macOS native keyring | current macOS host | development desktop binary + Keychain | available in principle; entitlement/signing behavior to capture |
| Windows native keyring | native Windows x64 (`x86_64-pc-windows-msvc`) | dedicated user-visible child thread/worktree on matching host | not yet provisioned; mandatory before DONE; ARM64 is compile-only and cannot substitute |
| secret-surface failure paths | both native hosts | generated runtime canary + artifact scanner | scanner not yet implemented |

## Early blockers that are not deferred

- Windows native evidence cannot be substituted by cross-compilation, WSL bridging, mocks or copied artifacts.
- A matching Windows thread must be created after source freeze so it tests an immutable commit, not a moving worktree.
- If Windows x64 cannot produce `result=pass` for real Credential Manager CRUD, real missing, separate injected locked and denied, injected unavailable, verification-fail and old-delete-fail, plus real capture accept/cancel UAT and a separate real-OS cancel failure item, task status remains non-DONE; the “three distinct failure” count never relaxes this fixed set.
- OS prompts can affect unattended CI. Native acceptance scripts therefore separate interactive secure capture UAT from non-interactive CRUD using a runtime-generated canary inserted inside the native test process.

## Evidence naming

The sole JSON `evidence_class` enum is the 11-value definition in `research/native-evidence-plan.md` §9.1: `source_report | code_audit | ci_compile | unit_test | integration_test | native_contract | native_runtime | failure_path | uat | runtime_screenshot | artifact_scan`. Every manifest item has one class. Failure evidence separately records `evidence_origin=real_os|fault_injection`; missing is real OS, while locked/denied/unavailable/verification/old-delete use fault injection unless an additional reproducible real-OS fixture exists. Composite strings such as `native_runtime+failure_path` are forbidden.

Generated design imagery may carry the human/report-only label `visual_reference`; it is not a manifest `evidence_class` and never closes renderer/native evidence. Human completion of secure native capture uses JSON class `uat` (human label `UAT`).

No lower evidence class closes a higher gate.
