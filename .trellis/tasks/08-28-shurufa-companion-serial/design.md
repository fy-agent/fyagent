# Technical Design — FY1111 Companion 全量迁移 + Shurufa Agent Bridge

## 1. Design Intent

本任务不是把 FY1111 的 `companion/` 目录复制到 FyAgent 里继续作为第二个应用运行，而是把它拆成两个可复用层：

```text
FY1111 native Companion capability
  -> FyAgent private Rust service/facade
  -> thin Tauri command + typed V2 port
  -> /shurufa page composition

VKEY_ASR/1 DONE
  -> same native service
  -> existing shurufa Agent core
  -> existing enigo typer
```

保留 FY1111 的已验证领域代码和 Win32 行为，替换独立 app shell、前端组件和“由 React poll 才读串口”的生命周期绑定。

## 2. Source-to-Target Reuse Map

| FY1111 source | FyAgent treatment | Notes |
| --- | --- | --- |
| `companion/src-tauri/src/input.rs` | 直接迁移到 private Companion service 子模块 | 保留 `InputId` / `Chord` / allowlist / canonicalization |
| `serial.rs` | 直接迁移 + ASR sequence admission 适配 | 保留 bounded decoder / `serialport`; serial reader 改为 native-owned |
| `network.rs` | 直接迁移 | 保留 config record / status / 5G heuristic |
| `device_settings.rs` | 直接迁移，storage path 改为 FyAgent shurufa data dir | 不重写 validation |
| `profile.rs` | 直接迁移，storage path 改为 FyAgent shurufa data dir | 保留 revision + backup + stale rejection |
| `target.rs` | Windows 逻辑直接迁移 | 本轮不做 macOS 等价实现 |
| `windows_foreground_restore.rs` | Windows 逻辑直接迁移 | 保留 exact executable path / bounded attach / post-focus verify |
| `runtime.rs` | 迁移 state machine + 把 polling owner 移到 native pump | 保留 DryRun/Live fail-closed semantics |
| `lib.rs::commands` | 不直接复制成大 command module | 映射为 FyAgent service methods + thin Tauri commands |
| `companion/src/types.ts` | 映射为 V2 feature wire/domain types | raw `unknown` 在 Tauri adapter 统一解析 |
| `host.ts` | 映射为现有 V2 FeaturePort | 不建立第二套 `CompanionHost` runtime architecture |
| `validation.ts` | 尽量迁移纯函数到 `/shurufa` feature/helper | frontend validation 与 Rust authority 保持一致，不取代 native validation |
| `ChordField.tsx` | 最小适配为 FyAgent page-local control | 使用 FyAgent Button/control styles |
| `SettingsPanel.tsx` | 只迁移信息架构/交互 | 用 V2 Collapsible/SecretInput/Badge 等重组 |
| `App.tsx` / `app.css` | 不直接复制 | 迁移 layout、状态和动作，不迁移独立 app chrome/CSS system |

## 3. Backend Ownership

### 3.1 Private service

推荐新增一个私有 owner，例如：

```text
src-tauri/src/services/shurufa_companion/
  mod.rs
  input.rs
  serial.rs
  network.rs
  device_settings.rs
  profile.rs
  target.rs
  windows_foreground_restore.rs
  runtime.rs
```

具体目录名可以在实现时按最新代码收敛，但必须满足：

- implementation modules private；
- 一个 crate-scoped service/facade 是其他 backend caller 的入口；
- `commands/**` 只处理 Tauri wire、参数和错误投影；
- 不把 serial parser、Win32 dispatch、文件写入继续堆进 `commands/shurufa.rs`。

### 3.2 Dependencies

新增：

```toml
serialport = "=4.8.1"
```

复用：

- `serde` / `serde_json`；
- `sha2`；
- `tempfile`（tests）；
- `windows-sys = 0.61`，只补 FY1111 已需的 Win32 feature flags；
- `enigo = 0.3`，只由现有 shurufa typer owner 使用；
- Tauri runtime/state。

不采用 `tauri-plugin-serialplugin`：它解决的是把串口能力暴露为 Tauri plugin/JS API 的通用问题，而本任务的正确 owner 是 native Companion。采用插件会引入额外 permissions、JS serial API 和第二个串口 abstraction，且不能复用 FY1111 已有 strict protocol/runtime 代码。

## 4. Native Serial Ownership

### 4.1 Problem in FY1111

FY1111 当前的 `poll_runtime_event()` / `poll_network_status()` 会在命令调用期间直接读取 `SerialPortSource`。前端 100 ms / 400 ms timers 因而同时承担“让 serial 往前走”的职责。

这个行为对于独立 Companion 页面可工作，但不满足输入法：用户切回目标应用后，FyAgent renderer 是否可见不应该决定 ASR 是否进入 Agent。

### 4.2 Recommended adaptation

