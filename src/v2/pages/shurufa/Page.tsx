import { CaretDownIcon } from "@phosphor-icons/react/dist/csr/CaretDown";
import { useEffect, useMemo, useRef, useState } from "react";

import { errorMessage, isNativeOnlyError } from "../../shared/features/helpers";
import { useFeatures } from "../../shared/features/provider";
import type {
  CompanionDeviceSettings,
  CompanionMapping,
  CompanionNetwork,
  CompanionProfile,
  CompanionRuntime,
  CompanionSnapshot,
  ShurufaConfig,
} from "../../shared/features/ports";
import { CopyablePath } from "../../shared/ui/CopyablePath";
import {
  Collapsible,
  CollapsibleCaret,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../../shared/ui/Collapsible";
import {
  Badge,
  Button,
  InlineNotice,
  Input,
  SecretInput,
  Spinner,
} from "../../shared/ui/primitives";

import { ChordField } from "./ChordField";
import {
  asrAdmissionLabel,
  asrHeadline,
  canonicalChord,
  CLOUD_MODELS,
  CLOUD_OPTIONAL_HINT,
  DEFAULT_DEVICE_SETTINGS,
  deviceSettingsError,
  EMPTY_PROFILE,
  EMPTY_RUNTIME,
  formatRuntimeText,
  hydrateProfile,
  INPUT_LABELS,
  mappingErrors,
  MIC_REC_HINT,
  networkChipLabel,
  networkReasonLabel,
  recReasonLabel,
  recStateLabel,
  RUNTIME_STATE_LABELS,
  SENSOR_HINT,
  ssidLooksFiveG,
  USB_LINK_BAUD,
  USB_LINK_ID,
  WIFI_BAND_HINT,
  WIFI_CONNECTING_STUCK_HINT,
  WIFI_FIVE_G_ALERT,
} from "./companion";

import "./page.css";

type ConfigDraft = Omit<ShurufaConfig, "configured">;

const EMPTY_CONFIG: ConfigDraft = {
  url: "https://api.openai.com/v1",
  model: "gpt-4o-mini",
  apiKey: "",
  maxSummaries: 8,
  timeoutSecs: 60,
};

const COMPANION_POLL_MS = 400;

function runtimeBadgeTone(
  state: CompanionRuntime["state"],
): "neutral" | "accent" | "warning" {
  if (state === "LIVE") return "accent";
  if (state === "DRY_RUN") return "warning";
  return "neutral";
}

function networkBadgeTone(
  state: CompanionNetwork["state"],
): "neutral" | "accent" | "warning" {
  if (state === "CONNECTED") return "accent";
  if (state === "FAILED" || state === "CONNECTING") return "warning";
  return "neutral";
}

export function ShurufaPage() {
  const { ports, notify } = useFeatures();
  const [prompt, setPrompt] = useState("");
  const [config, setConfig] = useState<ConfigDraft>(EMPTY_CONFIG);
  const [configured, setConfigured] = useState(false);
  const [running, setRunning] = useState(false);
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [shortcutLabel, setShortcutLabel] = useState("⌘M");
  const [dataDir, setDataDir] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const outputRef = useRef("");

  const [linkPorts, setLinkPorts] = useState<string[]>([]);
  const [draft, setDraft] = useState<CompanionProfile>(EMPTY_PROFILE);
  const [device, setDevice] = useState<CompanionDeviceSettings>(
    DEFAULT_DEVICE_SETTINGS,
  );
  const [runtime, setRuntime] = useState<CompanionRuntime>(EMPTY_RUNTIME);
  const [lastAsrAdmission, setLastAsrAdmission] =
    useState<CompanionSnapshot["lastAsrAdmission"]>("none");
  const [lastAsrError, setLastAsrError] = useState<string | null>(null);
  const [companionReady, setCompanionReady] = useState(false);
  const [deviceOpen, setDeviceOpen] = useState(false);
  const [agentOpen, setAgentOpen] = useState(false);
  const [debugOpen, setDebugOpen] = useState(false);
  const [notice, setNotice] = useState("插入 Board C 即可连接，无需选择串口。");
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const nativeUnavailable = isNativeOnlyError(error);

  const errors = useMemo(() => mappingErrors(draft.mappings), [draft.mappings]);
  const deviceError = useMemo(() => deviceSettingsError(device), [device]);
  const fiveG = ssidLooksFiveG(device.ssid);
  const stopped = runtime.state === "STOPPED";
  const editable = companionReady && stopped && !busy && !nativeUnavailable;
  const usbInserted = linkPorts.includes(USB_LINK_ID);
  const canSave = editable && errors.size === 0 && draft.target !== null;
  const canStart = canSave && !dirty && draft.revision !== null;
  const canApply =
    companionReady &&
    !busy &&
    !nativeUnavailable &&
    usbInserted &&
    deviceError === null;
  const networkChip = networkChipLabel(
    runtime.network.state,
    runtime.network.ip,
    runtime.network.reason,
    fiveG,
  );
  const recLabel = recStateLabel(runtime.network.recState);
  const recReason = recReasonLabel(runtime.network.recReason);
  const reasonLabel = networkReasonLabel(runtime.network.reason);
  const admissionLabel = asrAdmissionLabel(lastAsrAdmission);
  const voiceHeadline = asrHeadline(
    runtime.network.asrState,
    runtime.network.asrReason,
    runtime.network.recState,
  );
  const sensorWarn =
    runtime.network.sensorState === "TOF"
      ? "TOF 未就绪"
      : runtime.network.sensorState === "I2C"
        ? "I2C 未就绪"
        : null;

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const snapshot = await ports.shurufa.getSnapshot();
        if (cancelled) return;
        setPrompt(snapshot.prompt);
        setConfig({
          url: snapshot.config.url,
          model: snapshot.config.model,
          apiKey: snapshot.config.apiKey,
          maxSummaries: snapshot.config.maxSummaries,
          timeoutSecs: snapshot.config.timeoutSecs,
        });
        setConfigured(snapshot.config.configured);
        setRunning(snapshot.running);
        outputRef.current = snapshot.lastOutput;
        setOutput(snapshot.lastOutput);
        setError(snapshot.lastError);
        setShortcutLabel(snapshot.shortcutLabel);
        setDataDir(snapshot.dataDir);
      } catch (cause) {
        if (!cancelled) {
          setError(errorMessage(cause));
          setLoading(false);
        }
        return;
      }
      try {
        const companion = await ports.shurufa.getCompanionSnapshot();
        if (cancelled) return;
        setLinkPorts(companion.ports);
        setRuntime(companion.runtime);
        setLastAsrAdmission(companion.lastAsrAdmission);
        setLastAsrError(companion.lastAsrError);
        setDraft(hydrateProfile(companion.profile));
        setDevice(companion.device);
        setDirty(false);
        setCompanionReady(true);
        setNotice(
          companion.profile
            ? "已恢复保存的配置；实时权限保持关闭。"
            : "插入 Board C 即可连接，无需选择串口。",
        );
      } catch (cause) {
        if (cancelled) return;
        if (isNativeOnlyError(cause)) setError(errorMessage(cause));
        else setNotice("Companion 状态不可用。");
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [ports.shurufa]);

  useEffect(() => {
    let cancelled = false;
    let unlisten = () => {};
    void ports.shurufa
      .subscribe((event) => {
        if (cancelled) return;
        if (event.type === "started") {
          outputRef.current = "";
          setRunning(true);
          setOutput("");
          setError(null);
          return;
        }
        if (event.type === "delta") {
          outputRef.current += event.text;
          setOutput(outputRef.current);
          return;
        }
        if (event.type === "finished") {
          outputRef.current = event.output;
          setRunning(false);
          setOutput(event.output);
          return;
        }
        setRunning(false);
        setError(event.message);
      })
      .then((stop) => {
        if (cancelled) {
          stop();
          return;
        }
        unlisten = stop;
      })
      .catch((cause) => {
        setError(errorMessage(cause));
      });
    return () => {
      cancelled = true;
      unlisten();
    };
  }, [ports.shurufa]);

  useEffect(() => {
    if (!companionReady || nativeUnavailable) return undefined;
    let cancelled = false;
    let inFlight = false;
    const poll = async () => {
      if (inFlight || cancelled) return;
      inFlight = true;
      try {
        const snapshot = await ports.shurufa.getCompanionSnapshot();
        if (!cancelled) {
          setLinkPorts(snapshot.ports);
          setRuntime(snapshot.runtime);
          setLastAsrAdmission(snapshot.lastAsrAdmission);
          setLastAsrError(snapshot.lastAsrError);
        }
      } catch {
        if (!cancelled) setNotice("状态刷新失败。");
      } finally {
        inFlight = false;
      }
    };
    void poll();
    const timer = window.setInterval(() => {
      void poll();
    }, COMPANION_POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [companionReady, nativeUnavailable, ports.shurufa]);

  useEffect(() => {
    if (runtime.network.state !== "CONNECTING") return undefined;
    const timer = window.setTimeout(() => {
      setNotice(WIFI_CONNECTING_STUCK_HINT);
    }, 8000);
    return () => window.clearTimeout(timer);
  }, [runtime.network.state, runtime.network.ssid]);

  const updatePrompt = (text: string) => {
    setPrompt(text);
    void ports.shurufa.setPrompt(text).catch((cause) => {
      setError(errorMessage(cause));
    });
  };

  const saveConfig = async () => {
    setSaving(true);
    try {
      const next = await ports.shurufa.saveConfig(config);
      setConfig({
        url: next.url,
        model: next.model,
        apiKey: next.apiKey,
        maxSummaries: next.maxSummaries,
        timeoutSecs: next.timeoutSecs,
      });
      setConfigured(next.configured);
      setError(null);
      notify({
        tone: next.configured ? "success" : "info",
        title: next.configured ? "配置已保存" : "配置已写入，但仍不完整",
        description: next.configured
          ? "硬件 ASR 会使用这份桌面 Agent 配置"
          : "请检查 url、model 和 api_key 是否都已填好",
      });
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setSaving(false);
    }
  };

  const runPreview = async () => {
    setError(null);
    try {
      await ports.shurufa.run();
    } catch (cause) {
      setError(errorMessage(cause));
    }
  };

  const clearSession = async () => {
    try {
      const deleted = await ports.shurufa.clearSession();
      notify({
        tone: "success",
        title: "已清空会话摘要",
        description: `删除了 ${deleted} 条历史`,
      });
    } catch (cause) {
      setError(errorMessage(cause));
    }
  };

  const captureTarget = async () => {
    if (!editable) return;
    setBusy(true);
    setNotice("将在 3 秒后捕获前台目标，请立即切换到目标应用。");
    try {
      const target = await ports.shurufa.captureCompanionTarget();
      setDraft((current) => ({ ...current, target }));
      setDirty(true);
      setNotice(
        "已捕获前台目标。实时模式下，旋钮或 GPIO8 会先唤回该窗口，再发送快捷键。",
      );
    } catch {
      setNotice("捕获目标失败，已保留之前的目标。");
    } finally {
      setBusy(false);
    }
  };

  const saveProfile = async () => {
    if (!canSave) return;
    setBusy(true);
    try {
      setDraft(hydrateProfile(await ports.shurufa.saveCompanionProfile(draft)));
      setDirty(false);
      setNotice("配置已保存并生成新修订版本。");
    } catch {
      setNotice("保存配置失败；解决过期修订版本前请重新加载。");
    } finally {
      setBusy(false);
    }
  };

  const updateMapping = (index: number, patch: Partial<CompanionMapping>) => {
    setDraft((current) => ({
      ...current,
      mappings: current.mappings.map((mapping, currentIndex) =>
        currentIndex === index ? { ...mapping, ...patch } : mapping,
      ),
    }));
    setDirty(true);
    if (patch.keys) setNotice("快捷键已更新；保存配置后才会用于派发。");
  };

  const startDryRun = async () => {
    if (!canStart) return;
    setBusy(true);
    try {
      setRuntime(await ports.shurufa.startCompanionDryRun());
      setNotice("演练模式已启动，未创建输入派发器。");
    } catch {
      setNotice("无法启动演练模式，未启用实时输入。");
    } finally {
      setBusy(false);
    }
  };

  const startLive = async () => {
    if (!canStart) return;
    setBusy(true);
    try {
      setRuntime(await ports.shurufa.enableCompanionLive());
      setNotice(
        "仅为当前进程启用了实时权限：旋钮或 GPIO8 会先唤回已捕获窗口，再发送快捷键。",
      );
    } catch {
      setNotice("无法启动实时模式；实时权限保持关闭。");
    } finally {
      setBusy(false);
    }
  };

  const applyDevice = async () => {
    if (!canApply) return;
    setBusy(true);
    try {
      const settings = {
        ...device,
        apiKey: device.apiKey.trim(),
        model: device.model.trim() || DEFAULT_DEVICE_SETTINGS.model,
      };
      const network = await ports.shurufa.applyCompanionDeviceConfig({
        port: USB_LINK_ID,
        baud: USB_LINK_BAUD,
        settings,
      });
      setDevice(settings);
      setRuntime((current) => ({ ...current, network }));
      if (fiveG || network.reason === "BAND") {
        setNotice(WIFI_FIVE_G_ALERT);
      } else if (network.state === "CONNECTED") {
        setNotice("设备已联网。");
      } else if (network.state === "FAILED") {
        setNotice("联网失败。开发板只支持 2.4GHz，请改用 2.4G 名称后重试。");
      } else {
        setNotice("已下发 Wi-Fi，等待设备联网。开发板只支持 2.4GHz。");
      }
    } catch {
      setNotice("下发联网配置失败，未在界面显示密钥。");
    } finally {
      setBusy(false);
    }
  };

  const saveDevice = async () => {
    if (!companionReady || busy || nativeUnavailable || deviceError) return;
    setBusy(true);
    try {
      const next = await ports.shurufa.saveCompanionDeviceSettings({
        ...device,
        apiKey: device.apiKey.trim(),
        model: device.model.trim() || DEFAULT_DEVICE_SETTINGS.model,
      });
      setDevice(next);
      setNotice("设备转写配置已保存，尚未下发到硬件。");
    } catch {
      setNotice("保存设备转写配置失败，未在界面显示密钥。");
    } finally {
      setBusy(false);
    }
  };

  const stopRuntime = async () => {
    if (busy) return;
    setBusy(true);
    try {
      setRuntime(await ports.shurufa.stopCompanion());
      setNotice(
        "运行已停止，实时权限已清除。健康 USB 连接仍可继续接收网络/录音/转写。",
      );
    } catch {
      setRuntime((current) => ({
        ...current,
        lastEvent: "无法确认运行已停止。",
      }));
      setNotice("无法确认运行已停止；再次启用实时模式前请重启应用。");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="fy-feature-page fy-shurufa-page" data-testid="shurufa-page">
      <header className="fy-feature-header">
        <div>
          <h1 className="fy-shurufa-page-title">输入法</h1>
          <p className="fy-feature-description">
            正式演示输入来自硬件 ASR：Board C 录音后，native 会把转写交给输入法
            Agent，并流式写入当前焦点文本框。手工文本和 {shortcutLabel}{" "}
            只是无硬件时的调试后门。
          </p>
        </div>
        <div className="fy-feature-actions fy-shurufa-header-meta">
          <Badge tone={runtimeBadgeTone(runtime.state)}>
            {RUNTIME_STATE_LABELS[runtime.state]}
          </Badge>
          <Badge tone={networkBadgeTone(runtime.network.state)}>
            {networkChip}
          </Badge>
          <Button onClick={() => void clearSession()} disabled={running}>
            清空会话
          </Button>
        </div>
      </header>

      {nativeUnavailable ? (
        <InlineNotice tone="warning">
          输入法 Demo 只在 FyAgent
          桌面应用中可用。浏览器预览不会探测 Board C USB 或伪造硬件状态。
        </InlineNotice>
      ) : null}
      {error && !nativeUnavailable ? (
        <InlineNotice tone="error">{error}</InlineNotice>
      ) : null}

      {loading ? (
        <div className="fy-shurufa-loading">
          <Spinner label="加载输入法配置" />
        </div>
      ) : (
        <div className="fy-shurufa-workspace">
          <section
            className="fy-shurufa-panel fy-shurufa-usb"
            aria-label="设备连接"
            data-testid="companion-usb"
          >
            <div className="fy-shurufa-usb-copy">
              <strong>Board C USB</strong>
              <p>插入即可连接，无需选择串口</p>
            </div>
            <Badge tone={usbInserted ? "accent" : "neutral"}>
              {usbInserted ? "已插入" : "未插入"}
            </Badge>
            <Badge>波特率 {USB_LINK_BAUD}</Badge>
            <Badge tone={runtimeBadgeTone(runtime.state)}>
              {RUNTIME_STATE_LABELS[runtime.state]}
            </Badge>
          </section>

          <Collapsible
            open={deviceOpen}
            onOpenChange={setDeviceOpen}
            className="fy-shurufa-panel"
          >
            <CollapsibleTrigger asChild>
              <button
                type="button"
                className="fy-shurufa-collapse-trigger"
                data-testid="companion-device-toggle"
              >
                <span>
                  <strong>设备转写配置</strong>
                  <span className="fy-shurufa-collapse-hint">
                    SiliconFlow 下发到硬件，与桌面 Agent 无关
                  </span>
                </span>
                <span className="fy-shurufa-collapse-meta">
                  <Badge tone={networkBadgeTone(runtime.network.state)}>
                    {networkChip}
                  </Badge>
                  <CollapsibleCaret open={deviceOpen}>
                    <CaretDownIcon size={16} weight="bold" />
                  </CollapsibleCaret>
                </span>
              </button>
            </CollapsibleTrigger>
            <CollapsibleContent open={deviceOpen}>
              <div
                className="fy-shurufa-collapse-body"
                data-testid="companion-device-settings"
              >
                <p className="fy-shurufa-hint">{WIFI_BAND_HINT}</p>
                <div className="fy-shurufa-config-grid">
                  <label className="fy-control-field">
                    Wi-Fi 名称
                    <Input
                      value={device.ssid}
                      disabled={busy || nativeUnavailable}
                      autoComplete="off"
                      onChange={(event) =>
                        setDevice((current) => ({
                          ...current,
                          ssid: event.target.value,
                        }))
                      }
                    />
                  </label>
                  <label className="fy-control-field">
                    Wi-Fi 密码
                    <SecretInput
                      value={device.password}
                      disabled={busy || nativeUnavailable}
                      autoComplete="new-password"
                      onChange={(event) =>
                        setDevice((current) => ({
                          ...current,
                          password: event.target.value,
                        }))
                      }
                    />
                  </label>
                  <label className="fy-control-field fy-shurufa-config-span">
                    SiliconFlow API Key
                    <SecretInput
                      value={device.apiKey}
                      disabled={busy || nativeUnavailable}
                      autoComplete="off"
                      onChange={(event) =>
                        setDevice((current) => ({
                          ...current,
                          apiKey: event.target.value,
                        }))
                      }
                    />
                  </label>
                  <label className="fy-control-field fy-shurufa-config-span">
                    转写模型
                    <Input
                      list="companion-cloud-models"
                      value={device.model}
                      disabled={busy || nativeUnavailable}
                      autoComplete="off"
                      placeholder="仅 Wi-Fi 可留空，也可填写其他模型"
                      onChange={(event) =>
                        setDevice((current) => ({
                          ...current,
                          model: event.target.value,
                        }))
                      }
                    />
                    <datalist id="companion-cloud-models">
                      {CLOUD_MODELS.map((model) => (
                        <option key={model} value={model} />
                      ))}
                    </datalist>
                  </label>
                </div>
                <p className="fy-shurufa-hint">{CLOUD_OPTIONAL_HINT}</p>
                <p className="fy-shurufa-hint">{MIC_REC_HINT}</p>
                <p className="fy-shurufa-hint">{SENSOR_HINT}</p>
                <div className="fy-shurufa-inline-actions">
                  <Button
                    disabled={
                      !companionReady ||
                      busy ||
                      nativeUnavailable ||
                      deviceError !== null
                    }
                    onClick={() => void saveDevice()}
                  >
                    保存设备设置
                  </Button>
                  <Button
                    className="fy-control-button-primary"
                    disabled={!canApply}
                    onClick={() => void applyDevice()}
                  >
                    保存并下发联网
                  </Button>
                </div>
                {fiveG ? (
                  <InlineNotice tone="warning">
                    {WIFI_FIVE_G_ALERT}
                  </InlineNotice>
                ) : null}
                {deviceOpen && deviceError ? (
                  <p className="fy-control-field-error" role="alert">
                    {deviceError}
                  </p>
                ) : null}
              </div>
            </CollapsibleContent>
          </Collapsible>

          <section
            className="fy-shurufa-panel fy-shurufa-status"
            aria-label="网络与转写状态"
            data-testid="companion-status"
          >
            {reasonLabel ? (
              <InlineNotice tone="warning">{reasonLabel}</InlineNotice>
            ) : null}
            <dl className="fy-shurufa-status-grid">
              <div>
                <dt>网络</dt>
                <dd>{networkChip}</dd>
              </div>
              <div>
                <dt>当前 SSID</dt>
                <dd>
                  {runtime.network.ssid || "—"}
                  {runtime.network.rssi != null
                    ? ` · ${runtime.network.rssi} dBm`
                    : ""}
                  {runtime.network.beats != null
                    ? ` · 心跳 ${runtime.network.beats}`
                    : ""}
                </dd>
              </div>
              <div>
                <dt>连通探测</dt>
                <dd>
                  {runtime.network.pingHost
                    ? `${runtime.network.pingHost}${
                        runtime.network.pingOk
                          ? ` 成功 ${runtime.network.pingMs ?? "-"} ms`
                          : " 失败"
                      }${
                        runtime.network.pingSent != null
                          ? ` · 丢 ${runtime.network.pingLost ?? 0}/${runtime.network.pingSent}`
                          : ""
                      }`
                    : "—"}
                </dd>
              </div>
              <div>
                <dt>录音</dt>
                <dd>
                  {recLabel
                    ? `${recLabel}${
                        runtime.network.recMs != null
                          ? ` · ${runtime.network.recMs} ms`
                          : ""
                      }${
                        runtime.network.recSamples != null
                          ? ` · ${runtime.network.recSamples} 采样`
                          : ""
                      }${
                        runtime.network.recRms != null
                          ? ` · RMS ${runtime.network.recRms}`
                          : ""
                      }${
                        runtime.network.recPeak != null
                          ? ` · 峰 ${runtime.network.recPeak}`
                          : ""
                      }${runtime.network.recSilence ? " · 静音" : ""}${
                        runtime.network.recReason
                          ? ` · ${recReason ?? runtime.network.recReason}`
                          : ""
                      }`
                    : "—"}
                </dd>
              </div>
              <div>
                <dt>转写</dt>
                <dd>{voiceHeadline}</dd>
              </div>
              <div>
                <dt>座位</dt>
                <dd>
                  {runtime.network.pir == null
                    ? "—"
                    : runtime.network.pir
                      ? "有人"
                      : "无人"}
                </dd>
              </div>
              <div>
                <dt>激光测距</dt>
                <dd>
                  {runtime.network.tofMm != null
                    ? `${runtime.network.tofMm} mm`
                    : "—"}
                </dd>
              </div>
              <div>
                <dt>传感器</dt>
                <dd>{sensorWarn ?? runtime.network.sensorState ?? "—"}</dd>
              </div>
              <div>
                <dt>设备日志</dt>
                <dd>{runtime.network.lastLog || "—"}</dd>
              </div>
            </dl>
          </section>

          <section
            className="fy-shurufa-panel fy-shurufa-target"
            aria-label="前台目标"
          >
            <div>
              <strong>前台目标</strong>
              <p>
                {draft.target
                  ? `${draft.target.processName} · ${draft.target.processPath}`
                  : "尚未捕获。请先切到目标应用，再点 3 秒后捕获。"}
              </p>
            </div>
            <Button disabled={!editable} onClick={() => void captureTarget()}>
              3 秒后捕获
            </Button>
          </section>

          <section
            className="fy-shurufa-panel fy-shurufa-mappings"
            aria-label="固定输入映射"
          >
            {draft.mappings.map((mapping, index) => (
              <div className="fy-shurufa-mapping" key={mapping.input}>
                <strong>{INPUT_LABELS[mapping.input]}</strong>
                <Input
                  disabled={!editable}
                  aria-label={`${INPUT_LABELS[mapping.input]} 名称`}
                  value={mapping.displayName}
                  onChange={(event) =>
                    updateMapping(index, { displayName: event.target.value })
                  }
                />
                <ChordField
                  label={`${INPUT_LABELS[mapping.input]} 快捷键`}
                  keys={mapping.keys}
                  disabled={!editable}
                  onChange={(keys) => updateMapping(index, { keys })}
                />
                <span className="fy-shurufa-canonical">
                  {canonicalChord(mapping.keys) ?? "点击后按下快捷键"}
                </span>
                {errors.get(mapping.input) ? (
                  <small className="fy-control-field-error" role="alert">
                    {errors.get(mapping.input)}
                  </small>
                ) : null}
              </div>
            ))}
          </section>

          <section className="fy-shurufa-controls" aria-label="运行控制">
            <Button disabled={!canSave} onClick={() => void saveProfile()}>
              保存配置
            </Button>
            <Button disabled={!canStart} onClick={() => void startDryRun()}>
              启动演练模式
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={!canStart}
              onClick={() => void startLive()}
            >
              为本次运行启用实时模式
            </Button>
            <Button
              disabled={stopped || busy}
              onClick={() => void stopRuntime()}
            >
              停止运行
            </Button>
          </section>

          <section className="fy-shurufa-panel" aria-label="最后事件">
            <p className="fy-shurufa-event">
              <strong>最后事件：</strong> {formatRuntimeText(runtime.lastEvent)}
            </p>
            <InlineNotice
              tone={
                lastAsrAdmission === "busy" || lastAsrAdmission === "fail"
                  ? "warning"
                  : "info"
              }
            >
              {notice}
            </InlineNotice>
          </section>

          <section
            className="fy-shurufa-output"
            aria-label="语音与 Agent"
            data-testid="companion-voice"
          >
            <div className="fy-shurufa-output-head">
              <div>
                <h2>最近转写与优化结果</h2>
                <p className="fy-shurufa-hint">
                  正式链路：硬件 ASR → 输入法 Agent →
                  当前焦点文本框。页面只投影状态，不驱动 USB 读取。
                </p>
              </div>
              {running ? <Spinner label="正在生成" /> : null}
            </div>
            <dl className="fy-shurufa-status-grid">
              <div>
                <dt>录音与转写</dt>
                <dd>{voiceHeadline}</dd>
              </div>
              <div>
                <dt>最近原始 ASR</dt>
                <dd>{runtime.network.asrText || "还没有转写文本"}</dd>
              </div>
              <div>
                <dt>Agent 状态</dt>
                <dd>
                  {running
                    ? "生成中"
                    : admissionLabel ||
                      (configured ? "空闲" : "桌面 Agent 尚未配置")}
                  {lastAsrError ? ` · ${lastAsrError}` : ""}
                </dd>
              </div>
            </dl>
            <pre className="fy-shurufa-output-body">
              {output || (running ? "正在等待模型输出…" : "还没有优化结果")}
            </pre>
          </section>

          <Collapsible
            open={agentOpen}
            onOpenChange={setAgentOpen}
            className="fy-shurufa-panel"
          >
            <CollapsibleTrigger asChild>
              <button
                type="button"
                className="fy-shurufa-collapse-trigger"
                data-testid="companion-agent-toggle"
              >
                <span>
                  <strong>输入法 Agent 配置</strong>
                  <span className="fy-shurufa-collapse-hint">
                    桌面 OpenAI-compatible，不写入硬件
                  </span>
                </span>
                <span className="fy-shurufa-collapse-meta">
                  <Badge tone={configured ? "accent" : "warning"}>
                    {configured ? "可用" : "未完整"}
                  </Badge>
                  <CollapsibleCaret open={agentOpen}>
                    <CaretDownIcon size={16} weight="bold" />
                  </CollapsibleCaret>
                </span>
              </button>
            </CollapsibleTrigger>
            <CollapsibleContent open={agentOpen}>
              <div
                className="fy-shurufa-collapse-body"
                data-testid="companion-agent-config"
              >
                <p className="fy-shurufa-hint">
                  与 CLI 的 config.toml
                  相同：url、model、api_key、历史摘要条数、超时。 不要和上面的
                  SiliconFlow 设备转写字段混用。
                </p>
                <div className="fy-shurufa-config-grid">
                  <label className="fy-control-field">
                    API 地址
                    <Input
                      value={config.url}
                      onChange={(event) =>
                        setConfig((current) => ({
                          ...current,
                          url: event.target.value,
                        }))
                      }
                      placeholder="https://api.openai.com/v1"
                    />
                  </label>
                  <label className="fy-control-field">
                    模型
                    <Input
                      value={config.model}
                      onChange={(event) =>
                        setConfig((current) => ({
                          ...current,
                          model: event.target.value,
                        }))
                      }
                      placeholder="gpt-4o-mini"
                    />
                  </label>
                  <label className="fy-control-field fy-shurufa-config-span">
                    API Key
                    <SecretInput
                      value={config.apiKey}
                      onChange={(event) =>
                        setConfig((current) => ({
                          ...current,
                          apiKey: event.target.value,
                        }))
                      }
                      placeholder="sk-..."
                      autoComplete="off"
                    />
                  </label>
                  <label className="fy-control-field">
                    历史摘要条数
                    <Input
                      type="number"
                      min={1}
                      max={32}
                      value={config.maxSummaries}
                      onChange={(event) =>
                        setConfig((current) => ({
                          ...current,
                          maxSummaries: Number(event.target.value) || 0,
                        }))
                      }
                    />
                  </label>
                  <label className="fy-control-field">
                    超时（秒）
                    <Input
                      type="number"
                      min={1}
                      value={config.timeoutSecs}
                      onChange={(event) =>
                        setConfig((current) => ({
                          ...current,
                          timeoutSecs: Number(event.target.value) || 0,
                        }))
                      }
                    />
                  </label>
                </div>
                <div className="fy-shurufa-inline-actions">
                  <Button
                    className="fy-control-button-primary"
                    onClick={() => void saveConfig()}
                    disabled={saving || running}
                  >
                    {saving ? "保存中…" : "保存 Agent 配置"}
                  </Button>
                </div>
                {dataDir ? (
                  <CopyablePath value={dataDir} label="配置目录" />
                ) : null}
              </div>
            </CollapsibleContent>
          </Collapsible>

          <Collapsible
            open={debugOpen}
            onOpenChange={setDebugOpen}
            className="fy-shurufa-panel fy-shurufa-debug"
          >
            <CollapsibleTrigger asChild>
              <button
                type="button"
                className="fy-shurufa-collapse-trigger"
                data-testid="companion-debug-toggle"
              >
                <span>
                  <strong>调试后门</strong>
                  <span className="fy-shurufa-collapse-hint">
                    正式演示输入是硬件 ASR，不是这张文本框
                  </span>
                </span>
                <CollapsibleCaret open={debugOpen}>
                  <CaretDownIcon size={16} weight="bold" />
                </CollapsibleCaret>
              </button>
            </CollapsibleTrigger>
            <CollapsibleContent open={debugOpen}>
              <div
                className="fy-shurufa-collapse-body"
                data-testid="companion-debug-fallback"
              >
                <InlineNotice tone="warning">
                  正式演示输入来自硬件
                  ASR。下面的文本框和预览生成只用于没有硬件时快速验证
                  Agent，不会驱动自动 USB 链路。
                </InlineNotice>
                <label className="fy-control-field fy-shurufa-prompt-field">
                  调试文本
                  <textarea
                    className="fy-control-textarea fy-shurufa-prompt"
                    value={prompt}
                    onChange={(event) => updatePrompt(event.target.value)}
                    placeholder="例如：把登录按钮改成主色，点了要有 loading"
                    rows={6}
                  />
                </label>
                <div className="fy-shurufa-inline-actions">
                  <Button
                    onClick={() => void runPreview()}
                    disabled={running || loading || nativeUnavailable}
                  >
                    {running ? "生成中…" : "预览生成"}
                  </Button>
                </div>
              </div>
            </CollapsibleContent>
          </Collapsible>
        </div>
      )}
    </div>
  );
}
