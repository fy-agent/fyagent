# Implementation Plan — Fast Windows Demo Migration

## Phase 0 — Start Gate

1. 只使用当前 `demo/shurufa` 分支和本任务，不创建并行 Worktree/子任务。
2. 开始产品代码前重新确认 working tree，保护用户后续可能产生的未提交改动。
3. 读取本任务 `prd.md`、`design.md`、`research/planning-evidence.md` 和 jsonl context。
4. 读取当前 backend/frontend reuse、modular boundaries、V2 shell/type-safety 规范。
5. 本轮是 Windows-only Demo。实现中不要主动扩展 macOS、CI、release 或 installer scope。

## Phase 1 — Bring FY1111 Native Companion Into FyAgent

1. 在 FyAgent private Rust service 下建立 Companion owner；commands 保持 thin。
2. 从 FY1111 选择性原样迁移：
   - `input.rs`
   - `serial.rs`
   - `network.rs`
   - `device_settings.rs`
   - `profile.rs`
   - `target.rs`
   - `windows_foreground_restore.rs`
   - `runtime.rs`
3. 只做必要适配：crate path、FyAgent app-config path、module visibility、Tauri state/facade。
4. `Cargo.toml` 新增 pinned `serialport = "=4.8.1"`；复用已有依赖，补 `windows-sys` feature flags，不新增 serial plugin。
5. 保留 FY1111 模块内已有的纯 Rust unit tests，能低成本复制的 decoder/profile/runtime tests 一起迁入；不要先重写一套测试。
6. 用 focused Rust test/compile 验证迁移骨架，不碰前端。

**Checkpoint**：FY1111 native 功能在 FyAgent 内有单一 private owner，decoder/profile/runtime tests 可运行，没有把逻辑复制到 Tauri commands。

## Phase 2 — Native Serial Pump + ASR Exactly-once Admission

1. 把 serial read ownership 从 FY1111 的 frontend-driven poll 调整为 native-owned pump/tick。
2. commands `get snapshot` 只读 state；显式 start/stop/apply 才改变 runtime，不再通过 snapshot 命令竞争读取 COM。
3. 保留 `serialport` blocking read，使用 bounded timeout；不引入 tokio-serial。
4. 在 ASR decoder/status 中保留 `seq`，建立 ASR `SequenceTracker` 或等价 exactly-once admission。
5. `START` / `FAIL` 只更新状态；admitted `DONE + non-empty text` 产生一次 `AsrDone` internal event。
6. 确保 Agent trigger 在释放 Companion runtime/serial lock 后执行。
7. 覆盖 focused tests：duplicate/backward ASR、same text with new seq、invalid/overlong line、input gap、serial error stop。

**Checkpoint**：页面即使隐藏，native source 仍能推进；同一 ASR seq 最多触发一次 bridge。

## Phase 3 — Connect ASR to Existing Shurufa Agent

1. 在 `commands/shurufa.rs` / shurufacli integration 处做最窄 refactor：把“执行一个明确 text 的 turn”抽成复用函数。
2. 保持现有 debug path：`current_prompt()` -> same turn function。
3. Companion admitted `AsrDone.text` 直接调用 same turn function，`type_into_focus=true`。
4. 继续复用：Config、Store、recent summaries、`complete_turn`、Shurufa events、last output/error、`start_typer/enigo`。
5. 不把 ASR text 先写入 `prompt.txt` 再触发；`prompt.txt` 只属于 debug fallback。
6. 保持 current single-flight guard。busy 时返回/记录 stable error，不实现 queue。
7. 增加一个 focused bridge test，使用 fake/internal ASR event 证明：一次 DONE -> 一次 Agent admission；duplicate -> zero second admission。LLM/typing side 用可注入或已有 fake boundary，避免单测真实请求/真实键盘。

**Checkpoint**：正式链路已经是 serial ASR -> existing Agent core；没有第二个 LLM/history/typing implementation。

## Phase 4 — Tauri Commands and V2 Feature Port

1. 添加/整理 Companion thin commands，并逐项对照 FY1111 的 12 个 command 能力：`list_ports`、`capture_target_after_delay`、`load_profile`、`save_profile`、`start_dry_run`、`enable_live_for_run`、`poll_runtime_event`、`stop_runtime`、`load_device_settings`、`save_device_settings`、`apply_device_config`、`poll_network_status`。
2. `poll_runtime_event` / `poll_network_status` 的产品投影必须保留，但新命令只读 native snapshot；真正的 COM read 已在 Phase 2 由 pump 唯一拥有。
3. 命令命名避免与仓库其他 generic command 冲突；注册进唯一 `generate_handler!`。
4. 扩展现有 V2 shurufa feature port，或在该 feature boundary 下增加 Companion 子 port。
5. Rust DTO 使用 camelCase wire；TypeScript adapter 统一把 raw result 解析成 closed types。
6. browser adapter 继续对 native-only Companion 返回现有 `NATIVE_ONLY_ERROR` 语义，不伪造串口/Windows 行为。
7. 更新 focused feature-port test，冻结 command names 和 parser shape；不创建大 mock backend。