迁移后 `CompanionService` 成为唯一 serial reader owner：

```text
explicit command selects/opens source
  -> native pump/tick owns read
  -> decoder mutates RuntimeController status
  -> input event follows DryRun/Live state machine
  -> ASR DONE yields one admitted Agent turn
  -> UI commands only read snapshot / perform explicit actions
```

实现优先保守：继续使用 blocking `serialport` + short timeout，不为 Demo 改成 `tokio-serial` 或新 async protocol stack。

native pump 可以是 Tauri async runtime 中的轻量周期任务，也可以是一个受 service 控制的 thread。实现必须满足以下不变量：

- 同一 COM handle 只被一个 reader 消费；
- 每次 tick 有 bounded work，不长期持有 shared mutex；
- stop/close 能终止 read loop 并释放 handle；
- commands 不与 pump 竞争调用 `read()`；
- source 未打开时 pump 空闲，不做 busy loop。

UI 可以继续用低频 snapshot polling 来降低迁移量；如果实现选择事件，则只 emit bounded status/snapshot，不 emit raw serial chunks。

### 4.3 FY1111 command parity map

“全量迁移”按能力而不是按旧 command 名机械复制。FY1111 当前 12 个 commands 必须逐项有对应入口：

| FY1111 command | FyAgent semantic target |
| --- | --- |
| `list_ports` | Companion list ports action |
| `capture_target_after_delay` | delayed foreground capture action |
| `load_profile` | Companion snapshot/profile hydrate |
| `save_profile` | profile save action |
| `start_dry_run` | runtime transition to DryRun |
| `enable_live_for_run` | runtime transition to Live |
| `poll_runtime_event` | **runtime snapshot/status read**; serial consumption moves to native pump |
| `stop_runtime` | clear shortcut run/live permission; healthy device source may remain attached |
| `load_device_settings` | device settings hydrate |
| `save_device_settings` | device settings persist without silently applying other state |
| `apply_device_config` | validate/save + write `VKEY_CONFIG/1` to selected source |
| `poll_network_status` | **network/device snapshot read**; serial consumption moves to native pump |

FyAgent command names should be prefixed/scoped to avoid collisions, but all 12 behaviors remain reachable. The two old `poll_*` commands intentionally change implementation ownership, not product behavior.

## 5. Serial / Protocol Design

保留 FY1111 prefixes：

```text
Device -> Host:
VKEY_INPUT/1
VKEY_NET/1
VKEY_LOG/1
VKEY_PING/1
VKEY_REC/1
VKEY_ASR/1

Host -> Device:
VKEY_CONFIG/1
```

`BoundedLineBuffer`、1024-byte limit、strict serde DTO 和 input/network sequence tracker 直接复用。

为 Agent bridge 增加 ASR admission identity：

```text
AsrRecord.seq
  -> asr SequenceTracker
  -> duplicate/backward: drop
  -> START/FAIL: update status only
  -> DONE + non-empty text: update status + produce AsrDone{seq,text}
```

不要用 `asr_text != previous_text` 判重；用户连续说同一句话也应是两个不同 turn。判重依据是 sequence，不是文本内容。

## 6. Runtime State Machine

快捷键路径继续遵循 FY1111：

```text
Stopped
  -> source may remain open for device/network/ASR status
  -> shortcut input is not live-dispatched

DryRun
  -> decode VKEY_INPUT
  -> resolve saved mapping
  -> report only

Live
  -> decode VKEY_INPUT
  -> resolve saved mapping
  -> restore saved exact-path foreground target if needed
  -> re-read foreground identity
  -> reject on mismatch / unavailable / dirty modifiers
  -> SendInput once

Stop / SerialError
  -> liveEnabled=false
  -> user Stop returns shortcut runtime to Stopped but may retain a healthy
     selected source so network/REC/ASR can continue
  -> SerialError closes/invalidates the source and reports disconnected
```

设备 network/rec/asr 状态在 Stopped/DryRun/Live 三种模式都允许更新；它不应被 shortcut mode 人为关闭。

## 7. Shurufa Agent Bridge

### 7.1 Reuse existing Agent

现有 `commands/shurufa.rs` 已经拥有：

- agent config loading；
- context DB / recent summaries；
- `complete_turn()` Responses stream；
- `ShurufaEvent::{Started,Delta,Finished,Error}`；
- `start_typer()` + `enigo.text(delta)`；
- single-flight `running` guard。

不要复制这些代码到 Companion service。

### 7.2 Narrow refactor

把当前“从 `current_prompt()` 取输入”与“执行一个 turn”拆开：

```text
run_ingest_from_debug_prompt(app, type_into_focus)
  -> current_prompt()
  -> run_ingest_text(app, text, type_into_focus)

run_ingest_text(app, asr_text, true)
  -> existing single-flight guard
  -> existing history / complete_turn / events / persistence
  -> existing enigo streaming typer
```

