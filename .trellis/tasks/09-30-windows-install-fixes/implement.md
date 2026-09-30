# Implement

1. 代码与单测（本分支 `maintenance/windows-install-fixes-20260930`）。
   - Rust：helper.rs、interactive_user.rs、process_launch.rs、tooling.rs、grok.rs、claude.rs、cli.rs、mod.rs、
     package_bridge.rs、fetch.rs、opencode.rs、sources/mod.rs、desktop.rs。
   - TS：installerErrorCopy.ts 与 tests/shared/codexInstallConfirmation.test.ts。
2. 本地验证：`cargo fmt --all --check`；`pnpm typecheck`、`pnpm format:check`、`pnpm lint`、
   `pnpm test:unit --project contracts --project renderer`。主 crate 在 Linux 主机不能编译（已知），Rust 测试在 CI 跑。
3. 在 fork 上 dispatch `ci.yml`（Windows x64/ARM、macOS、frontend）。
4. fork 的 `test/installer-smoke` 分支（不进上游）：
   - 新增构建 workflow，从本分支构建未签名的 Windows x64/ARM64 NSIS 和 macOS app。
   - installer-smoke 新增 `artifact_run_id` 输入，安装 fork 构建并在 summary 标注「非 release 构建」。
5. 虚拟机复测并与 run4/run7 对比；结果写入 `vm-smoke/`。
6. 开一个 PR（中英文说明，`Refs #29 (missed coverage)`），批准 fork PR 的 action_required run。
7. 按用户 19:29 授权：CI 绿且复测通过后合并；另开版本 PR（0.4.10 + zh/en release notes），合并、打 tag、发布、校验资产、
   对已发布资产跑 installer-smoke。任何失败则停止并报告。
