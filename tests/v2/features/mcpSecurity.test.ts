import { describe, expect, it } from "vitest";

import {
  mcpRecipeIdentity,
  mcpUrlSearchToken,
  redactMcpArgs,
  redactMcpUrl,
} from "@/v2/shared/features/mcpSecurity";

describe("MCP secret redaction", () => {
  it("masks sensitive URL query values but keeps the public origin and path", () => {
    expect(redactMcpUrl("https://mcp.amap.com/mcp?key=amap-query-secret")).toBe(
      "https://mcp.amap.com/mcp?key=••••••",
    );
    expect(
      mcpUrlSearchToken("https://mcp.amap.com/mcp?key=amap-query-secret"),
    ).toBe("https://mcp.amap.com/mcp");
    expect(
      redactMcpUrl(
        "https://mcp.lexiang-app.com/mcp?company_from=acme&access_token=lexiang-secret",
      ),
    ).toBe(
      "https://mcp.lexiang-app.com/mcp?company_from=acme&access_token=••••••",
    );
    expect(redactMcpUrl("https://example.com/mcp?secretKey=cloud-secret")).toBe(
      "https://example.com/mcp?secretKey=••••••",
    );
  });

  it("masks sensitive command arguments after known flags", () => {
    expect(
      redactMcpArgs(["mcp", "-a", "cli_app", "-s", "feishu-app-secret"]),
    ).toEqual(["mcp", "-a", "cli_app", "-s", "••••••"]);
    expect(redactMcpArgs(["--token=github-pat"])).toEqual(["--token=••••••"]);
  });

  it("treats Windows cmd npx and Unix npx as the same recipe", () => {
    expect(
      mcpRecipeIdentity({
        type: "stdio",
        command: "cmd",
        args: ["/c", "npx", "-y", "@playwright/mcp@latest"],
      }),
    ).toBe(
      mcpRecipeIdentity({
        type: "stdio",
        command: "npx",
        args: ["-y", "@playwright/mcp@latest"],
      }),
    );
  });
});
