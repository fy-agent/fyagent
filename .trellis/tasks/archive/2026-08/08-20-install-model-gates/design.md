# Design

## Codex installer

Keep: HTTPS source, caps, exclusive create, no-follow, native PackageManager/hdiutil, post-install one operational result.

Remove as product feature: full-file SHA reread before consume (`DownloadedArtifact::revalidate`, `PreparedInstallPackage::revalidate_artifact`, `VerifiedFilePin` open/recheck `verify_reader`). PackageBridge copy may keep hash-while-copy (same I/O as copy) or size-only; do not add a second full-file pass.

`ChecksumMismatch` stays only if some remaining same-file mutation path still needs it; otherwise stop emitting it from installer admission.

## Claude v1

Helper in `quickSetup.ts`: pathname segments include `v1`. `ProviderPanel` when `app === "claude"` shows `FieldFeedback` tone warning. Change Claude placeholder away from `/v1` so placeholder does not teach the bad pattern.

## Connectivity

`StreamCheckService::check_url(base_url)` wrapping public `probe_reachability` + config timeout/retry.

IPC `stream_check_url { baseUrl: string }` — HTTP(S) only, no userinfo. Do not take arbitrary file paths.

V2: `providers.checkReachability(baseUrl)` plus WorkBuddy/OpenCode ports. Button label 「测试连通」 next to 拉取模型 / 保存. Show result via existing FieldFeedback/InlineNotice.
