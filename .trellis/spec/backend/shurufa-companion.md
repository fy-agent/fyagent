# Shurufa Companion USB Link and Agent Bridge

## 1. Scope / Trigger

Read this before changing `/shurufa`, `ShurufaPort`, `commands/shurufa.rs`,
`commands/shurufa_companion.rs`, or `services/shurufa_companion/**`.

This is a Windows-first Demo contract on `demo/shurufa`. It does **not**
authorize macOS Companion HIL, a second WinUSB/rusb host transport, a Tauri
serial plugin, Web Serial, a second LLM/history/typing stack, ASR queues,
firmware redesign, CI/release work, or SecretRef migration.

Trigger: new/changed `shurufa_companion_*` commands, Board C HID auto-link,
`VKEY_ASR/1` admission, or the desktop Agent ingest path.

## 2. Signatures

Private owner: `src-tauri/src/services/shurufa_companion/**`.
Thin transport: `src-tauri/src/commands/shurufa_companion.rs`.

```rust
pub async fn run_ingest_text<R: Runtime>(
    app: AppHandle<R>,
    text: String,
    type_into_focus: bool,
) -> Result<String, String>;

pub async fn run_ingest<R: Runtime>(
    app: AppHandle<R>,
    type_into_focus: bool,
) -> Result<String, String>; // debug/hotkey: current_prompt() -> run_ingest_text
```

Tauri commands (camelCase wire):

```text
shurufa_companion_list_ports() -> Vec<String>  // USB presence projection
shurufa_companion_capture_target() -> CompanionTarget
shurufa_companion_get_snapshot() -> CompanionSnapshot
shurufa_companion_save_profile(draft) -> CompanionProfile
shurufa_companion_start_dry_run() -> CompanionRuntime
shurufa_companion_enable_live() -> CompanionRuntime
shurufa_companion_stop() -> CompanionRuntime
shurufa_companion_save_device_settings(draft) -> CompanionDeviceSettings
shurufa_companion_apply_device_config(request) -> CompanionNetwork
```

Board C identity:

```text
VID 0x303A
PID 0x82D0
link id usb:ventured
logical baud 115200
HID 64-byte RawHID; report[0]=payload length; payload max 63
host write prefers 65 bytes with report-id 0
```

V2 port methods live on the existing `ShurufaPort` and are parsed once in
`src/v2/shared/platform/tauri/feature-ports/shurufa.ts`. Pages must not
`invoke()`.

## 3. Contracts

```text
Windows hidapi 2.6.3
  -> native pump (only reader of the HID source; also owns auto-discover)
  -> shared LinkDecoder
       INPUT: DryRun report / Live restore+verify+SendInput
              (ENCODER_CW/CCW/PRESS + BUTTON_A/B)
       NET/LOG/PING/REC/SENSOR: status only
       ASR DONE(seq, trimmed text): exactly-once admission
       ASR START/FAIL without text: keep previous asrText
  -> run_ingest_text(text, type_into_focus=true)
  -> existing shurufacli Config/Store/complete_turn
  -> existing enigo typer -> current OS focus
```

- Formal Board C traffic uses `UsbLinkSource` + the same `LinkDecoder` as the
  leftover `SerialPortSource`. Do not copy a second USB decoder.
- One HID source has exactly one native reader. Snapshot/status commands never
  call `read()` and never enumerate COM ports. `UsbLinkSource::poll_event`
  takes one HID read per pump tick, matching `SerialPortSource`. Timeout,
  would-block, empty, and generic `hidapi error` reads are idle (`Ok(None)`),
  not disconnect. Only an explicit device-gone / access error is `SerialError::Read`.
- USB auto-discover/reconnect belongs to `spawn_pump`. About every 500ms, if
  runtime has no source, the pump tries `UsbLinkSource::open()`. Success
  attaches `usb:ventured / 115200` and projects `ports = ["usb:ventured"]`.
  Absence projects `[]`. Page polling must not attach USB.
- Snapshot must `try_lock` the pump mutex and fall back to the last published
  snapshot. A hung HID read during Wi-Fi join must not freeze the UI thread.
