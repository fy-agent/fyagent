# 实施清单：Grok 大陆 npm 一键安装与 OpenCode Windows x64

## 执行原则

- 用户已要求写入任务并推进到归档；规划完成后运行 `task.py start`。
- 两条后端工作流并行：Grok npm 计划 / OpenCode Windows source+handoff。不要抢对方文件表。
- 共享 UI、i18n、SPEC、用户手册等两流都结束后再改。
- 不以隐藏 UI 代替 backend policy；不以 installer exit 代替 Windows handoff；不以本机下载成功代替大陆 HIL 承诺。
- 禁止 git commit。归档前由主会话做 Trellis spec 更新与提交。

## Phase 0：清单与证据

1. [x] 向 `registry.npmjs.org` 复核 `@xai-official/grok` 当期 stable 的主包与 macOS/Windows 平台包 SHA-512；写入单一 manifest 文件。当前 1.0.13。
2. [x] 确认当前分支工作树与本任务 scope。
3. [x] OpenCode：若执行环境有 Windows，刷新当期 stable EXE 身份；否则 Windows identity 保持 TBD/fail-closed。

## Workstream A — Grok npm（可并行）

只改 design.md §9 中 Grok 列的文件。

1. [x] 增加 `GrokNpmInstallPlan` 与编译进应用的 manifest；解析失败 fail-closed。
2. [x] 计划生成：精确版本、闭集 registry 顺序、integrity、npm 12 allow-scripts。
3. [x] 删除所有可达 `@xai-official/grok@latest`。
4. [x] 默认 `install`（无 expected native owner）规划 OfficialNpm。
5. [x] macOS `execute_official_npm` 使用计划与 `--registry` / `GROK_NPM_REGISTRY`；不再问 npmjs latest。
6. [x] Windows helper 接收计划；无计划拒绝；不再使用 `GROK_NPM_INSTALL_SPEC=@latest`。
7. [x] native update 去掉 installer fallback；npm update 必须有 package-manager anchor。
8. [x] 源失败换下一个；不降级；失败保留旧安装。
9. [x] 聚焦测试：计划、哈希换源、allow-scripts、helper、planner、无 `@latest` 扫描。Mac 已通过；Windows 实机 HIL 未做。

建议命令：

```bash
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml -p fyagent-user-helper --offline
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml --offline tooling grok
```

## Workstream B — OpenCode Windows（可并行）

只改 design.md §9 中 OpenCode 列的文件。

1. [x] Windows x64 固定 `windows-x64-nsis` → `PackageFormat::Exe`。
2. [x] GitHub latest 改为非阻断；GitHub 失败时 stable source 仍成功。
3. [x] 分平台校验 token/arch/format。
4. [x] 追加 `AgentInstallerProduct::OpenCode` 与 wire **14**（不占用 Grok 5–13）。
5. [x] 本机无 Windows WinVerifyTrust HIL：ProductName / relative EXE / signer 保持空；`windows_exe_install_admitted` 拒绝安装与下载。source 解析测试已覆盖。
6. [x] helper 已把 OpenCode 接到现有 `ShellExecute(open)`；宿主 download 在身份闭合前 fail-closed。
7. [x] source / helper 测试需在 Grok 工作流完成后整库复跑（并行改动曾打断 `agent_install` 复跑）。

建议命令：

```bash
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml --offline agent_install
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml -p fyagent-user-helper --offline
```

## Phase 共享 — 前端、文档、SPEC

两工作流代码完成后再做：

1. [x] Settings/Agent：Grok 主按钮 = 一键安装；显式原生/换归属不自动。
2. [x] 删除前端自己拼的 `@latest` 命令展示。
3. [x] OpenCode Windows handoff 文案；ARM64 不可用。文案不宣称 Windows 已支持。
4. [x] 四语言 i18n parity。
5. [x] 用户手册：大陆可通过官方 npm + 国内镜像安装 CLI；不保证登录/推理。
6. [x] 更新 `.trellis/spec/backend/external-agent-lifecycle.md`、`windows-runtime-security.md`、`frontend/user-facing-copy.md` 及受影响 frontend SPEC。

```bash
mise exec -- pnpm exec vitest run \
  tests/components/AboutSection.test.tsx \
  tests/v2/pages/agents/AgentInstallReadinessSection.test.tsx \
  tests/v2/platform/grokToolingPort.test.ts \
  tests/codexUserHelperContract.test.ts \
  tests/desktopSecurityBoundary.test.ts
```

## 仓库门禁

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck:v2
mise run test:v2
```

归档前按 backend/frontend index 跑匹配的 `mise run check` / prearchive（exact `--exclude-active-task`）。

## Native HIL

1. [ ] macOS：默认一键安装装到清单版本；`grok --version` 匹配；不改全局 npmrc。
2. [ ] Windows 11 x64：helper npm 计划安装；无 `@latest`；显式 native 仍可触发但不自动。
3. [ ] 阻断 `x.ai` / GCS 时 npm 路径仍可安装，或记录未验证。
4. [ ] OpenCode Windows 正式包：download、verify、handoff、完成后 inventory；无 HIL 则不宣称支持。
5. [ ] GitHub API blocked 时 OpenCode source 仍可解析。

## 回滚顺序

1. OpenCode Windows：关 Windows source/action。
2. helper OpenCode：撤回新 product code。
3. Grok 镜像：保留精确版本，不要回退 `@latest`。
4. Grok update 无法锚定：关该 owner update。

## Archive prerequisites

1. [ ] 自动化 gate 通过。
2. [x] Grok 默认安装无 `@latest`，Mac/Windows 计划共用。
3. [x] SPEC 已按 Trellis 3.3 更新。
4. [x] 未完成 HIL 已记录为残余风险（见上 Native HIL）。
