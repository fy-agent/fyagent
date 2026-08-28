# 完整迁移 Companion 并接入输入法 Agent

## Goal

把 `/Users/pythonrust/Desktop/projects/FY1111/companion` 当前已经实现的 Companion 功能整体迁入 FyAgent `demo/shurufa` 分支，并与现有输入法 Agent 形成一条可演示的 Windows 端完整链路：

```text
硬件按键录音
  -> 板端 ASR
  -> USB 串口 VKEY_ASR/1 DONE
  -> FyAgent native Companion
  -> 现有 shurufa Agent 做口语/切词/提示词优化
  -> enigo 流式写入当前焦点文本框
```

迁移目标是“功能完整、实现快速、边界清楚”，不是重写 FY1111，也不是把它作为第二个独立 Tauri 应用嵌套进来。UI 只参考 FY1111 的信息架构和布局，用 FyAgent V2 现有组件重新组合；native 侧优先直接迁移已经在 FY1111 中工作的 Rust 模块和协议实现。

本任务是 Demo 优先级。Windows 是当前唯一需要工作的产品平台；macOS 功能适配、完整 CI 扩展、生产级安全重构和大规模测试不进入本轮。

## Confirmed Facts

- FyAgent 当前分支为 `demo/shurufa`，工作树在任务创建前是干净的。
- FyAgent 已有输入法 Agent：`src-tauri/shurufacli/**` + `src-tauri/src/commands/shurufa.rs`，已经支持 OpenAI-compatible Responses 流式输出、历史摘要、配置持久化和 `enigo.text()` 流式输入当前焦点。
- 当前 `/shurufa` 页面仍以手工 textarea 作为主要输入源；native `run_ingest(..., true)` 已具备把 Agent delta 写进焦点输入框的能力。
- FY1111 Companion 是 Tauri 2 + React/Vite。前端通过一个 `CompanionHost` 访问 native；原始串口解析、文件持久化、前台进程、运行态和 Win32 输入派发均在 Rust。
- FY1111 当前串口依赖为 `serialport = 4.8.1`。它已经实现端口枚举、115200 打开、100 ms timeout、读写、flush、clear buffer、1024-byte bounded line decoder。
- FY1111 当前支持并解析：`VKEY_INPUT/1`、`VKEY_NET/1`、`VKEY_LOG/1`、`VKEY_PING/1`、`VKEY_REC/1`、`VKEY_ASR/1`；host 向设备写 `VKEY_CONFIG/1`。
- FY1111 的 Windows 前台捕获、窗口恢复和 `SendInput` 逻辑是 Windows 专用。用户已明确本轮不要求 macOS 适配。
- FY1111 `VKEY_ASR/1` 已携带 `seq`、`state`、`text` / `reason`，但当前 Companion 仅把 ASR 投影到 network status；还没有自动调用输入法 Agent。
- FyAgent 已经依赖 `sha2`、`windows-sys 0.61`、`enigo 0.3`、Tauri 2、React 19，以及 V2 `Button` / `Input` / `SecretInput` / `Spinner` / `InlineNotice` / `Collapsible` 等 UI owners。

## Requirements

### R1 — FY1111 Companion 用户功能整体迁移

本轮迁移不能只拿串口 ASR 文本。以下 FY1111 Companion 功能全部进入 FyAgent：

- 串口列表刷新、端口选择、波特率投影；默认继续使用 115200。
- Companion profile 的 load/save/revision/backup 行为。
- 3 个固定物理输入：`ENCODER_CW`、`ENCODER_CCW`、`ENCODER_PRESS`。
- 3 行映射的 display name、快捷键捕获、canonicalization、重复检查和 bounded allowlist。
- 3 秒后捕获前台目标，保存精确 process name + normalized process path。
- `STOPPED` / `DRY_RUN` / `LIVE` 运行态。
- Dry-run 读取真实串口、解析真实输入、只投影动作而不构造 live dispatcher。
- Live 模式按保存目标恢复窗口、再次核验 foreground identity、检查 modifier 状态后派发快捷键。
- Stop / serial error 清理运行权限；重启后 live 永远默认关闭且不得持久化为开启。
- Device settings：SSID、Wi-Fi 密码、SiliconFlow API Key、ASR model。
- Device settings load/save/apply；host 通过 `VKEY_CONFIG/1` 下发设备配置。
- Network status：unknown/disconnected/connecting/connected/failed、SSID、IP、RSSI、reason、heartbeat。
- Ping 状态、串口日志、录音状态/时长/采样/RMS/峰值/静音/失败原因。
- ASR START/DONE/FAIL、最终文本和失败原因。
- 2.4 GHz / 5 GHz SSID 提示语义继续保留。

