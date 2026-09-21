# Production API protocols and vendor presets #40

## Goal

Connect shared model quick setup presets and explicit protocols through typed native save and credential-free readback.

## Requirements

- Current shared Quick Setup supports endpoint/key/model and a closed explicit protocol through native save and credential-free readback.
- Alibaba pay-as-you-go and Coding Plan, Tencent TokenHub Hy3, and Ark general API/Coding Plan have separate production presets with endpoint, key scope and usage copy.
- Existing requests retain protocol defaults. Unknown protocols and invalid target combinations fail before mutation; failed edits preserve existing configuration through the current transaction owner.
- Connection checks remain user initiated, use the selected protocol and do not send generic probes using tool-restricted Coding Plan credentials.
- Preserve #73 Chat/Responses migration fields. No credential-storage, auth, proxy, navigation or release changes.

## Acceptance Criteria

- [x] Production preset selection fills the shared form without network or writes.
- [x] Closed typed/native protocol contracts and exact public connection readback are covered by focused tests.
- [x] Native derivation tests added for defaults, explicit protocols, wrong target/protocol, secret-free summaries and plan/key mismatch.
- [x] Typecheck, lint and focused tests pass; heavy integration checks remain root owned.

## Notes

- Parent is 09-20-next-iteration-engineering in root's integration tree; root explicitly authorized this child and local commit.
- Temporary NEXT_ISSUES.json and VENDOR_SOURCES.md are inputs and must not be committed.

Native fixture execution, integrated runtime and final acceptance remain pending with root.
