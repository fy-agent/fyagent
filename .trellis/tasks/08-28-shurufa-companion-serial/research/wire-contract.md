# Locked Wire Contract — do not invent alternate names

This file is the parallel-implementation contract. Backend, Agent-bridge, and
frontend agents must follow it exactly.

## File ownership (avoid merge conflicts)

| Agent | May edit | Must not edit |
| --- | --- | --- |
| Native Companion | `src-tauri/src/services/shurufa_companion/**`, `src-tauri/src/commands/shurufa_companion.rs`, `src-tauri/src/services/mod.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` (handler + `.manage` only), `src-tauri/Cargo.toml` | `src-tauri/src/commands/shurufa.rs`, `src/v2/**`, `tests/**` |
| Agent ingest | `src-tauri/src/commands/shurufa.rs` only | everything else |
| Frontend | `src/v2/**`, `tests/v2/platform/featurePorts.test.ts`, `tests/v2/pages/shurufa/**` | `src-tauri/**` |

## Tauri commands

Register next to the existing `shurufa_*` commands.

```text
shurufa_companion_list_ports() -> Result<Vec<String>, String>
shurufa_companion_capture_target() -> Result<CompanionTarget, String>
shurufa_companion_get_snapshot() -> Result<CompanionSnapshot, String>
shurufa_companion_save_profile(draft: CompanionProfile) -> Result<CompanionProfile, String>
shurufa_companion_start_dry_run() -> Result<CompanionRuntime, String>
shurufa_companion_enable_live() -> Result<CompanionRuntime, String>
shurufa_companion_stop() -> Result<CompanionRuntime, String>
shurufa_companion_save_device_settings(draft: CompanionDeviceSettings) -> Result<CompanionDeviceSettings, String>
shurufa_companion_apply_device_config(request: CompanionApplyDeviceConfig) -> Result<CompanionNetwork, String>
```

`load_profile` / `load_device_settings` / `poll_runtime_event` / `poll_network_status`
are folded into `shurufa_companion_get_snapshot`. The snapshot command is
read-only and must never call serial `read()`.

## Rust Agent API (owned by Agent ingest)

```rust
pub async fn run_ingest_text<R: Runtime>(
    app: AppHandle<R>,
    text: String,
    type_into_focus: bool,
) -> Result<String, String>;

pub async fn run_ingest<R: Runtime>(
    app: AppHandle<R>,
    type_into_focus: bool,
) -> Result<String, String> {
    run_ingest_text(app, current_prompt(), type_into_focus).await
}
```

Busy error text stays exactly: `正在生成中，请稍后再试`.

Companion pump, after dropping all Companion/runtime/serial locks:

```rust
let handle = app.clone();
tauri::async_runtime::spawn(async move {
    if let Err(message) = crate::commands::shurufa::run_ingest_text(handle, text, true).await {
        log::warn!("shurufa companion asr ingest failed: {message}");
    }
});
```

## CamelCase DTOs

```ts
type CompanionInputId = "ENCODER_CW" | "ENCODER_CCW" | "ENCODER_PRESS";
type CompanionRuntimeState = "STOPPED" | "DRY_RUN" | "LIVE";
type CompanionNetworkState =
  | "UNKNOWN"
  | "DISCONNECTED"
  | "CONNECTING"
  | "CONNECTED"
  | "FAILED";
type CompanionAsrAdmission =
  | "none"
  | "start"
  | "fail"
  | "empty"
  | "admitted"
  | "duplicate"
  | "busy";

interface CompanionTarget {
  processName: string;
  processPath: string;
}

interface CompanionMapping {
  input: CompanionInputId;
  displayName: string;
  keys: string[];
}

interface CompanionProfile {
  version: 1;
  revision: string | null;
  serial: { port: string; baud: number };
  target: CompanionTarget | null;
  mappings: CompanionMapping[];
}

interface CompanionDeviceSettings {
  version: 1;
  ssid: string;
  password: string;
  apiKey: string;
  model: string;
}

interface CompanionApplyDeviceConfig {
  port: string;
  baud: number;
  settings: CompanionDeviceSettings;
}

interface CompanionNetwork {
  state: CompanionNetworkState;
  ssid: string;
  ip: string;
  rssi: number | null;
  reason: string | null;
  pingHost: string | null;
  pingOk: boolean | null;
  pingMs: number | null;
  pingLost: number | null;
  pingSent: number | null;
  lastLog: string | null;
  beats: number | null;
  recState: string | null;
  recMs: number | null;
  recSamples: number | null;
  recRms: number | null;
  recPeak: number | null;
  recSilence: boolean | null;
  recReason: string | null;
  asrState: string | null;
  asrText: string | null;
  asrReason: string | null;
}

interface CompanionRuntime {
  state: CompanionRuntimeState;
  liveEnabled: boolean;
  lastEvent: string;
  gapMissed: number | null;
  network: CompanionNetwork;
}

interface CompanionSnapshot {
  ports: string[];
  profile: CompanionProfile | null;
  device: CompanionDeviceSettings;
  runtime: CompanionRuntime;
  lastAsrSeq: number | null;
  lastAsrAdmission: CompanionAsrAdmission;
  lastAsrError: string | null;
}
```

Default device model: `XingChenAGI/XingChenASR-V3.2-Ultra`.
Default baud: `115200`.
Persistence: `<app-config>/shurufacli/companion/profile.json` and `device.json`.

## ShurufaPort additions

Keep existing Agent methods. Add Companion methods on the same port:

```ts
listCompanionPorts(): Promise<string[]>;
captureCompanionTarget(): Promise<CompanionTarget>;
getCompanionSnapshot(): Promise<CompanionSnapshot>;
saveCompanionProfile(draft: CompanionProfile): Promise<CompanionProfile>;
startCompanionDryRun(): Promise<CompanionRuntime>;
enableCompanionLive(): Promise<CompanionRuntime>;
stopCompanion(): Promise<CompanionRuntime>;
saveCompanionDeviceSettings(draft: CompanionDeviceSettings): Promise<CompanionDeviceSettings>;
applyCompanionDeviceConfig(request: CompanionApplyDeviceConfig): Promise<CompanionNetwork>;
```

Browser adapter: every new method uses existing `rejectNativeOnly`.
Tauri adapter: parse unknown payloads once; page never casts wire data.
