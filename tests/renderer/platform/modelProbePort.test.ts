import { beforeEach, describe, expect, it, vi } from "vitest";
import { createModelFeaturePorts } from "@/shared/platform/tauri/feature-ports/models";
import {
  createBrowserFeaturePorts,
  NATIVE_ONLY_ERROR,
} from "@/shared/platform/browser/features";
import type { ModelProbeRequest } from "@/shared/features/models";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
const requestId = "00000000-0000-4000-8000-000000000001";
const otherId = "00000000-0000-4000-8000-000000000002";
const request: ModelProbeRequest = {
  requestId,
  app: "codex",
  baseUrl: "https://example.test/v1",
  apiKey: "fixture-key",
  modelId: "model-a",
  protocol: "chat",
};
const result = {
  requestId,
  terminal: "completed",
  requestCount: 2,
  retryCount: 1,
  inputMode: "compatibility",
  success: true,
  status: "operational",
  message: "OK",
  responseTimeMs: 12,
  httpStatus: 200,
  modelUsed: "model-a",
  testedAt: 1,
};
const snapshot = {
  requestId,
  phase: "cancelling",
  requestCount: 1,
  retryCount: 0,
};
const owners = ["providers", "workbuddy", "opencodeModels"] as const;

beforeEach(() => invoke.mockReset());
describe.each(owners)(
  "%s model probe identity and lifecycle boundary",
  (owner) => {
    it("preserves exact dispatch and retry counts in the public result", async () => {
      invoke.mockResolvedValue(result);
      await expect(
        createModelFeaturePorts()[owner].checkModel(request),
      ).resolves.toMatchObject({
        requestId,
        terminal: "completed",
        requestCount: 2,
        retryCount: 1,
        inputMode: "compatibility",
      });
      expect(invoke).toHaveBeenCalledWith(
        "stream_check_model",
        expect.objectContaining(request),
      );
    });
    it("sends identity alone to cancellation and state reads", async () => {
      invoke.mockResolvedValue(snapshot);
      const port = createModelFeaturePorts()[owner];
      await expect(port.cancelModelProbe(requestId)).resolves.toEqual(snapshot);
      await expect(port.getModelProbeStatus(requestId)).resolves.toEqual(
        snapshot,
      );
      expect(invoke.mock.calls).toEqual([
        ["stream_check_model_cancel", { requestId }],
        ["stream_check_model_status", { requestId }],
      ]);
      expect(JSON.stringify(invoke.mock.calls)).not.toContain(request.apiKey);
    });
    it.each([
      "",
      "legacy",
      "ABCDEF00-0000-4000-8000-000000000001",
      "00000000-0000-4000-0000-000000000001",
    ])("rejects invalid identity %s before any invocation", async (invalid) => {
      const port = createModelFeaturePorts()[owner];
      await expect(
        port.checkModel({ ...request, requestId: invalid }),
      ).rejects.toThrow("invalid");
      await expect(port.getModelProbeStatus(invalid)).rejects.toThrow(
        "invalid",
      );
      await expect(port.cancelModelProbe(invalid)).rejects.toThrow("invalid");
      expect(invoke).not.toHaveBeenCalled();
    });
    it.each([
      { ...result, requestId: otherId },
      { ...result, requestId: null },
      { ...result, requestCount: 3 },
      { ...result, requestCount: 2, retryCount: 0 },
      { ...result, requestCount: 0, retryCount: 0 },
      { ...result, terminal: "cancelled" },
      { ...result, apiKey: "forbidden" },
      { ...result, inputMode: "unknown" },
    ])(
      "rejects mismatched, impossible or expanded terminal payload %#",
      async (invalid) => {
        invoke.mockResolvedValue(invalid);
        await expect(
          createModelFeaturePorts()[owner].checkModel(request),
        ).rejects.toThrow("unavailable");
      },
    );
    it("allows cancelled-before-dispatch without inventing a request", async () => {
      invoke.mockResolvedValue({
        ...result,
        terminal: "cancelled",
        success: false,
        status: "failed",
        requestCount: 0,
        retryCount: 0,
      });
      await expect(
        createModelFeaturePorts()[owner].checkModel(request),
      ).resolves.toMatchObject({
        requestId,
        terminal: "cancelled",
        success: false,
        requestCount: 0,
        retryCount: 0,
      });
    });
    it.each([
      { ...snapshot, requestId: otherId },
      { ...snapshot, phase: "unknown" },
      { ...snapshot, phase: "retrying" },
      { ...snapshot, phase: "running", requestCount: 2, retryCount: 1 },
      { ...snapshot, requestCount: 2, retryCount: 0 },
      { ...snapshot, credentialRef: "forbidden" },
    ])("rejects invalid state/cancel acknowledgement %#", async (invalid) => {
      invoke.mockResolvedValue(invalid);
      const port = createModelFeaturePorts()[owner];
      await expect(port.getModelProbeStatus(requestId)).rejects.toThrow(
        "unavailable",
      );
      await expect(port.cancelModelProbe(requestId)).rejects.toThrow(
        "unavailable",
      );
    });
    it("keeps all lifecycle calls native-only in browser adapters", async () => {
      const port = createBrowserFeaturePorts()[owner];
      await expect(port.getModelProbeStatus(requestId)).rejects.toThrow(
        NATIVE_ONLY_ERROR,
      );
      await expect(port.cancelModelProbe(requestId)).rejects.toThrow(
        NATIVE_ONLY_ERROR,
      );
    });
  },
);
