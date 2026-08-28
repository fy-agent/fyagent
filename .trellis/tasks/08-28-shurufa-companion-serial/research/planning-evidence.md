# Planning Evidence and Self-review

Date: 2026-08-28

## 1. Local Evidence — FY1111 Companion

### Frontend surface

- `companion/src/App.tsx`
  - serial refresh/select；
  - profile hydrate/save；
  - delayed foreground capture；
  - three fixed mapping rows；
  - DryRun/Live/Stop；
  - network/apply settings；
  - runtime/network polling and user notices。
- `companion/src/SettingsPanel.tsx`
  - SSID/password/SiliconFlow key/model；
  - 2.4 GHz warning；
  - network chip、RSSI、heartbeat、ping；
  - REC state/statistics；
  - ASR START/DONE/FAIL/text；
  - serial log。
- `companion/src/ChordField.tsx` + `validation.ts`
  - real keyboard chord capture；
  - `CTRL/ALT/SHIFT` + bounded primary allowlist；
  - canonical chord and duplicate identity；
  - display name and device setting bounds。
- `companion/src/types.ts` / `host.ts`
  - complete typed UI/native contract。

### Native surface

- `companion/src-tauri/src/serial.rs`
  - `serialport::available_ports()`；
  - `serialport::new(...).timeout(100ms).open()`；
  - read/write/flush/clear；
  - bounded 1024-byte lines；
  - strict decoders for INPUT/NET/LOG/PING/REC/ASR。
- `network.rs` / `device_settings.rs`
  - `VKEY_CONFIG/1`；
  - network DTO and device config validation/storage。
- `profile.rs`
  - version 1、revision SHA-256、stale check、backup、atomic-ish temp rename；
  - target + three mappings validation。
- `target.rs`
  - exact foreground process name/path capture/evaluation。
- `windows_foreground_restore.rs`
  - enumerate visible unowned non-tool windows；
  - exact executable path match；
  - restore minimized；
  - bounded `AttachThreadInput` fallback；
  - `SetForegroundWindow` then verify within 250 ms。
- `runtime.rs`
  - STOPPED/DRY_RUN/LIVE；
  - dry-run no dispatcher；
  - live target restore + recheck + modifier guard + `SendInput`；
  - serial failure/stop fail closed；
  - network config/status sharing same source。

### Protocol authority

- `protocol/input-event-v1.md`: `VKEY_INPUT/1` and fixed input IDs。
- `protocol/device-link-v1.md`: CONFIG/NET/LOG/PING/REC/ASR fields and limits。
- `AI_HANDOFF.md`: explicitly states Companion contracts, current implementation status, and the prior gap “focused-window insertion of VKEY_ASR/1 text”。

## 2. Local Evidence — Existing FyAgent Shurufa

- `src-tauri/src/commands/shurufa.rs`
  - persisted debug prompt；
  - Agent config load/save；
  - context DB summaries；
  - streaming `complete_turn()`；
  - `ShurufaEvent` stream；
  - `enigo.text()` typer thread；
  - global shortcut；
  - single-flight guard。
- `src-tauri/shurufacli/**`
  - existing Agent implementation and system prompt；
  - no need for another LLM client。
- `src/v2/pages/shurufa/Page.tsx`
  - current manual textarea、preview、output、Agent config；
  - correct place to absorb Companion UI。
- `src/v2/shared/features/ports.ts` + `shared/platform/tauri/feature-ports/shurufa.ts`
  - existing typed V2 feature boundary；page should continue through this owner。
- `src/v2/shared/ui/primitives.tsx` / `Collapsible.tsx`
  - existing components can replace FY1111 UI implementation。

## 3. External Research

### serialport-rs

Official project/docs confirm `serialport-rs` provides cross-platform blocking I/O, port enumeration, builder timeout/open, read/write and RAII close. Version 4.8.1 is the same version FY1111 already uses. The 4.8 series moved Windows FFI to `windows-sys`, which aligns with FyAgent's existing dependency stack.

Decision: keep `serialport = 4.8.1` for this Demo. Do not change serial stack during migration.

### Tauri communication model

Tauri 2 official docs describe Commands as the typed frontend→Rust primitive and managed State as the native state holder. Rust→frontend Events are intended for small streamed data; Channels are the optimized mechanism for high-throughput stream data.

Decision: serial bytes remain entirely native. UI gets typed snapshots/small state events only. There is no reason to channel raw UART chunks into React.

### Tauri serial plugins

Maintained community plugins exist for Tauri 2 and can list/open/read/write ports, including background watch APIs. They also introduce plugin permissions and a JS serial API surface, and one reviewed plugin documents Windows enumeration behavior that may depend on supplemental `wmic` metadata in some builds.

Decision: reject plugin adoption for this task. The capability already exists and is protocol-specialized in FY1111; adding a plugin would duplicate the owner and widen renderer access.

### enigo

Enigo 0.3 documents `Enigo::text()` and Windows support. FyAgent already depends on it and already has a streaming typer implementation.

Decision: reuse the current shurufa typer; no `SendInput` text implementation, clipboard paste path or new input package is needed for ASR output.

## 4. Self-review Round 1 — Feature Completeness

Initial mistake: planning was narrowed to “serial ASR bridge only”.

Review method: compare every `CompanionHost` method, `App.tsx` action, `SettingsPanel` status field and native module against the proposed task.

Findings:

