# Apple replacement and authorization evidence

## File replacement

Apple FileManager replacement documentation:

https://developer.apple.com/documentation/foundation/filemanager/replaceitem(at:withitemat:backupitemname:options:resultingitemurl:)

Apple describes replacing an item with backup/result semantics intended to avoid data loss. FyAgent does not have to call this exact Foundation API from Rust, but its transaction should preserve the same invariant: stage first, retain a recoverable old item, replace atomically on the target volume where possible, and verify before deleting the backup.

## Authorization

Apple NSWorkspace authorization type for file replacement:

https://developer.apple.com/documentation/appkit/nsworkspace/authorizationtype/replacefile

This is evidence that macOS has an explicit authorization concept for replacement. Implementation must still verify whether it is usable from FyAgent's current Tauri/Rust runtime and distribution model. Until that proof exists, a permission error must remain an explicit non-destructive state.

## Decision

- Do not implement `sudo`, arbitrary AppleScript or renderer-supplied privileged paths.
- Do not automatically change destination after permission failure.
- Prefer the existing tested rename/backup transaction plus a separately reviewed OS authorization adapter.
- Record manual/HIL evidence separately from portable unit tests.
