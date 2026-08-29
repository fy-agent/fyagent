# Design — Windows Inventory and Installer Runner

## 1. Inventory architecture

```text
WindowsInstalledAppInventory
  +-- ShellUserUninstallAdapter
  +-- MachineUninstallAdapter (32/64 views)
  +-- ShellUserAppPathsAdapter
  +-- MachineAppPathsAdapter (32/64 views)
  +-- PackageManagerAdapter
  +-- ProductKnownPathAdapter
  `-- WindowsFileIdentityAdapter

evidence[] -> Stage 1 CandidateNormalizer -> inventory snapshot
```

Adapters return bounded evidence records; they do not construct public DTOs or choose a winner. The normalizer canonicalizes paths/package identity, opens the target as a regular file/package, reads product/version identity and merges records referring to the same installation.

## 2. Registry boundary

Extend the existing `windows_runtime::registry` fixed-location model. New locations are enum variants with compile-time components and explicit 32/64 view:

```text
ShellUserUninstall(view)
MachineUninstall(view)
ShellUserAppPaths(executable, view)
MachineAppPaths(executable, view)
```

`executable` is selected from the closed product descriptor, not IPC or registry data. Every component is opened with link-safe semantics. Values are read-only, bounded and normalized; UninstallString is not executed.

Machine inventory may be read from HKLM; per-user inventory uses the frozen shell SID under HKEY_USERS. Access denied or missing user hive produces partial/unknown evidence, not a false negative.

## 3. Identity and dedup

For unpackaged apps, a trusted candidate requires:

- canonical regular `.exe` under a candidate root or registered absolute location;
- closed expected product identity from Win32 version resources;
- compatible architecture where observable;
- signer/product policy when required for execution;
- a stable file identity/canonical path capability held backend-side.

Registry DisplayVersion is evidence; executable version is the runtime candidate version. Conflicts are retained. UninstallString is metadata only and never a launch target.

Packaged apps reuse PackageManager package identity/version/AUMID rules from Codex Desktop where applicable.

## 4. Product install descriptor

Extend the source/product policy with:

```text
installerScope: current_user | all_users | vendor_choice | unknown
interactionMode: vendor_ui | silent_reviewed | package_manager
expectedSignerPolicy
expectedInstalledIdentity
supportedArchitectures
postInstallVersionPolicy
```

Examples at the current baseline:

- Qoder resolved EXE: current-user x64 vendor UI.
- WorkBuddy EXE: vendor UI chooses destination.
- TRAE Work EXE: use only the scope/architecture proven by its official source and HIL.
- Codex MSIX: remains on the existing PackageManager/helper adapter.

The descriptor contains no renderer-provided switches.

## 5. Download and admission

Use the existing streaming downloader and private canonical job directory. The returned prepared package retains file capability and release binding.

Admission sequence:

1. revalidate release/source descriptor;
2. revalidate retained artifact identity;
3. inspect PE format/architecture with bounded platform APIs;
4. verify Authenticode chain and expected signer policy with WinVerifyTrust;
5. freeze interactive-user context;
6. create a protected bridge to the fixed user helper/runner;
7. admit the helper only after image, SID, nonce and bridge checks.

Never reopen an arbitrary renderer path after validation.

## 6. Closed installer runner

Preferred design is an extension of the fixed FyAgent user-helper protocol:

```text
RunVerifiedInstaller {
  job capability,
  product enum,
  package format enum,
  interaction mode enum
}
```

The bridge conveys a protected file capability/identity, not a free path. The helper performs one closed launch operation as the interactive user. For EXE vendor UI it uses a reviewed ShellExecuteEx path with `SEE_MASK_NOCLOSEPROCESS` so process termination can be observed. UAC is OS-owned.

If the vendor bootstrapper exits after spawning another process, the runner records that process result only as a hint and relies on bounded inventory polling. It must not follow arbitrary child process trees by display name.

## 7. Job states

Install lifecycle may require stages beyond the current generic list:

```text
checking
downloading
verifying_package
launching_installer
awaiting_user
verifying_installation
succeeded
failed
cancelled
incomplete
```

`awaiting_user` means the vendor installer owns interaction. Once an external installer may have side effects, FyAgent does not offer a misleading “cancel installation” unless the adapter has a real cancellation handle. It may offer “stop waiting” with distinct semantics.

## 8. Post-install comparison

Capture a pre-inventory and fresh post-inventory. Compare by trusted identity:

- fresh install: expected new candidate appears;
- update: selected candidate remains or an explicitly reviewed identity migration occurs;
- vendor-choice destination: result may be any one newly trusted candidate, but the UI must disclose that the vendor owns location choice;
- duplicate/ambiguous result: incomplete, user must choose/clean up;
- no trusted result: failed/incomplete even if process exit was zero.

Rollback is only claimed for adapters that actually own the transaction. Vendor UI execution is assisted and cannot borrow macOS rollback language.

## 9. Frontend

Use Stage 1 target/destination picker. Add one shared typed status renderer for external-installer stages if the existing lifecycle component cannot express them. Product rows provide copy and descriptor facts, not custom control trees.

## 10. Security invariants

- no generic command/args/verb/cwd IPC;
- no execution of UninstallString/App Paths registry data;
- no full-disk scan;
- no fallback launcher after interactive-user/helper failure;
- no success from launch or exit code alone;
- no secret/path/certificate dump in renderer errors or task evidence.