- serial selection alone was insufficient；
- profile/revision/target/mappings/DryRun/Live/Stop were missing；
- device Wi-Fi/SiliconFlow configuration and all network/REC status were missing；
- Windows foreground restore and shortcut dispatcher were missing。

Correction: PRD R1 now enumerates the complete user-visible Companion surface and acceptance includes both shortcut and ASR paths.

## 5. Self-review Round 2 — Architecture / Reuse

Review method: compare FyAgent backend reuse/modular-boundary specs, existing dependencies, FY1111 modules, and community serial options.

Findings:

- copying FY1111 `lib.rs::commands` into `commands/shurufa.rs` would create an oversized mixed transport/domain owner；
- adding a Tauri serial plugin would duplicate `serialport` and leak a broad serial API into renderer；
- rewriting FY1111 Win32 code onto another Windows binding solely for style would slow Demo and raise regression risk；
- copying FY1111 React/CSS would create a second design system。

Correction:

- private Companion service/facade + thin commands；
- pin existing FY1111 `serialport` version；
- reuse existing FyAgent `windows-sys`, `enigo`, `sha2`；
- reuse V2 primitives and only move UI layout/semantics。

## 6. Self-review Round 3 — Data Flow / Concurrency

Review method: trace source-to-sink for both `VKEY_INPUT/1` and `VKEY_ASR/1`, including what happens after the user leaves the FyAgent window.

Critical finding: FY1111 serial progress is currently driven by React timers invoking poll commands. If copied literally, automatic input method behavior can stop when `/shurufa` was never visited or its effects are not active. Multiple UI/native consumers could also compete for the same COM stream.

Second finding: `VKEY_ASR/1` currently parses `seq` but discards sequence identity in `NetworkStatus`; comparing text strings cannot safely dedupe repeated speech.

Correction:

- native service becomes sole serial reader owner；
- frontend polls/subscribes snapshot only；
- ASR sequence is retained/tracked native-side；
- Agent bridge happens native-side, outside serial locks；
- same text with a new seq is a new turn, duplicate/backward seq is dropped。

## 7. Self-review Round 4 — Demo Scope / UX / Verification

Review method: remove every item not required to demonstrate the requested experience, while checking that removing it does not delete FY1111 functionality.

Findings:

- macOS parity would force new foreground/window behavior with no immediate demo value；
- async serial rewrite, ASR queue, SecretRef migration, CI and production IME would materially expand scope；
- deleting manual textarea/global shortcut immediately would remove useful no-hardware debug paths without helping the requested hardware flow；
- device ASR key/model and desktop Agent key/model are two distinct systems and could be confused in a merged UI。

Correction:

- Windows-only；
- blocking `serialport` retained；
- Agent remains single-flight, no queue；
- debug manual input remains secondary only；
- UI explicitly separates “设备转写配置” and “输入法 Agent 配置”；
- validation uses focused unit/type tests plus one real Windows manual end-to-end demo, no new CI。

## 8. Final Pre-write Decision Matrix

| Decision | Chosen | Rejected / deferred |
| --- | --- | --- |
| Companion scope | all current FY1111 user features | ASR-only subset |
| Platform | Windows-only functional target | macOS parity now |
| Serial crate | FY1111 `serialport 4.8.1` | Web Serial / Tauri serial plugin / tokio-serial rewrite |
| Serial owner | native Companion service | React polling as reader |
| Agent | existing shurufacli core | second Agent/LLM client |
| Text injection | existing enigo stream | new clipboard/IME/text SendInput path |
| Shortcut dispatch | FY1111 target/restore/SendInput | redesign |
| UI | FY1111 layout semantics + FyAgent V2 components | copy FY1111 components/CSS/app shell |
| ASR dedupe | protocol seq | compare text strings |
| Concurrent ASR | single-flight fail-fast | queue/backpressure system |
| Testing | focused + Windows manual Demo | CI expansion / exhaustive E2E |

### Exact FY1111 command completeness check

Final review compared `companion/src-tauri/src/main.rs` directly against this task. All 12 commands are preserved semantically:

`list_ports`, `capture_target_after_delay`, `load_profile`, `save_profile`, `start_dry_run`, `enable_live_for_run`, `poll_runtime_event`, `stop_runtime`, `load_device_settings`, `save_device_settings`, `apply_device_config`, `poll_network_status`.

The only deliberate shape change is that the two `poll_*` capabilities become snapshot reads because native pump is the sole serial reader. This removes a lifecycle bug without deleting their status projections.

The final review also corrected Stop semantics: healthy serial connectivity may remain active in `STOPPED` so device/network/REC/ASR continues; only serial failure closes the source. Shortcut live permission is still always cleared on Stop.

## 9. Remaining Implementation-level Flexibility

These details may be adjusted by the executor without changing the plan:

- exact Rust service folder/file names；
- whether status projection uses low-frequency snapshot polling or a small Tauri event；
- exact TS nesting of Companion methods under the shurufa FeaturePort；
- precise page CSS grid arrangement and responsive breakpoint；
- native pump as Tauri async task vs controlled thread。

These may not be changed without returning to planning:

- dropping any FY1111 user feature；
- moving serial parsing into React；
- replacing existing shurufa Agent/typing engine；
- adding macOS/CI/release scope；
- changing `VKEY_* /1` protocol semantics incompatibly；
- adding a new serial framework/dependency instead of the reviewed direct `serialport` path。
