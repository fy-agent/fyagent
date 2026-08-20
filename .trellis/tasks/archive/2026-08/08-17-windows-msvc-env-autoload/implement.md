# Windows MSVC 编译环境自动接入 — 执行计划

## 实施清单（顺序执行）

1. 新增 `scripts/tasks/windows-msvc-env.mjs`
   - `VCTOOLS_COMPONENT` 常量、`msvcArchitecture`、`vswhereCandidates`、`findVsInstallation`、`resolveMsvcEnvironment`、`msvcRequirementHint`。
   - 纯函数、可注入 `spawn`，仅用 Node 内置。
   - `resolveMsvcEnvironment` 非 win32 返回 `null`；win32 走 vswhere 定位 → 组件校验 → `cmd.exe /d /s /c` + `windowsVerbatimArguments: true` → Node dump JSON → 解析返回 env。

2. 修改 `scripts/tasks/host-native.mjs`
   - import `resolveMsvcEnvironment as loadMsvcEnvironment`。
   - `executeTauriTask` / `executeCargoTask` 增加可选参数 `loadMsvcEnvironment`（默认真实实现）。
   - 按 design §3.2 的时序注入并合并 env。

3. 修改 `scripts/tasks/system-check.mjs`
   - import `VCTOOLS_COMPONENT` 与 `findVsInstallation`。
   - win32 用 vswhere 诊断项替换 `where.exe cl.exe`；`probe` 对 `vswhere.exe` 用绝对路径。

4. 更新测试
   - `tests/miseTaskContract.test.ts`：win32 的 `executeCargoTask`/`executeTauriTask` 用例注入 mock `loadMsvcEnvironment`；更新 sequence 断言（新增 `resolve:msvc`）。
   - `tests/systemCheck.test.ts`：`--describe-platform` 的 win32 断言适配 vswhere 诊断项。
   - 新增 `tests/windowsMsvcEnv.test.ts`：架构映射、vswhere 候选、`resolveMsvcEnvironment`（mock spawn：非 win32 返回 null、win32 成功解析、组件缺失报错、JSON 解析失败报错）。

5. 更新 spec
   - `backend/development-environment.md` §5：补充 Windows MSVC 环境加载边界。
   - `backend/task-runner-contract.md` §4：记录 cmd.exe 受控例外。

## 验证命令

```bash
mise run typecheck
mise run format:check
mise run test:unit
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
mise run check            # 聚合门（含 env:check、system:check、contracts）
```

针对性单测：

```bash
mise run test:unit -- tests/windowsMsvcEnv.test.ts
mise run test:unit -- tests/miseTaskContract.test.ts
mise run test:unit -- tests/systemCheck.test.ts
mise run test:unit -- tests/localBuildBoundary.test.ts
mise run test:unit -- tests/developmentEnvironment.test.ts
```

## 回滚点

- 每个文件改动独立可回退；`windows-msvc-env.mjs` 是新增文件，删除即回退。
- 若 Windows runner 上 cmd 引号/编码异常，回退 `host-native.mjs` 的 MSVC 注入、保留 `system-check.mjs` 的 vswhere 诊断（该部分无运行时风险）。

## 启动前检查

- [ ] 三个工件齐备（prd/design/implement）。
- [ ] `implement.jsonl` / `check.jsonl` 含真实 spec 条目。
- [ ] 无阻塞开放问题（需求已通过两轮确认，用户已授权端到端执行）。

## 风险提示

- 本机为 macOS，真实 Windows 环境加载/编译无法本地验证；纯逻辑用注入 mock 覆盖，真实证据由 Windows native Actions runner 闭环。
- 保留对 `rust:fmt`/`rust:fmt:check` 不加载 MSVC 的既有行为。
