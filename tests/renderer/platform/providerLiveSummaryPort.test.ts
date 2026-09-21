import { beforeEach, describe, expect, it, vi } from "vitest";
import { createModelFeaturePorts } from "@/shared/platform/tauri/feature-ports/models";
import type { ProviderAppId } from "@/shared/features/models";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const saved = {
  providers: {
    saved: {
      id: "saved",
      name: "Saved source",
      writeTargets: [],
      connection: {
        baseUrl: "https://saved.example.test/v1",
        modelId: "saved-model",
        protocol: "responses",
      },
    },
  },
  currentId: "saved",
  writeTargets: [],
};
const connection = {
  baseUrl: "https://external.example.test/v1",
  modelId: "external-model",
  protocol: "chat",
};
const configured = {
  target: "codex",
  state: "configured",
  exists: true,
  connection,
};
const unknownLive = {
  target: "codex",
  state: "unreadable",
  exists: null,
  connection: null,
};

async function read(live: unknown, target: ProviderAppId = "codex") {
  invoke.mockResolvedValue({ ...saved, live });
  return createModelFeaturePorts().providers.getSummary(target);
}

describe("actual Provider target summary boundary", () => {
  beforeEach(() => invoke.mockReset());

  it("keeps live external routing separate from DB current and only reads once", async () => {
    const summary = await read(configured);
    expect(summary.providers).toEqual(saved.providers);
    expect(summary.currentId).toBe("saved");
    expect(summary.live).toEqual(configured);
    expect(invoke.mock.calls).toEqual([
      ["get_provider_summary", { app: "codex" }],
    ]);
  });

  it.each([
    { state: "missing", exists: false, connection: null },
    { state: "not_configured", exists: true, connection: null },
    { state: "unreadable", exists: true, connection: null },
    { state: "unreadable", exists: false, connection: null },
    { state: "unreadable", exists: null, connection: null },
  ])(
    "preserves independent %s without dropping saved sources",
    async (state) => {
      const live = { target: "codex", ...state };
      const summary = await read(live);
      expect(summary.live).toEqual(live);
      expect(summary.providers).toEqual(saved.providers);
    },
  );

  it("treats old hosts with no live field as unknown, never missing", async () => {
    invoke.mockResolvedValue(saved);
    const summary =
      await createModelFeaturePorts().providers.getSummary("codex");
    expect(summary.live).toEqual(unknownLive);
    expect(summary.providers).toEqual(saved.providers);
  });

  it.each([
    { baseUrl: null, modelId: "explicit-model", protocol: null },
    {
      baseUrl: "https://external.example.test/v1",
      modelId: null,
      protocol: null,
    },
  ])("keeps unspecified partial fields null", async (value) => {
    const live = { ...configured, connection: value };
    expect((await read(live)).live).toEqual(live);
  });

  it.each([
    undefined,
    null,
    {},
    { ...configured, target: "claude" },
    { ...configured, exists: false },
    { ...configured, state: "missing" },
    { ...configured, rawText: "fixture-private-config" },
    { ...configured, connection: { ...connection, apiKey: "fixture-secret" } },
    {
      ...configured,
      connection: { ...connection, credentialRef: "secretref" },
    },
    { ...configured, connection: { ...connection, protocol: "anthropic" } },
    { ...configured, connection: { ...connection, protocol: "unknown" } },
    { ...configured, connection: { ...connection, modelId: " " } },
    { ...configured, connection: { ...connection, modelId: "line\nbreak" } },
    { ...configured, connection: { ...connection, modelId: "m".repeat(257) } },
    {
      ...configured,
      connection: { ...connection, baseUrl: "file:///tmp/config" },
    },
    {
      ...configured,
      connection: {
        ...connection,
        baseUrl: "https://user:secret@example.test/v1",
      },
    },
    {
      ...configured,
      connection: {
        ...connection,
        baseUrl: "https://example.test/v1?api_key=secret",
      },
    },
    {
      ...configured,
      connection: { ...connection, baseUrl: "https://example.test/v1#secret" },
    },
    {
      ...configured,
      connection: {
        ...connection,
        baseUrl: `https://example.test/${"x".repeat(2048)}`,
      },
    },
    {
      ...configured,
      connection: { baseUrl: null, modelId: null, protocol: "responses" },
    },
    { ...configured, connection: {} },
  ])("isolates malformed or unsafe live data %#", async (live) => {
    const summary = await read(live);
    expect(summary).toEqual({ ...saved, live: unknownLive });
  });

  it.each([
    ["claude", "anthropic"],
    ["grokbuild", "responses"],
  ] as const)(
    "binds the observation to requested target %s",
    async (target, protocol) => {
      const live = {
        ...configured,
        target,
        connection: { ...connection, protocol },
      };
      expect((await read(live, target)).live).toEqual(live);
    },
  );

  it("does not relax the saved source or root allowlist", async () => {
    for (const payload of [
      { ...saved, live: configured, apiKey: "fixture-secret" },
      { ...saved, live: configured, currentId: "not-saved" },
      {
        ...saved,
        live: configured,
        providers: {
          saved: { ...saved.providers.saved, auth: "fixture-secret" },
        },
      },
    ]) {
      invoke.mockResolvedValue(payload);
      await expect(
        createModelFeaturePorts().providers.getSummary("codex"),
      ).rejects.toThrow("Provider public summary is unavailable");
    }
  });
});
