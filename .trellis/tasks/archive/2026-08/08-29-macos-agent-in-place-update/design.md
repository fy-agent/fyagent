# Design — macOS Bundle Replacement

## 1. Intent model

Deployment receives a backend-authorized intent, not a destination path:

```text
FreshInstall {
  destination capability,
  expected destination revision,
  release descriptor
}

UpdateExisting {
  trusted candidate capability,
  expected candidate revision,
  release descriptor
}
```

The candidate/destination capability comes from Stage 1 and remains non-serializable. The macOS adapter resolves the exact target parent/path only after the current inventory is revalidated.

## 2. Shared deployment owner

Preferred architecture:

```text
ManagedPackageCoordinator
  -> MacDmgSourcePreparation
  -> MacBundleReplacementTransaction
  -> ProductBundlePolicy
  -> InstallationInventory verifier
```

`MacBundleReplacementTransaction` is the single owner for mount discovery, staging, backup, commit, rollback and cleanup. `ProductBundlePolicy` supplies only closed bundle ID and version projection/equivalence.

Do not make a generic filesystem replacement API callable from IPC. The transaction receives already-verified package and target capabilities.

## 3. Transaction sequence

### Preflight

1. Force-refresh source and target inventory.
2. Validate release ID and target revision.
3. Resolve target parent/path from the backend capability.
4. Verify parent, volume, file kinds and generated-name absence.
5. Check target runtime state and authorization capability.
6. Compute free-space requirements for download, staging and backup volumes.

### Package preparation

1. Download through the existing bounded downloader/job ownership.
2. Mount read-only with bounded command output.
3. Discover exactly one regular `.app`; reject symlink and multiple candidates.
4. Read and verify product bundle identity and product-specific version shape.
5. Copy to a generated staging path in the selected target parent/same volume.
6. Verify staged bundle again from the staged object.

### Commit

1. Revalidate target candidate immediately before mutation.
2. Mark the job non-cancellable.
3. If updating, rename current target to a generated backup.
4. Rename staged bundle to the exact selected target path.
5. Verify installed identity/version/path.
6. Reread inventory and ensure the selected scope gained the expected candidate without an undeclared duplicate.
7. Remove backup only after verification and inventory readback succeed.

### Rollback

- On failed replacement verification, remove only the exact staged replacement whose identity still matches the transaction record, then restore the backup.
- Refuse cleanup if another actor replaced the path after commit; preserve backup and return recovery-required.
- Fresh-install failure removes only the transaction-owned target after rechecking identity.
- Detach mount and clean generated paths on every terminal path.

## 4. Authorization policy

Fresh system install and system update have different semantics:

- fresh install may offer explicit user/system choices;
- update must preserve the selected existing scope.

An OS-native authorization adapter may be added only after reviewing supported Apple APIs and the existing Tauri/runtime model. It accepts a closed replace operation and validated capabilities, never a shell command or path from renderer.

Without a proven adapter:

- mark system destination as `authorization_required`;
- keep old application untouched;
- offer the official/manual installation route or an explicit later authorized retry;
- never reuse Codex fresh-install permission fallback for an update.

## 5. Runtime coordination

Use exact bundle identifier/runtime evidence. The transaction may:

- refuse while running and ask the user to quit;
- use an existing trusted close/restart coordinator if generalized safely;
- never force terminate by name.

The runtime instance and disk candidate are separate. Post-install verification reads the disk target first; launch verification, when requested, happens afterward.

## 6. Product policies

```text
QoderWork:  bundle id com.qoder.work.cn
TRAE Work:  bundle id cn.trae.solo.app; version from product.json tronBuildVersion
WorkBuddy:  bundle id com.workbuddy.workbuddy; dotted-prefix marketing equivalence
```

The source app basename is not identity. Update preserves the selected target path; fresh install can use the reviewed package basename under the chosen destination.

## 7. Frontend flow

```text
refresh inventory -> choose target -> preflight disclosure
-> download/stage -> commit (cancel removed) -> verifying
-> authoritative reread -> success / restored failure / recovery required
```

The UI uses Stage 1 target labels and existing lifecycle controller where its stages fit. If rollback/recovery states cannot be expressed, evolve the closed job stage/reason contract instead of hiding them in free-form error text.

## 8. Reuse constraints

- Keep one DMG mount/parser/transaction implementation.
- Preserve Codex Desktop tests as golden behavior.
- Do not copy product-specific bundle/version readers into the transaction; call product policy.
- Do not move source download logic into the platform deployer.
- Do not create a cross-platform “copy application” helper that bypasses platform identity and rollback rules.