浏览器 fixture、FY1111 独立 app shell、独立 package/lockfile 和测试脚手架不属于“用户功能”，不要求原样迁移。

### R2 — 串口 ASR 成为输入法 Agent 的正式输入源

- `VKEY_ASR/1 { state: "DONE", text: ... }` 是正常演示链路的正式输入源。
- ASR DONE 必须在 native Companion 边界被识别并触发现有 shurufa Agent，不能依赖某个 React 页面当前可见、某个组件 mounted 或某次 UI polling 才触发。
- Agent 输入使用该次 ASR 的 `text` 原文。现有 history 只继续用于 shurufa Agent 自身的消歧/摘要，不把 Companion 状态、串口日志或快捷键事件混进 Agent prompt。
- Agent 输出继续使用现有 streaming delta；正常硬件链路要求 `type_into_focus = true`，边生成边写入当前操作系统焦点文本框。
- 当前 FY1111 保存的 shortcut target 继续服务 `VKEY_INPUT/1` 快捷键派发；ASR 输入法链路默认仍遵循现有 shurufa “当前焦点文本框”语义，不强制绑定到保存的 shortcut target。
- 为避免同一 ASR 状态被 snapshot/poll 重放，native 必须使用 `VKEY_ASR/1` 的序号或等价 source identity 做 exactly-once admission。重复/倒退的 ASR 记录不得再次调用 Agent。
- 本 Demo 维持 shurufa Agent 的 single-flight 语义。上一轮仍在生成时的新 ASR 不要求实现队列；必须明确报告 busy/error，不能悄悄覆盖正在执行的 turn，也不能重复注入。

### R3 — 保留手工输入作为调试后门，不再作为正式链路

- 当前 textarea / `shurufa_set_prompt` / 手工预览能力允许保留，以便没有硬件时快速验证 Agent。
- UI 文案和状态必须明确：正式演示输入来自串口 ASR，手工输入只是 debug fallback。
- 不得因为保留 debug 输入而让自动串口链路继续依赖 `prompt.txt`；ASR turn 必须直接把本轮文本传给 Agent。
- 现有 `Ctrl+M` / `Cmd+M` 行为可以在本轮继续保留以减少回归，但它不是硬件链路的触发条件。

### R4 — 串口协议与 native 边界保持严格

- 直接复用/迁移 FY1111 的 bounded line、strict JSON 和 prefix decoder，不把原始串口字符串交给 React 解析。
- 继续拒绝或忽略：超过 1024 bytes、invalid UTF-8、invalid JSON、unknown prefix、unknown fields、unknown input ID、unsupported version。
- `VKEY_INPUT/1` 保持 duplicate/backward drop 和 forward-gap report。
- `VKEY_NET/1` 保持 sequence tracking；为自动 Agent bridge，`VKEY_ASR/1` 也必须有明确的 duplicate/backward 防重语义。
- 串口只允许选择协议中定义的数据。设备不得通过串口注入快捷键、进程路径、Agent URL、模型配置文件路径或任意命令。
- Device password、SiliconFlow API key、shurufa Agent API key 不得进入普通日志、错误消息或 UI status event 的明文回显。

### R5 — native 串口读循环不能由页面生命周期拥有

- 迁移后 native 是串口 reader 的唯一 owner；React 只能发显式动作命令和读取/订阅 typed snapshot。
- 不允许保留“React 每 100 ms 调 `poll_runtime_event` 才真正 read COM”的架构作为自动 ASR 的唯一驱动，否则隐藏页面或未访问页面会使输入法失效。
- 推荐在打开 serial source 后由一个轻量 native pump/tick 持续读取并更新 runtime snapshot；实现可以复用 FY1111 `RuntimeController`，无需引入新的 async serial stack。
- UI 获取状态可使用 snapshot polling 或小数据 Tauri event；原始字节和高频串口 chunk 不需要发送到 renderer。
- 同一个 COM handle 只有一个 reader owner，避免 UI poll、network poll 和 Agent bridge 三条路径竞争消费同一串口。

