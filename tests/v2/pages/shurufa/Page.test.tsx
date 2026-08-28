import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";

import { ShurufaPage } from "@/v2/pages/shurufa/Page";
import type {
  CompanionSnapshot,
  FeaturePorts,
  ShurufaSnapshot,
} from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";

const agentSnapshot: ShurufaSnapshot = {
  prompt: "调试文本",
  config: {
    url: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    apiKey: "",
    maxSummaries: 8,
    timeoutSecs: 60,
    configured: true,
  },
  running: false,
  lastOutput: "优化后的提示词",
  lastError: null,
  shortcutLabel: "Ctrl+M",
  dataDir: "/tmp/shurufa",
};

const companionSnapshot: CompanionSnapshot = {
  ports: ["COM3"],
  profile: {
    version: 1,
    revision: "rev-1",
    serial: { port: "COM3", baud: 115200 },
    target: { processName: "notepad.exe", processPath: "C:\\\\Windows\\\\notepad.exe" },
    mappings: [
      { input: "ENCODER_CW", displayName: "上一项", keys: ["CTRL", "TAB"] },
      { input: "ENCODER_CCW", displayName: "下一项", keys: ["CTRL", "SHIFT", "TAB"] },
      { input: "ENCODER_PRESS", displayName: "确认动作", keys: ["ENTER"] },
    ],
  },
  device: {
    version: 1,
    ssid: "lab-24g",
    password: "",
    apiKey: "",
    model: "XingChenAGI/XingChenASR-V3.2-Ultra",
  },
  runtime: {
    state: "STOPPED",
    liveEnabled: false,
    lastEvent: "尚无事件。",
    gapMissed: null,
    network: {
      state: "UNKNOWN",
      ssid: "",
      ip: "",
      rssi: null,
      reason: null,
      pingHost: null,
      pingOk: null,
      pingMs: null,
      pingLost: null,
      pingSent: null,
      lastLog: null,
      beats: null,
      recState: "DONE",
      recMs: 1200,
      recSamples: 19200,
      recRms: null,
      recPeak: null,
      recSilence: null,
      recReason: null,
      asrState: "DONE",
      asrText: "把按钮改成主色",
      asrReason: null,
    },
  },
  lastAsrSeq: 2,
  lastAsrAdmission: "admitted",
  lastAsrError: null,
};

function desktopPorts(): FeaturePorts {
  const ports = createBrowserFeaturePorts();
  ports.shurufa.getSnapshot = vi.fn(async () => agentSnapshot);
  ports.shurufa.getCompanionSnapshot = vi.fn(async () => companionSnapshot);
  ports.shurufa.subscribe = vi.fn(async () => () => {});
  ports.shurufa.setPrompt = vi.fn(async () => undefined);
  ports.shurufa.saveConfig = vi.fn(async (config) => ({
    ...config,
    configured: true,
  }));
  ports.shurufa.clearSession = vi.fn(async () => 0);
  ports.shurufa.run = vi.fn(async () => "ok");
  ports.shurufa.listCompanionPorts = vi.fn(async () => ["COM3", "COM5"]);
  ports.shurufa.captureCompanionTarget = vi.fn(async () => ({
    processName: "code.exe",
    processPath: "C:\\\\App\\\\code.exe",
  }));
  ports.shurufa.saveCompanionProfile = vi.fn(async (draft) => ({
    ...draft,
    revision: "rev-2",
  }));
  ports.shurufa.startCompanionDryRun = vi.fn(async () => ({
    ...companionSnapshot.runtime,
    state: "DRY_RUN" as const,
  }));
  ports.shurufa.enableCompanionLive = vi.fn(async () => ({
    ...companionSnapshot.runtime,
    state: "LIVE" as const,
    liveEnabled: true,
  }));
  ports.shurufa.stopCompanion = vi.fn(async () => companionSnapshot.runtime);
  ports.shurufa.saveCompanionDeviceSettings = vi.fn(async (draft) => draft);
  ports.shurufa.applyCompanionDeviceConfig = vi.fn(async () => ({
    ...companionSnapshot.runtime.network,
    state: "CONNECTING" as const,
    ssid: "lab-24g",
  }));
  return ports;
}

function renderPage(ports: FeaturePorts) {
  render(
    <FeatureProvider ports={ports}>
      <ShurufaPage />
    </FeatureProvider>,
  );
}

describe("Shurufa companion page", () => {
  it("keeps native-only browser preview honest and does not seed ports", async () => {
    renderPage(createBrowserFeaturePorts());
    expect(
      await screen.findByText(/只在 FyAgent 桌面应用中可用/),
    ).toBeInTheDocument();
    expect(screen.queryByText("COM3")).not.toBeInTheDocument();
    expect(screen.queryByText("把按钮改成主色")).not.toBeInTheDocument();
  });

  it("recomposes Companion console with separated device and Agent configs", async () => {
    const ports = desktopPorts();
    renderPage(ports);

    const serial = await screen.findByTestId("companion-serial");
    expect(serial).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "COM3" })).toBeInTheDocument();
    expect(screen.getByText("波特率 115200")).toBeInTheDocument();
    expect(screen.getAllByText("已停止").length).toBeGreaterThan(0);
    expect(screen.getByText("把按钮改成主色")).toBeInTheDocument();
    expect(screen.getByText(/19200 采样/)).toBeInTheDocument();
    expect(screen.getByText("优化后的提示词")).toBeInTheDocument();
    expect(
      screen.getAllByText(/正式演示输入来自串口 ASR/).length,
    ).toBeGreaterThan(0);

    expect(screen.getByText("设备转写配置")).toBeInTheDocument();
    await userEvent.click(screen.getByTestId("companion-device-toggle"));
    expect(await screen.findByLabelText("SiliconFlow API Key")).toBeInTheDocument();
    expect(screen.getByLabelText("转写模型")).toBeInTheDocument();

    await userEvent.click(screen.getByTestId("companion-agent-toggle"));
    expect(await screen.findByTestId("companion-agent-config")).toHaveTextContent(
      "不要和上面的 SiliconFlow",
    );
    expect(screen.getByLabelText("API 地址")).toBeInTheDocument();

    await userEvent.click(screen.getByTestId("companion-debug-toggle"));
    expect(await screen.findByTestId("companion-debug-fallback")).toHaveTextContent(
      "正式演示输入来自串口 ASR",
    );
    expect(screen.getByLabelText("调试文本")).toHaveValue("调试文本");

    await waitFor(() => {
      expect(ports.shurufa.getCompanionSnapshot).toHaveBeenCalled();
    });
  });

  it("refreshes ports through the feature port, not a page-local invoke", async () => {
    const ports = desktopPorts();
    renderPage(ports);
    await screen.findByTestId("companion-serial");
    await userEvent.click(screen.getByRole("button", { name: "刷新" }));
    await waitFor(() => {
      expect(ports.shurufa.listCompanionPorts).toHaveBeenCalledTimes(1);
    });
    expect(await screen.findByRole("option", { name: "COM5" })).toBeInTheDocument();
  });

  it("does not call invoke from the shurufa page tree", () => {
    const pageDir = join(process.cwd(), "src/v2/pages/shurufa");
    const source = [
      "Page.tsx",
      "ChordField.tsx",
      "companion.ts",
    ]
      .map((name) => readFileSync(join(pageDir, name), "utf8"))
      .join("\n");
    expect(source).not.toMatch(/\binvoke\s*\(/);
    expect(source).not.toMatch("@tauri-apps/api");
    expect(source).not.toMatch("src/components");
    expect(source).not.toMatch("src/lib");
  });
});