- `list_ports` is an explicit USB-presence refresh, not COM enumeration. The
  `/shurufa` page does not call it to drive connection.
- `apply_device_config` pauses the pump, writes `VKEY_CONFIG/1` over HID, then
  `drain_while_stopped` like FY1111, and must not wait on the pump lock
  forever (2s timeout).
- `start_dry_run` / `enable_live` / `apply_device_config` open or reuse the USB
  link. They ignore a saved `COMx` name.
- React may poll `getCompanionSnapshot` (~400ms) for projection only, including
  `snapshot.ports.includes("usb:ventured")`. Page lifecycle must not determine
  whether HID is consumed. Polls must not overlap and must not enumerate HID.
- Agent trigger happens after Companion/runtime/serial locks are released.
- Persistence: `<app-config>/shurufacli/companion/{profile,device}.json`.
  Do not write ASR text into `prompt.txt`.
- Device SiliconFlow `{ssid,password,apiKey,model}` and desktop Agent
  `{url,model,apiKey,maxSummaries,timeoutSecs}` are separate stores, labels,
  and call chains.
- Saved `profile.serial` schema stays `{port,baud}` / version 1. Load may
  memory-normalize `COMx` to `usb:ventured` while keeping the original
  revision. Next save upgrades to five mappings + USB link, writes `.bak`,
  and uses the old revision for optimistic concurrency.
- `validate_loaded()` still accepts a three-mapping profile. UI hydrate fills
  BUTTON_A/B defaults, but a missing-button fallback must not reuse a chord
  already present on a saved mapping (shift to `CTRL+1`…`CTRL+5`). Save still
  requires five mappings. User-saved collisions stay visible as errors.
- Default baud `115200`. Default device model
  `XingChenAGI/XingChenASR-V3.2-Ultra`.
- Fresh / fallback shortcut defaults:
  `ENCODER_CW=CTRL+SHIFT+]`, `ENCODER_CCW=CTRL+SHIFT+[`,
  `ENCODER_PRESS=CTRL+,` 新建窗口, `BUTTON_A=CTRL+N` 新建,
  `BUTTON_B=ENTER` 确认动作. `ENCODER_PRESS` UI label is `GPIO8 上拉按键`.
  Existing user mappings are not rewritten.
- hidapi is Windows-only. Non-Windows `UsbLinkSource` is fail-closed so Mac
  hosts can compile and run logic tests. That is not macOS Companion support.
- New commands must land in all three places:
  `generate_handler!`, `legacy-application-commands.toml`, and the V2
  `tauriAclContract` renderer-invoke count. This USB change reuses the
  existing `shurufa_companion_*` surface.
- Do not add `tauri-plugin-serialplugin`, Web Serial, or a second WinUSB host.
- Wi-Fi password, SiliconFlow key, and Agent API key must not appear in
  `lastEvent`, ordinary logs, or status-event plaintext.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Board C not inserted / HID open failure | unavailable; `ports=[]`; no DryRun/Live/Apply |
| Line > 1024 bytes, invalid UTF-8/JSON, unknown prefix/field/input/version | ignore; no dispatch; no Agent |
| HID report length illegal / payload > 63 | ignore frame |
| `VKEY_INPUT` duplicate/backward seq | drop |
| `VKEY_INPUT` forward gap | accept current + `gapMissed` |
| `VKEY_ASR` duplicate/backward seq | `lastAsrAdmission=duplicate`; no Agent |
| ASR START / FAIL | status only (`start` / `fail`); keep previous `asrText` |
| ASR FAIL reason=CANCEL | status only; never admit Agent |
| `VKEY_SENSOR/1` | merge `pir` / `tofMm` / `sensorState`; no shortcut or Agent |
| Saved 3-mapping COM profile | load + runtime still valid; memory USB normalize; UI hydrates BUTTON_A/B without inventing a colliding fallback; next save requires 5 + `usb:ventured` |
| ASR DONE + empty/whitespace text | `empty`; no Agent |
| ASR DONE + new seq + non-empty text | `admitted` + one `run_ingest_text` |
| Same ASR text, new seq | second admission (do not dedupe by text) |
| Agent already running | `正在生成中，请稍后再试`; `busy`; no second turn |
| Agent config missing | preserve raw ASR; Agent error; no typing |
| Live target mismatch / dirty modifiers | zero shortcut dispatch |
| HID read error | clear live; close/invalidate source; pump re-enters discover |
| Re-insert USB | pump reopens HID; `/shurufa` need not be open |
| Snapshot while pump holds the HID mutex | return last snapshot; no HID enumerate |
| Apply while pump is mid-read | pause pump; 2s try-lock; then HID write + drain |
| Stop | clear live; healthy USB source may stay attached |
| Browser / non-Windows capture-restore-dispatch | fail closed; no fake hardware state |