这样：

- 手工 preview / global shortcut 可保留；
- serial bridge 不需要写 `prompt.txt` 再读回来；
- 同一个 Agent core 只有一个 owner。

### 7.3 Trigger

native pump 发现 admitted `AsrDone` 后，在退出 serial/runtime 锁之后触发 `run_ingest_text()`。绝对不要在持有 Companion mutex/COM access lock 时等待网络 LLM 请求完成。

Agent busy 时沿用 single-flight fail-fast：

- 本轮 ASR 标记为未进入 Agent / busy；
- UI 可显示 stable error；
- 不新增 queue/backpressure framework。

## 8. Focus / Target Semantics

两条输入路径保持独立：

1. `VKEY_INPUT/1` 快捷键：使用 FY1111 保存的 exact process target、restore 和 recheck。
2. `VKEY_ASR/1` 输入法文本：使用现有 shurufa 的“当前 OS focus”行为，stream delta 直接发送到当前焦点文本框。

这是用户当前描述的交互：先在目标文本框放好光标，再在硬件上说话。硬件操作本身不应把 FyAgent 窗口抢到前台。

本 Demo 不引入控件级 HWND/Accessibility/IME 锁定。如果用户生成期间主动切换焦点，`enigo` 仍按实时焦点行为工作；这是已知 Demo 边界。

## 9. Persistence

推荐：

```text
<FyAgent app config>/shurufacli/
  config.toml             # existing desktop Agent config
  context.db              # existing Agent history
  prompt.txt              # existing debug fallback
  companion/
    profile.json
    profile.json.bak
    device.json
```

FY1111 `ProfileStore` / `DeviceSettingsStore` 继续负责完整 validation。React 草稿不是 schema authority。

本任务不做旧 FY1111 app-config 自动导入。

## 10. Tauri / Frontend Boundary

### 10.1 Wire owner

V2 必须继续经 `shared/features/ports` + `shared/platform/tauri/feature-ports` 访问 native。可以扩展现有 `ShurufaPort`，也可以在其内部拆一个 `ShurufaCompanionPort`；具体 TypeScript 组织不锁死，但不能在 page 中散落 `invoke()`。

建议 wire 能力分组：

```text
snapshot
  - agent state/config/output
  - companion profile/runtime/device/network/asr status

actions
  - list ports
  - save profile
  - capture target
  - start dry-run
  - start live
  - stop
  - save/apply device settings
  - existing save agent config / clear history / debug run
```

raw Tauri response 统一在 adapter 做 parsing/normalization；page 不 cast unknown payload。

### 10.2 UI composition

推荐 `/shurufa` 结构：

```text
Header: 输入法 + runtime/device chips + clear/debug action

Companion main surface
  Serial row
  Device / ASR settings collapsible
  Foreground target row
  Three fixed mapping rows
  Runtime controls
  Last event / notice

Voice / Agent surface
  Latest raw ASR
  Agent running / optimized output
  Agent config collapsible
  Manual debug textarea + preview (secondary)
```

视觉上参考 FY1111 的紧凑操作台，不复制它的白卡/独立窗口样式。使用 FyAgent V2 tokens 和既有 primitive。

## 11. Failure Semantics

| Condition | Required behavior |
| --- | --- |
| no serial port / open failure | show unavailable, no runtime/live |
| invalid/overlong serial line | ignore, no dispatch, no Agent |
| duplicate/backward input seq | drop |
| input forward gap | accept current valid event + report missed count |
| duplicate/backward ASR seq | drop, no Agent |
| ASR START | status only |
| ASR FAIL | status/error only, no Agent |
| ASR DONE empty text | no Agent |
| Agent config invalid/missing | preserve raw ASR in status, Agent error, no typing |
| Agent already running | busy/error, no second concurrent turn |
| wrong foreground target in Live shortcut path | zero shortcut dispatch |
| dirty modifiers | zero shortcut dispatch |
| serial read error | clear live permission and close/invalidate source |
| stop | clear live permission deterministically |

## 12. Compatibility / Rollback

- 保持 `VKEY_* /1` wire prefixes，不要求 firmware 同步升级。
- 保持现有 shurufa config/history 数据格式。
- 手工 prompt/preview/global shortcut 尽量不删除，方便快速回归和无硬件调试。
- 新 Companion files 使用独立子目录，删除该子目录即可回退 Companion persistence，不影响 Agent config/context DB。
- 如果 native background pump 实现出现问题，可以临时回退为 explicit start + snapshot polling，但不得回退到“只有 React read COM 才会触发 ASR Agent”的最终架构。

## 13. Deliberately Deferred

- macOS functional parity；
- async serial rewrite；
- Tauri serial plugin / Web Serial；
- ASR queue/backpressure；
- formal Windows IME；
- SecretRef migration；
- automated hardware CI。
