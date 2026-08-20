# 精简 GitHub 发布流程并加速 CI

## Goal

正式发布不必先等一轮 main push CI 再整包重建；tag 在尚未发布 Release 时可以重打；缩短墙钟但不缓存巨型 Rust `target/`。

## Requirements

- 正式权威是 tag 指向的 commit SHA，而不是全程冻结的 live `main` HEAD。
- 不再要求同 SHA、`event=push`、`headBranch=main` 的成功 CI 作为发布资格。Release 自己的编译即证明。
- 允许 lightweight tag；允许在 GitHub Release 尚不存在时 force-update 同一 `vX.Y.Z`。已发布的 Release 仍拒绝原地覆盖。
- 从 `origin/main` `04bf9939` 移植「只公证 DMG 一次 + staple app」。
- CI/Release：`setup-rust-toolchain` 仍 `cache: false`；不得缓存 `src-tauri/target`。允许 lockfile 键控 `~/.cargo/registry`（及可选 `~/.cargo/git`）。Release native jobs 可开 pnpm cache。
- 不把 sccache / `RUSTC_WRAPPER` 写入仓库 Cargo config。

## Acceptance Criteria

- [x] eligibility 在 main 已向前移动、或没有 push CI 时仍可从匹配 Cargo 版本的 tag 发布
- [x] lightweight tag 可通过资格检查
- [x] 已存在 published Release 仍失败关闭
- [x] workflows 不出现 `src-tauri/target` cache；Rust action cache 仍为 false
- [x] 本地树具备 0.4.2 的单次 DMG 公证
