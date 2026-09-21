import { describe, expect, it } from "vitest";
import {
  PROVIDER_API_PRESETS,
  isToolOnlyApi,
  presetConnection,
  providerApiCredentialError,
} from "@/domain/configuration/providerApi";
import {
  buildQuickSetupRequest,
  validateQuickSetup,
} from "@/pages/models/quickSetup";

describe("production API presets and protocol boundary", () => {
  it("keeps vendor products, endpoints and credentials separate", () => {
    expect(PROVIDER_API_PRESETS.map((preset) => preset.id)).toEqual([
      "aliyun-payg",
      "aliyun-coding",
      "tencent-tokenhub",
      "ark-payg",
      "ark-coding",
    ]);
    const [aliApi, aliCoding, tencent, arkApi, arkCoding] =
      PROVIDER_API_PRESETS;
    expect(presetConnection(aliApi, "codex")).toMatchObject({
      protocol: "responses",
      baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    });
    expect(presetConnection(aliCoding, "codex")).toMatchObject({
      protocol: "chat",
      baseUrl: "https://coding.dashscope.aliyuncs.com/v1",
    });
    expect(presetConnection(aliCoding, "claude")?.baseUrl).toBe(
      "https://coding.dashscope.aliyuncs.com/apps/anthropic",
    );
    expect(presetConnection(aliCoding, "grokbuild")).toBeNull();
    expect(presetConnection(tencent, "codex")).toMatchObject({
      baseUrl: "https://tokenhub.tencentmaas.com/v1",
      modelId: "hy3",
    });
    expect(tencent.keyScope).toContain("Hy3");
    expect(presetConnection(arkApi, "codex")?.baseUrl).toBe(
      "https://ark.cn-beijing.volces.com/api/v3",
    );
    expect(presetConnection(arkCoding, "codex")?.baseUrl).toBe(
      "https://ark.cn-beijing.volces.com/api/coding/v3",
    );
    expect(presetConnection(arkCoding, "claude")?.baseUrl).toBe(
      "https://ark.cn-beijing.volces.com/api/coding",
    );
  });

  it("carries explicit Chat through validation and the existing request builder", () => {
    const validated = validateQuickSetup(
      {
        name: "Coding",
        baseUrl: "https://coding.dashscope.aliyuncs.com/v1",
        apiKey: "sk-sp-fixture",
        modelId: "model-a",
        protocol: "chat",
      },
      "codex",
    );
    expect(validated.ok).toBe(true);
    if (validated.ok)
      expect(buildQuickSetupRequest("codex", validated.value)).toEqual({
        name: "Coding",
        baseUrl: "https://coding.dashscope.aliyuncs.com/v1",
        apiKey: "sk-sp-fixture",
        modelId: "model-a",
        protocol: "chat",
      });
    expect(
      validateQuickSetup(
        {
          name: "Wrong target",
          baseUrl: "https://example.com/v1",
          apiKey: "fixture",
          modelId: "m",
          protocol: "chat",
        },
        "claude",
      ).ok,
    ).toBe(false);
  });

  it("rejects known wrong-product keys and protocol without including secrets", () => {
    const fixture = "sk-sp-not-a-real-credential";
    const failures = [
      providerApiCredentialError(
        "https://dashscope.aliyuncs.com/compatible-mode/v1",
        fixture,
        "responses",
      ),
      providerApiCredentialError(
        "https://coding.dashscope.aliyuncs.com/v1",
        "ordinary-fixture",
        "chat",
      ),
      providerApiCredentialError(
        "https://coding.dashscope.aliyuncs.com/v1",
        fixture,
        "responses",
      ),
    ];
    expect(failures.every(Boolean)).toBe(true);
    expect(JSON.stringify(failures)).not.toContain(fixture);
  });

  it.each([
    "https://coding.dashscope.aliyuncs.com/v1",
    "https://ark.cn-beijing.volces.com/api/coding/v3",
    "https://ark.cn-beijing.volces.com/api/coding",
    "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
  ])("blocks generic discovery and probes for %s", (endpoint) => {
    expect(isToolOnlyApi(endpoint)).toBe(true);
  });

  it("does not classify lookalike or general API URLs as Coding Plan", () => {
    expect(isToolOnlyApi("https://ark.cn-beijing.volces.com/api/v3")).toBe(
      false,
    );
    expect(
      isToolOnlyApi(
        "https://ark.cn-beijing.volces.com.example.com/api/coding/v3",
      ),
    ).toBe(false);
    expect(isToolOnlyApi("https://example.com/v1", "sk-sp-fixture")).toBe(true);
  });
});
