# Shurufa Companion Serial and Agent Bridge

## 1. Scope / Trigger

Read this before changing `/shurufa`, `ShurufaPort`, `commands/shurufa.rs`,
`commands/shurufa_companion.rs`, or `services/shurufa_companion/**`.

This is a Windows-first Demo contract on `demo/shurufa`. It does **not**
authorize macOS Companion HIL, a Tauri serial plugin, Web Serial, a second
LLM/history/typing stack, ASR queues, firmware redesign, CI/release work, or
SecretRef migration.

Trigger: new/changed `shurufa_companion_*` commands, serial reader ownership,
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
shurufa_companion_list_ports() -> Vec<String>
shurufa_companion_capture_target() -> CompanionTarget
shurufa_companion_get_snapshot() -> CompanionSnapshot
shurufa_companion_save_profile(draft) -> CompanionProfile
shurufa_companion_start_dry_run() -> CompanionRuntime
shurufa_companion_enable_live() -> CompanionRuntime
shurufa_companion_stop() -> CompanionRuntime
shurufa_companion_save_device_settings(draft) -> CompanionDeviceSettings
shurufa_companion_apply_device_config(request) -> CompanionNetwork
```

V2 port methods live on the existing `ShurufaPort` and are parsed once in
`src/v2/shared/platform/tauri/feature-ports/shurufa.ts`. Pages must not
`invoke()`.

## 3. Contracts

```text
serialport 4.8.1
  -> native pump (only reader of the COM handle)
  -> VKEY decoder
       INPUT: DryRun report / Live restore+verify+SendInput
              (ENCODER_CW/CCW/PRESS + BUTTON_A/B)
       NET/LOG/PING/REC/SENSOR: status only
       ASR DONE(seq, trimmed text): exactly-once admission
       ASR START/FAIL without text: keep previous asrText
  -> run_ingest_text(text, type_into_focus=true)
  -> existing shurufacli Config/Store/complete_turn
  -> existing enigo typer -> current OS focus
```

- One COM handle has exactly one native reader. Snapshot/status commands never
  call `read()` and never call `available_ports()`.
- Snapshot must `try_lock` the pump mutex and fall back to the last published
  snapshot. A hung COM read during Wi-Fi join must not freeze the UI thread.
- `list_ports` is explicit refresh only (FY1111 behavior). It runs off the UI
  thread via `spawn_blocking`.
- `apply_device_config` pauses the pump, writes `VKEY_CONFIG/1`, then
  `drain_while_stopped` like FY1111, and must not wait on the pump lock
  forever (2s timeout).
- React may poll `getCompanionSnapshot` (~400ms) for projection only. Page
  lifecycle must not determine whether COM is consumed. Polls must not overlap
  and must not re-enumerate COM ports.
- Agent trigger happens after Companion/runtime/serial locks are released.
- Persistence: `<app-config>/shurufacli/companion/{profile,device}.json`.
  Do not write ASR text into `prompt.txt`.
- Device SiliconFlow `{ssid,password,apiKey,model}` and desktop Agent
  `{url,model,apiKey,maxSummaries,timeoutSecs}` are separate stores, labels,
  and call chains.
- Default baud `115200`. Default device model
  `XingChenAGI/XingChenASR-V3.2-Ultra`.
- New commands must land in all three places:
  `generate_handler!`, `legacy-application-commands.toml`, and the V2
  `tauriAclContract` renderer-invoke count.
- Serial crate is pinned `serialport = "=4.8.1"`. Do not add
  `tauri-plugin-serialplugin` or Web Serial.
- Wi-Fi password, SiliconFlow key, and Agent API key must not appear in
  `lastEvent`, ordinary logs, or status-event plaintext.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| No port / open failure | unavailable; no DryRun/Live |
| Line > 1024 bytes, invalid UTF-8/JSON, unknown prefix/field/input/version | ignore; no dispatch; no Agent |
| `VKEY_INPUT` duplicate/backward seq | drop |
| `VKEY_INPUT` forward gap | accept current + `gapMissed` |
| `VKEY_ASR` duplicate/backward seq | `lastAsrAdmission=duplicate`; no Agent |
| ASR START / FAIL | status only (`start` / `fail`); keep previous `asrText` |
| ASR FAIL reason=CANCEL | status only; never admit Agent |
| `VKEY_SENSOR/1` | merge `pir` / `tofMm` / `sensorState`; no shortcut or Agent |
| Saved 3-mapping profile | load + runtime still valid; UI hydrates BUTTON_A/B defaults; next save requires 5 |
| ASR DONE + empty/whitespace text | `empty`; no Agent |
| ASR DONE + new seq + non-empty text | `admitted` + one `run_ingest_text` |
| Same ASR text, new seq | second admission (do not dedupe by text) |
| Agent already running | `正在生成中，请稍后再试`; `busy`; no second turn |
| Agent config missing | preserve raw ASR; Agent error; no typing |
| Live target mismatch / dirty modifiers | zero shortcut dispatch |
| Serial read error | clear live; close/invalidate source |
| Snapshot while pump holds the COM mutex | return last snapshot; no `available_ports()` |
| Apply while pump is mid-read | pause pump; 2s try-lock; then write + drain |
| Stop | clear live; healthy source may stay attached |
| Browser / non-Windows capture-restore-dispatch | fail closed; no fake hardware state |

## 5. Good/Base/Bad Cases

- Good: hardware ASR DONE → native admission → existing Agent stream →
  `enigo` into the current focus box; `/shurufa` only displays snapshot/events.
- Base: Stopped + healthy source still pumps NET/REC/ASR; shortcut Live is
  off after restart and is never persisted on.
- Bad: React `setInterval` is the only thing that calls serial `read()`, or
  ASR is written to `prompt.txt` before ingest, or two COM readers share one
  handle, or every snapshot enumerates Windows COM ports on the UI thread.

## 6. Tests Required

- Rust `shurufa_companion`: duplicate/backward ASR, same text + new seq admits
  twice, whitespace DONE is `empty`, invalid/overlong line ignored, input gap,
  serial error clears live, snapshot does not consume serial, Stopped DONE
  admits without shortcut dispatch, busy projection uses the exact Chinese
  string.
- Rust `commands::shurufa`: first `run_ingest_text` admission succeeds; second
  concurrent call returns `正在生成中，请稍后再试`.
- V2: `featurePorts` freezes command names/parser shape; browser methods
  reject `NATIVE_ONLY_ERROR`; `tauriAclContract` keeps renderer invokes ⊆
  registered ⊆ ACL; `/shurufa` page test covers layout and port boundary.
- Windows HIL (manual, not CI): COM select, `VKEY_CONFIG/1`, DryRun input,
  Live shortcut, REC, ASR DONE → Agent → focus box, duplicate seq no replay,
  Stop clears live.

Assertion points: `read()` is unreachable from snapshot; admitted seq is
unique; busy does not start a second typer; device and Agent API keys never
share a field.

## 7. Wrong vs Correct

#### Wrong
```ts
// page drives COM
setInterval(() => invoke("poll_runtime_event"), 100);
```

#### Correct
```ts
// page reads a native-owned snapshot; pump already consumed the port
const snapshot = await ports.shurufa.getCompanionSnapshot();
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