**Checkpoint**：React page 不直接 `invoke()`，所有 Companion wire 都从一个 typed feature owner 进入。

## Phase 5 — Recompose FY1111 UI Inside `/shurufa`

1. 保留 FyAgent feature header/shell，重排内容为紧凑 Companion 操作台。
2. 用 V2 primitives 重建：
   - serial select/refresh/runtime badge；
   - device settings Collapsible；
   - SecretInput for Wi-Fi password / SiliconFlow key / Agent key；
   - network/ping/record/asr status；
   - foreground target + 3-second capture；
   - fixed mapping rows + migrated ChordField behavior；
   - Save/DryRun/Live/Stop；
   - last event / notice；
   - latest raw ASR + optimized output；
   - Agent config；
   - manual debug textarea/preview as secondary fallback。
3. 不复制 FY1111 `app.css`；只在 `/shurufa/page.css` 添加必要布局，并复用 `--fy-*` tokens / control classes。
4. 明确区分“设备转写配置”和“输入法 Agent 配置”。
5. ASR/Agent status 从 native snapshot/event 投影；page 不负责串口读取或 Agent auto-trigger。
6. 保留页面在 browser preview 中的 native-only honesty；不要 seed fake hardware business state 到生产 adapter。

**Checkpoint**：功能布局与 FY1111 一一可找到，但视觉和组件属于 FyAgent；正常使用不需要手工 textarea。

## Phase 6 — Focused Validation Only

按风险做最小验证，不扩 CI。

优先运行：

```bash
# Rust: exact command may按实现后的 module/test name调整
mise run rust:fmt:check
cargo test --manifest-path src-tauri/Cargo.toml shurufa_companion

# Renderer / port / page
mise run typecheck:v2
mise run test:v2 -- tests/v2/platform/featurePorts.test.ts
mise run test:v2 -- tests/v2/pages/shurufa
```

若仓库当前没有独立 shurufa page test，就补一个 focused test，不为了 Demo 新建整套 suite。

Windows 手工验收必须记录：

1. 真实 COM 枚举并选择；
2. 下发一次 device config 并看到 network 状态；
3. `VKEY_INPUT/1` DryRun；
4. capture target + Live shortcut；
5. 硬件录音 -> `VKEY_REC/1`；
6. `VKEY_ASR/1 DONE` raw text 出现在 FyAgent state；
7. 同一 DONE 自动触发 Agent；
8. 优化 delta 流式进入事先选中的目标文本框；
9. duplicate ASR seq 不重复输入；
10. Stop/断线后 live permission 清除。

其中 Stop 不等于强制断开健康设备：如果 source 仍健康，Stopped 状态应继续允许 network/REC/ASR pump，以便输入法语音链路无需开启快捷键 Live 模式。

当前 Mac 开发环境不需要、也不能替代上述 Windows COM/Win32/HIL 证据。

## Phase 7 — Review / Spec Decision

1. 对照 PRD 的 FY1111 功能清单逐项勾选，不能因为 ASR 主链成功就遗漏 shortcut/device status 功能。
2. 检查没有两个 serial owner、两个 Agent client、两个 history DB、两个 typing engine。
3. 检查 secrets 没有被新的 debug log/status event 明文输出。
4. 检查 browser/non-Windows path fail closed，没有为了 Mac parity 扩 scope。
5. 如果实现形成稳定新 contract，再更新 owning backend/frontend spec；不要把 Demo 临时施工步骤写成长期项目规则。

## Rollback Points

- Phase 1 native import 可以独立回退，不影响当前 shurufa Agent。
- Phase 2 pump 失败时回退 pump 实现，但保留 serial decoder；不要交付 frontend-owned reader 作为最终方案。
- Phase 3 Agent bridge 可以通过关闭 auto-trigger 回退到原 debug shurufa，方便定位 serial vs Agent 问题。
- Companion persistence 独立在 `shurufacli/companion/`，删除该子目录不影响现有 Agent config/context DB。

## Explicit Non-goals During Execution

- 不做 macOS 功能；
- 不做 firmware 大改；
- 不加 Tauri serial plugin/Web Serial；
- 不改 CI/release；
- 不做 ASR queue；
- 不做 IME/clipboard redesign；
- 不把 demo credentials 迁到全新 secret architecture；
- 不为代码漂亮而重写已验证的 FY1111 Win32 实现。
