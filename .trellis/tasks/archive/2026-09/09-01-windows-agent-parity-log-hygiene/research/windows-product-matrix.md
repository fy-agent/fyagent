# Windows product implementation matrix

This matrix separates facts already present in the repository from facts that must be frozen by current artifact inspection/HIL. `TBD by HIL` is intentional and must not be replaced by a guess.

## 1. Target matrix

| Product | Surface | Planned Windows strategy | Existing code | Blocking evidence before production |
| --- | --- | --- | --- | --- |
| QoderWork | desktop | signed interactive EXE | source, names/paths, signer policy, runner exist | current artifact signer/product, user/system scope, default/custom path, native HIL |
| TRAE Work | desktop | signed interactive EXE | source, names/paths, signer policy, runner exist | current artifact signer/product, scope/path, bootstrapper behavior, native HIL |
| WorkBuddy | desktop | reviewed signed EXE; packaged Store only if superior/proven | EXE source, names/paths, signer policy, runner exist | compare EXE vs Store exact identity/update/scope; native HIL |
| Grok Build | CLI | existing Grok Tooling rules through ordinary-user helper in formal builds | mature CLI lifecycle exists but formal elevated build is disabled | owner-preserving observe/install/update HIL; do not conflate Grok Bot |
| Codex | desktop | dedicated PackageManager/MSIX owner; exact identity migration only if proven | mature dedicated path exists | new ChatGPT clean install, Codex upgrade and ChatGPT Classic coexistence package/AUMID HIL |
| Claude Desktop | desktop | prefer official MSIX through existing packaged-app capability; EXE fallback only if reviewed | macOS source exists; Windows descriptor/source absent | current MSIX identity/publisher/AUMID/architecture/update owner; native HIL |
| OpenCode Desktop | desktop | official signed desktop EXE through existing runner | macOS source exists; Windows descriptor/source absent; upstream workflow builds x64/ARM64 | current release asset per architecture, PE product/signer/scope/path/updater; native HIL |

## 2. Product-specific invariants

### QoderWork

- policy: install + launch, no FyAgent update;
- user/system installer scopes must not be silently substituted;
- known path is a hint only;
- exact signer/product admission remains required;
- custom location is accepted only through trustworthy Registry/App Paths evidence plus file validation.

### TRAE Work

- policy: install + launch, no FyAgent update;
- exact `data.solo`/official source selection from existing source adapter remains closed;
- installer child/bootstrapper behavior must be observed rather than killed or assumed;
- post-install fresh inventory owns success.

### WorkBuddy

- policy: install + launch, no FyAgent update;
- existing vendor update metadata may remain a fresh-install release source but must not create an update action;
- Store product ID is source evidence, not package identity;
- Store and EXE installations must either normalize to one trusted product identity or remain explicitly distinct/ambiguous.

### Grok Build

- policy: CLI install + update only;
- existing official/native/npm distribution-owner logic remains authoritative;
- other independent Settings/Tooling products remain out of scope unless a direct conflict is reproduced;
- no desktop product substitution.

### Codex / ChatGPT

- policy: desktop install + update + launch;
- package identity and runtime identity must be exact;
- historical/current identities may coexist only through a closed migration policy proven by HIL;
- `ChatGPT Classic` cannot be mistaken for the new app;
- restart/launch must remain capability-bound and package-aware.

### Claude Desktop

- policy: desktop install + update + launch;
- physical component label is Claude Desktop, stable product ID remains `claude-code`;
- packaged-app strategy preferred only after exact manifest and current-user behavior are proven;
- one update owner must be documented;
- admin/UAC-dependent Cowork capabilities must not be misrepresented as installation failure if the base app is valid.

### OpenCode Desktop

- policy: desktop install + update + launch;
- fixed first-party repository/release only;
- select exact desktop asset by architecture;
- Authenticode, PE product and installed target must all match;
- no CLI installer or Scoop dependency fallback;
- upstream self-updater is not the FyAgent job owner.

## 3. Architecture claims

| Claim | Gate |
| --- | --- |
| Windows x64 supported | current x64 artifact inspection + Windows 11 x64 native HIL |
| Windows ARM64 native | current native ARM64 artifact + package/PE identity + Windows ARM64 native HIL |
| x64 on ARM64 compatible | vendor support evidence + ARM64 host HIL using x64 artifact; label as compatibility, not native |
| x86 supported | explicit first-party artifact and HIL; otherwise unsupported |
| all-users/system install | product installer/package supports it + helper/UAC/scope HIL |

## 4. Source-to-success invariant

For every product:

```text
fixed first-party metadata/artifact kind
  -> frozen opaque release
  -> shared download/package deployment
  -> local artifact/package identity admission
  -> execution in frozen interactive-user context
  -> fresh inventory
  -> unique trusted target + actual version
  -> success
```

Any break before the final readback is not success.
