import {
  buildQuickSetupRequest,
  claudeBaseUrlHasExplicitV1Path,
  MODEL_TARGETS,
  parseManualModelIds,
  parseModelTarget,
  QUICK_SETUP_PROVIDER_IDS,
  validateQuickSetup,
} from "@/v2/pages/models/quickSetup";

describe("models quick setup helpers", () => {
  it("accepts only known non-secret target query values", () => {
    expect(MODEL_TARGETS).toEqual([
      "qoderwork",
      "trae",
      "workbuddy",
      "grokbuild",
      "codex",
      "claude",
      "opencode",
    ]);
    expect(parseModelTarget("codex")).toBe("codex");
    expect(parseModelTarget("trae")).toBe("trae");
    expect(parseModelTarget("unknown")).toBe("qoderwork");
    expect(parseModelTarget(null)).toBe("qoderwork");
  });

  it("normalizes and validates every provider field", () => {
    expect(
      validateQuickSetup({
        name: "  Team Gateway  ",
        baseUrl: " https://gateway.example/v1 ",
        apiKey: " test-key ",
        modelId: " model-a ",
      }),
    ).toEqual({
      ok: true,
      value: {
        name: "Team Gateway",
        baseUrl: "https://gateway.example/v1",
        apiKey: "test-key",
        modelId: "model-a",
      },
    });

    const invalid = validateQuickSetup({
      name: " ",
      baseUrl: "https://user:pass@gateway.example/v1",
      apiKey: " ",
      modelId: " ",
    });
    expect(invalid.ok).toBe(false);
    if (!invalid.ok) expect(Object.keys(invalid.errors)).toHaveLength(4);
  });

  it.each([
    "https://gateway.example/v1?api_key=secret",
    "https://gateway.example/v1#secret",
  ])("rejects query and fragment URL material: %s", (baseUrl) => {
    const result = validateQuickSetup({
      name: "Gateway",
      baseUrl,
      apiKey: "secret",
      modelId: "model-a",
    });
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.errors.baseUrl).toContain("HTTP(S)");
  });

  it("builds the exact minimal quick-setup request", () => {
    const request = buildQuickSetupRequest("claude", {
      name: "Claude Gateway",
      baseUrl: "https://claude.example/v1",
      apiKey: "claude-test-key",
      modelId: "claude-model",
    });

    expect(request).toEqual({
      name: "Claude Gateway",
      baseUrl: "https://claude.example/v1",
      apiKey: "claude-test-key",
      modelId: "claude-model",
    });
  });

  it.each(["name", "modelId"] as const)(
    "rejects a trimmed %s containing the API key",
    (field) => {
      const result = validateQuickSetup({
        name: field === "name" ? " prefix-secret-key-suffix " : "Gateway",
        baseUrl: "https://gateway.example/v1",
        apiKey: " secret-key ",
        modelId: field === "modelId" ? " prefix-secret-key-suffix " : "model-a",
      });

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.errors[field]).toContain("API Key");
        expect(JSON.stringify(result.errors)).not.toContain("secret-key");
      }
    },
  );

  it("rejects an API key equal to the target's reserved provider ID", () => {
    const result = validateQuickSetup(
      {
        name: "Gateway",
        baseUrl: "https://gateway.example/v1",
        apiKey: QUICK_SETUP_PROVIDER_IDS.codex,
        modelId: "model-a",
      },
      "codex",
    );
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.errors.apiKey).toContain("不能使用该值");
      expect(JSON.stringify(result.errors)).not.toContain(
        QUICK_SETUP_PROVIDER_IDS.codex,
      );
    }
  });

  it("keeps Codex request data in the dedicated DTO", () => {
    const request = buildQuickSetupRequest("codex", {
      name: 'Codex "Gateway"',
      baseUrl: "https://codex.example/v1",
      apiKey: "codex-test-key",
      modelId: "vendor/model-a",
    });

    expect(request).toEqual({
      name: 'Codex "Gateway"',
      baseUrl: "https://codex.example/v1",
      apiKey: "codex-test-key",
      modelId: "vendor/model-a",
    });
  });

  it.each(["safe-key", "safe%2Dkey"])(
    "rejects an API key in a decoded URL path segment: %s",
    (segment) => {
      const result = validateQuickSetup({
        name: "Gateway",
        baseUrl: `https://gateway.example/${segment}/v1`,
        apiKey: "safe-key",
        modelId: "model-a",
      });
      expect(result.ok).toBe(false);
      if (!result.ok) expect(result.errors.baseUrl).toContain("API Key");
    },
  );

  it("parses ordered unique manual model IDs", () => {
    expect(parseManualModelIds(" alpha, beta\nalpha\n\nGamma ")).toEqual([
      "alpha",
      "beta",
      "Gamma",
    ]);
  });

  it.each([
    ["https://gateway.example/v1", true],
    ["https://gateway.example/v1/", true],
    ["https://gateway.example/api/v1/messages", true],
    ["https://gateway.example/anthropic", false],
    ["https://v1.example.com", false],
    ["https://v1.example.com/anthropic", false],
    ["https://gateway.example/v10", false],
    ["not a url", false],
  ])("detects an explicit Claude v1 path in %s", (baseUrl, expected) => {
    expect(claudeBaseUrlHasExplicitV1Path(baseUrl)).toBe(expected);
  });
});