### R6 — UI 迁移布局，不迁移 FY1111 组件实现

- `/shurufa` 继续是 FyAgent V2 feature page，不嵌入第二个 WebView/Tauri app。
- 参考 FY1111 的单窗布局和信息顺序，大体保留：
  1. 串口选择 + runtime 状态；
  2. 设备/云端设置折叠区 + 网络/录音/ASR 状态；
  3. 前台目标；
  4. 三路固定输入映射；
  5. Save / Dry-run / Live / Stop 控件；
  6. 最后事件/notice；
  7. 输入法 Agent 配置、最近 ASR、优化结果和 debug fallback。
- Button/Input/SecretInput/Spinner/InlineNotice/Collapsible/Badge 等优先用 `src/v2/shared/ui` 现有 owner。
- 串口 `<select>` 可按 V2 现有页面做法使用原生 select + FyAgent token/class，不为一个下拉框新增组件库。
- 快捷键捕获控件可从 FY1111 `ChordField` 逻辑做最小迁移并改用 FyAgent 样式；它当前只有 `/shurufa` 一个真实消费者时保持 page-local 即可。
- 不复制 FY1111 `app.css` 作为第二套设计系统；只迁移必要布局结构和状态语义。

### R7 — 两套云端配置必须明确区分

- “设备转写配置”仍是 FY1111 的 SiliconFlow `apiKey + model`，由 host 下发到硬件。
- “输入法 Agent 配置”仍是当前 shurufacli 的 OpenAI-compatible `url + model + api_key + max_summaries + timeout`，只在桌面端使用。
- 两者 UI 标签、保存路径和调用链必须分开，不能因为都叫 API Key / model 就合并字段或相互覆盖。

### R8 — 持久化语义迁入 FyAgent 自己的配置目录

- FY1111 的 profile/device persistence 行为迁入 FyAgent，但不依赖 FY1111 独立应用的 app-config path。
- 推荐把 Companion 文件放到现有 shurufa 数据目录的子目录，例如 `shurufacli/companion/profile.json` 和 `device.json`，与 `config.toml` / `context.db` / debug prompt 隔离。
- Profile 继续保留版本、revision hash、stale-write rejection、change backup。
- Device settings 继续做 bounded validation。
- 本 Demo 不要求自动读取或迁移旧 FY1111 应用已经落盘的配置；目标是迁移功能，不是跨应用数据迁移。

### R9 — Reuse-first / 不造轮子

- native 优先直接迁移 FY1111 已工作的 `input.rs`、`serial.rs`、`network.rs`、`device_settings.rs`、`profile.rs`、`runtime.rs`、`target.rs`、`windows_foreground_restore.rs`，再按 FyAgent 模块边界做最小适配。
- FyAgent 已有 Agent、history、Responses stream 和 `enigo` typing 必须复用；不得实现第二个 LLM client、第二个 context DB 或第二个输入注入器。
- 串口新增依赖优先继续使用 FY1111 已验证的 `serialport 4.8.1`；不引入 Web Serial，也不引入 Tauri serial plugin 让 renderer 直接掌握串口。
- FyAgent 已有 `windows-sys 0.61`；只补 FY1111 所需 feature flags，除非实现证据证明现有 `windows` owner 可以无成本复用。不要仅为 API 风格统一重写已经工作的 Win32 代码。
- 新 native 业务代码按 FyAgent modular-monolith 规则放在 private service/facade 后面；Tauri commands 只做 wire translation/delegation。

### R10 — Windows-only 快速 Demo 约束

- Windows 是唯一需要真正工作的 native target。
- macOS 本轮只需不被无关改动破坏到无法开发；不要求实现前台捕获、窗口恢复、快捷键派发或 HIL 等价能力。
- 不新建 CI workflow，不修改 release pipeline，不做 installer/release scope。
- 测试只做能阻止明显迁移错误的 focused checks；不为这个 Demo 补齐生产级矩阵。
- 不重构 firmware。只有当当前 `VKEY_* /1` 协议与迁移实现存在真实不兼容时，才允许最小、向后兼容的 firmware/protocol 调整，并在执行时单独说明。

## Acceptance Criteria