## 5. Good/Base/Bad Cases

- Good: insert Board C → native pump attaches HID → hardware ASR DONE →
  native admission → existing Agent stream → `enigo` into the current focus
  box; `/shurufa` only displays snapshot/events, including 已插入/未插入.
- Base: Stopped + healthy USB source still pumps NET/REC/ASR; shortcut Live is
  off after restart and is never persisted on.
- Bad: React `setInterval` is the only thing that opens HID or calls serial
  `read()`, or ASR is written to `prompt.txt` before ingest, or two HID
  readers share one device, or every snapshot enumerates USB devices on the
  UI thread, or `poll_network_status` is the attach owner.

## 6. Tests Required

- Rust `shurufa_companion`: HID pack/unpack (64-byte report, 63-byte payload,
  report-id 0, illegal length); shared `LinkDecoder` after HID framing keeps
  INPUT/NET/REC/SENSOR/ASR admission; duplicate/backward ASR; same text + new
  seq admits twice; whitespace DONE is `empty`; invalid/overlong line ignored;
  input gap; HID/serial error clears live; snapshot does not consume the
  source; Stopped DONE admits without shortcut dispatch; busy projection uses
  the exact Chinese string; old 3-mapping COM profile hydrates USB in memory
  and save upgrades to five mappings + `usb:ventured` without breaking stale
  revision / `.bak`; hydrate of `ENCODER_PRESS=ENTER` does not leave
  `BUTTON_B=ENTER`; idle HID read errors do not emit `serial input stopped`.
- Rust `commands::shurufa`: first `run_ingest_text` admission succeeds; second
  concurrent call returns `正在生成中，请稍后再试`.
- V2: `featurePorts` freezes command names/parser shape; browser methods
  reject `NATIVE_ONLY_ERROR`; `tauriAclContract` keeps renderer invokes ⊆
  registered ⊆ ACL; `/shurufa` shows Board C USB 已插入/未插入 and no COM
  picker; fresh defaults use the three new shortcut chords.
- Windows HIL (manual, not CI): insert USB → 已插入 without COM select,
  `VKEY_CONFIG/1` over HID, DryRun input, Live shortcut, REC, ASR DONE →
  Agent → focus box, duplicate seq no replay, unplug clears live, re-insert
  restores pump without reopening `/shurufa`. Old COM firmware cannot verify
  this path.

Assertion points: `read()` is unreachable from snapshot; admitted seq is
unique; busy does not start a second typer; device and Agent API keys never
share a field; page snapshot polling is projection only.

## 7. Wrong vs Correct

#### Wrong
```ts
// page drives COM / HID
setInterval(() => invoke("poll_runtime_event"), 100);
```

#### Correct
```ts
// page reads a native-owned snapshot; pump already consumed HID
const snapshot = await ports.shurufa.getCompanionSnapshot();
const inserted = snapshot.ports.includes("usb:ventured");
```

#### Wrong
```rust
if asr_text == previous_text { return; } // drops a repeated utterance
```

#### Correct
```rust
match asr_tracker.accept(seq) {
    SequenceOutcome::DuplicateOrBackward => AsrAdmission::Duplicate,
    SequenceOutcome::Accepted if !text.trim().is_empty() => AsrAdmission::Admitted,
    _ => AsrAdmission::Empty,
}
```
