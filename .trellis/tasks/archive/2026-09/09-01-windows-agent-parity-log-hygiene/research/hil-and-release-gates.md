# Windows HIL and release gates

## 1. Why native HIL is mandatory

The current repository already has substantial Windows mocks and contract tests, yet the reported latest branch still fails to discover/install applications on real Windows. This is exactly the class of problem that cross-compilation and fixture tests cannot prove:

- interactive user vs elevated service profile;
- Registry 32/64 view and per-user hive behavior;
- UAC/vendor UI;
- actual signer leaf and version resources;
- bootstrapper/child installer timing;
- PackageManager current-user registration;
- Store/vendor auto-update interactions;
- default/custom directories and application launch.

No Windows feature is marked accepted without sanitized native evidence.

## 2. Test environments

### Required

1. **Windows 11 x64 clean VM — standard user**
   - non-admin primary user;
   - separate administrator available for UAC approval;
   - no preinstalled target applications except OS dependencies.
2. **Windows 11 x64 clean VM — administrator session**
   - verifies behavior when the interactive shell user is already administrator;
   - confirms code does not accidentally bind to a different account/profile.
3. **Windows 11 x64 dirty/upgrade VM**
   - historical Codex installation or other previous product versions;
   - duplicate/partial/uninstall-remnant scenarios.

### Conditional

4. **Windows 11 ARM64 native device/VM**
   - required for any native ARM64 claim;
   - x64-on-ARM64 compatibility recorded separately.

Cross-compiled Rust, emulation and static PE inspection are supporting evidence only.

## 3. Per-product HIL template

For each admitted Windows product, record:

### Artifact provenance

- source category and fixed first-party page/repository;
- release version/time observed;
- artifact filename/format/architecture/size/hash;
- signature validity and bounded signer/publisher evidence;
- package manifest/PE product and version identity.

### Pre-install inventory

- package/registry/App Paths/known-path evidence;
- interactive user SID category (redacted/hash, not raw SID in shared logs);
- readiness state/reason/actions;
- selected destination/scope.

### Execution

- job stages and progress;
- UAC/vendor interaction;
- cancellation at allowed stages;
- installer/package terminal result;
- bootstrapper/child behavior;
- elapsed time only as evidence, not production constant.

### Post-install inventory

- exact installed package/executable identity;
- version, architecture and scope;
- unique/ambiguous result;
- launch target capability;
- app launch only after explicit action.

### Update where policy permits

- older version preparation;
- remote version comparison;
- update owner behavior;
- in-place identity/scope retention or reviewed migration;
- rollback/recovery on failure;
- valid external vendor/Store update accepted on reread.

### Negative cases

- offline/redirect/source failure;
- wrong signer/publisher/product/architecture fixture;
- UAC cancel;
- vendor cancel/nonzero exit;
- post-install not observed;
- duplicate installs;
- stale Registry/App Paths;
- application running;
- low disk;
- reboot/app restart persistence.

## 4. Product gates

### QoderWork

- current user installer HIL;
- system installer HIL if exposed as a destination;
- default and custom directory discovery;
- install-only action proof.

### TRAE Work

- current official installer and signer HIL;
- vendor-choice/default directory;
- bootstrapper/child observation;
- install-only action proof.

### WorkBuddy

- existing signed EXE HIL;
- Store installed package inspection;
- explicit EXE-vs-Store strategy decision;
- no Store page launch reported as install success;
- install-only action proof.

### Grok Build

- install/update under standard user;
- official distribution-owner readback;
- duplicate npm/native owner ambiguity;
- confirmation that no desktop lifecycle was introduced.

### Claude Desktop

- current x64 MSIX manifest and install HIL;
- ARM64 only on native host;
- current-user/admin/UAC capability distinction;
- exact package family/application IDs;
- update-owner decision and update HIL if FyAgent exposes it;
- user-installer fallback only if MSIX gate fails and EXE is independently reviewed.

### OpenCode Desktop

- x64 desktop EXE HIL;
- ARM64 desktop asset and native HIL before claim;
- exact Authenticode signer/PE product;
- default/custom scope and installed executable;
- update and duplicate install HIL.

### Codex / ChatGPT Desktop

- clean current install identity;
- historical Codex -> new ChatGPT desktop update;
- old ChatGPT Classic coexistence where applicable;
- exact package/AUMID/process identity;
- launch/restart while running;
- ambiguous coexistence fail-closed behavior.

## 5. Codex log HIL

Use sanitized real or representative rollout files with a parent append lag:

1. run periodic sync long enough to cover multiple unchanged passes;
2. count WARN/INFO lines by bounded reason class;
3. append parent events until the child fork cutoff is available;
4. verify one recovery transition at most;
5. compare token totals, event counts and cursor state with a full rebuild;
6. repeat after app restart if pending state is persistent;
7. ensure routine logs do not include full `C:\Users\...` paths.

Acceptance target:

```text
unchanged expected lag across N passes:
  repeated WARN = 0
  repeated INFO = 0
parent catches up:
  usage recovered exactly once
  recovery event <= 1
```

## 6. Release gates

- **G1 — Policy**: only Grok Build has installable CLI lifecycle.
- **G2 — Reuse**: no new generic installer/downloader/helper/launcher/command IPC.
- **G3 — Identity**: every installed target is exact package/PE/publisher/signer evidence.
- **G4 — x64 HIL**: all claimed Windows products pass clean/installed/action flows.
- **G5 — Codex identity**: new ChatGPT, upgraded Codex and ChatGPT Classic identities are proven; migration is added only if the existing exact owner is demonstrably stale.
- **G6 — Log correctness**: spam removed without usage/cursor regression.
- **G7 — macOS regression**: working desktop install/update/launch chain remains intact.
- **G8 — Specs/CI**: owning contracts, focused/full tests and packaging checks pass.
- **G9 — ARM64 honesty**: native claim only with native artifact and native HIL.

If a product misses its identity/source/HIL gate, ship it as unavailable/manual fallback. Do not weaken a shared trust boundary to meet a feature checklist.
