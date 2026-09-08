# Windows runtime 相关合同摘记

来源：`.trellis/spec/backend/windows-runtime-security.md`。

用途：该 SPEC 超过 Trellis 单文件上下文注入上限；本摘记只聚焦本任务会修改的场景。执行与检查阶段仍必须打开原 SPEC 的对应章节，并在归档前更新原 SPEC。

## 1. Formal elevated Windows 边界

- 正式 Windows 发行默认 elevated parent；不得从该进程枚举、信任或执行普通用户可控的 PATH、CLI shim、下载目录和 profile binary。
- 与普通用户交互的受控操作必须绑定 frozen Explorer user context，通过现有 user-helper 与认证 pipe 完成。
- helper 失败、Explorer context 失效或身份不完整时必须 fail-closed，不得回退 elevated parent 执行。
- 不得新增通用 `run --cmd`、任意 executable/path/URL/argv helper。

## 2. 当前 helper 产品合同

当前闭集包含：

```text
codex-msix-install
agent-exe-install --product qoderwork|trae-work|workbuddy
grok-tool --action observe|install|update [--owner native|npm]
```

本任务目标：

```text
agent-exe-install --product qoderwork|trae-work|workbuddy|opencode
grok-tool --action observe|install|update [--owner native|npm]
```

- OpenCode 只能扩展现有 `agent-exe-install` product enum；不得新增 OpenCode CLI helper verb。
- Grok install wire family 保留；默认 install 改为执行宿主下发的 `GrokNpmInstallPlan`，不得自己跑 `@latest`。
- 不把 Grok wire 值给 OpenCode 复用。
- OpenCode product code 追加到未使用值，保持 existing product/wire compatibility。

## 3. Agent EXE handoff

- Parent只把受保护 PackageBridge artifact和闭集 product action交给helper；renderer不提供URL/path/command/signer。
- Helper重新验证bridge/pin/pipe/admission后，固定调用 `ShellExecuteExW`，verb为 `open`，无自由参数和silent switch。
- `launching_installer` 是不可取消副作用边界；vendor UI/UAC接管后不得wait、kill或从exit code推断安装。
- 成功 `ShellExecute` 表示vendor-wizard handoff，Agent job可以succeeded，但inventory在新扫描前可继续not-installed。
- retained EXE leaf按既有handoff合同处理；不要在成功交接时过早删除导致安装器失效。

## 4. Windows artifact 与 installed target admission

- 下载产物和installed target都不能只凭路径/文件名/注册表显示名信任。
- 必须组合稳定无reparse文件身份、支持的PE架构、closed ProductName、`WinVerifyTrust`、exactly-one signer与reviewed signer leaf。
- x64产品允许已审查的i386 installer stub特例，但installed application target仍必须是x64。
- Registry/App Paths是候选证据，不是命令；parent key枚举权限、WOW64 view和link处理按原SPEC执行。
- incomplete/access/bound/Shell-context failure必须投影unknown，不得误报not-installed。
- 多个可信candidate保持multiple并要求opaque target selection，不选第一个。

## 5. Grok formal Windows 目标

- observe/install/update 仍走 closed ordinary-user helper；失败无 elevated fallback。
- 默认 install 执行宿主下发的 `GrokNpmInstallPlan`；无计划不得跑 `@latest` 或 PowerShell。
- 显式 native install 仍合法，但不得作为 npm 失败 fallback。
- update 必须保持 observed owner；native 无 installer fallback；npm 必须有 package-manager anchor。
- 静默 owner 迁移禁止。ambiguous owner 不允许 mutation。

## 6. 本任务需要回写原 SPEC 的事实

1. Grok 默认一键安装是官方 npm + 大陆镜像 + 应用内精确版本清单，不是原生 `x.ai`。
2. helper usage 保留 grok-tool install，追加 OpenCode closed EXE product。
3. OpenCode Desktop Windows x64 复用 Agent EXE handoff。
4. handoff success 仍不是 installed proof。
5. 不把“能装上 CLI”写成“中国大陆 Grok 登录/推理可用”；Windows ARM64 在 HIL 前不声明。