- [ ] `/shurufa` 能刷新并选择真实 Windows 串口，显示当前 baud/runtime state。
- [ ] FY1111 profile 能在 FyAgent 中 load/save，revision stale check、backup、三路固定 mapping 和 chord/name validation 保持工作。
- [ ] 能执行 3 秒 foreground target capture，并在 UI 显示保存的 process identity。
- [ ] Dry-run 能从真实 serial source 接收 `VKEY_INPUT/1` 并只显示解析后的 mapping，不发送快捷键。
- [ ] Live 能按 FY1111 语义恢复保存目标、重新验证目标、检查 modifier 并发送对应快捷键；Stop/serial error 后 live permission 清零。
- [ ] Device settings 能 load/save/apply，`VKEY_CONFIG/1` 能写到所选端口；2.4 GHz/5 GHz 提示和 network fail reason 正常。
- [ ] UI 能显示 FY1111 已有 network/ping/log/record/ASR 状态字段。
- [ ] 同一 native serial reader 同时处理 input/network/log/ping/rec/asr，不由 React 页面生命周期决定 COM 是否继续被读取。
- [ ] 收到新的 `VKEY_ASR/1 DONE` 时，native exactly-once 把 `text` 交给现有 shurufa Agent；duplicate/backward ASR 不重复触发。
- [ ] Agent delta 使用现有 `enigo` 实时写入当前焦点文本框，最终优化文本同时投影到 `/shurufa` 页面。
- [ ] Agent 正在运行时的第二个 ASR 不会覆盖当前 turn 或产生双重输入；UI/native 有明确 busy/error 结果。
- [ ] 正常硬件链路不依赖 textarea、`prompt.txt`、手工 Preview 或 `Ctrl+M`；这些只可作为 debug fallback。
- [ ] SiliconFlow 设备配置与 desktop Agent 配置在 UI、native DTO 和持久化上保持独立。
- [ ] `/shurufa` 使用 FyAgent V2 组件/样式重新组合 FY1111 布局，没有复制第二套 Companion app shell 或整份 FY1111 CSS。
- [ ] 除 `serialport` 外没有为本需求新增重复能力依赖；没有引入 Tauri serial plugin/Web Serial/第二个 LLM 或第二个 typing engine。
- [ ] focused Rust decoder/runtime/bridge tests、V2 typecheck、shurufa focused frontend tests通过；真实 COM + Win32 快捷键 + ASR→Agent→文本框链路在 Windows 上完成一次人工 Demo 验收。

## Out of Scope

- macOS Companion 功能适配或 macOS HIL。
- 自动跨应用迁移 FY1111 已保存的 profile/device JSON。
- 改写板端 ASR、麦克风、LCD、Wi-Fi 实现。
- BLE、Web Serial、移动端串口或新的无线传输方案。
- 多设备同时连接、多 COM 自动故障切换、串口热插拔自动重连框架。
- 多 ASR turn 排队、后台长队列、复杂 backpressure；本 Demo 保持 Agent single-flight。
- 为输入法实现正式 Windows IME、剪贴板管线或应用专用插件。
- SecretRef 全量迁移、凭据体系重构；本轮只保证现有 demo 凭据不被日志/事件明文泄漏。
- 新 CI、release、installer、自动化 Windows HIL、大规模 E2E 或视觉回归体系。

## Risks / Deferred Items

- FY1111 当前靠 UI polling 驱动 serial read。自动输入法若照搬这一点会受页面生命周期影响，因此本任务需要把“读串口的 owner”提升到 native service；这是必要的适配，不是额外产品扩展。
- FY1111 `NetworkStatus` 当前不暴露 ASR sequence；自动 bridge 需要把 ASR sequence/identity 保留到 native admission 层，避免相同文本或 snapshot 被重复处理。
- 现有 shurufa Agent 的 stream typing 跟随操作系统当前焦点。用户在生成过程中主动切换焦点仍可能改变输入目标；本 Demo 不新增正式 IME/控件级锁定机制。
- FyAgent 当前 shurufa Agent config 和 FY1111 device settings 都是本地 demo 配置。更强的 credential storage 可以后续单独做，不应阻塞本轮演示迁移。

## Planning Evidence

详细源文件映射、外部调研和多轮自审记录见 `research/planning-evidence.md`。
