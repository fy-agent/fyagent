import { afterEach, describe, expect, it, vi } from "vitest";

async function loadPresets(platform: "windows" | "macos" | "unknown") {
  vi.doMock("@/v2/shared/platform/runtime", () => ({
    detectNativePlatform: () => platform,
  }));
  vi.resetModules();
  const [legacy, v2] = await Promise.all([
    import("@/config/mcpPresets"),
    import("@/v2/shared/features/presets"),
  ]);
  expect(legacy.mcpPresets).toBe(v2.mcpPresets);
  return legacy;
}

describe("MCP preset platform commands", () => {
  afterEach(() => {
    vi.doUnmock("@/v2/shared/platform/runtime");
    vi.resetModules();
  });

  it("uses uvx for the time preset on every platform", async () => {
    const { mcpPresets } = await loadPresets("windows");
    const time = mcpPresets.find((preset) => preset.id === "time");

    expect(time?.server).toMatchObject({
      type: "stdio",
      command: "uvx",
      args: ["mcp-server-time"],
    });
  });

  it("wraps npx with cmd only on Windows", async () => {
    const { mcpPresets } = await loadPresets("windows");
    const memory = mcpPresets.find((preset) => preset.id === "memory");

    expect(memory?.server).toMatchObject({
      type: "stdio",
      command: "cmd",
      args: ["/c", "npx", "-y", "@modelcontextprotocol/server-memory"],
    });
  });

  it("uses direct npx on macOS and unknown hosts", async () => {
    const macos = await loadPresets("macos");
    expect(
      macos.mcpPresets.find((preset) => preset.id === "memory")?.server,
    ).toMatchObject({
      type: "stdio",
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-memory"],
    });

    const unknown = await loadPresets("unknown");
    expect(
      unknown.mcpPresets.find((preset) => preset.id === "memory")?.server,
    ).toMatchObject({
      type: "stdio",
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-memory"],
    });
    expect(unknown.mcpPresets.map((preset) => preset.id)).toEqual([
      "fetch",
      "time",
      "memory",
      "sequential-thinking",
      "context7",
    ]);
  });
});
