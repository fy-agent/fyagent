# Execution
- [x] Read parent authorization, current specs, native Provider and Models owners.
- [x] Implement strict portable projection, file policy and bounded native preview.
- [x] Implement atomic inactive-draft persistence and readback with stale protection.
- [x] Register thin commands, ACL and typed ports; compose Models dialog.
- [x] Add a typed model-form fill callback to import readback and reopened candidates.
- [x] Test adversarial input, rollback, drift, port and UI behavior.
- [x] Run canonical bootstrap, focused unit/type/lint/format; coordinated first Rust pass is 9/9.
- [x] Document evidence and residual native/cross-device limits in evidence.md.
- [ ] Root integration: wire onFillModelForm in Models (root owns current save-handler work); preserve Chat in the #40 shared form/request.
- [ ] Root integration: rerun focused Rust after final safety/fixture edits; run Clippy, renderer build and native picker checks after the release quiet window.

Implementation and local commit are authorized by the parent task. This worktree
does not claim integrated acceptance or issue closure; the parent explicitly owns
the pending heavy checks. See evidence.md for final-source vs earlier test scope.
