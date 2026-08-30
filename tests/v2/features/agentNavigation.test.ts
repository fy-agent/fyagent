import { describe, expect, it } from "vitest";

import {
  agentReturnDescriptorFromManagementSearch,
  agentReturnPathFromSearch,
  appendAgentReturnToPath,
} from "@/v2/shared/features/agent-navigation";

describe("Agent return navigation", () => {
  it("derives the same closed return URL from an exact Agents query", () => {
    expect(agentReturnPathFromSearch("?target=workbuddy&section=mcp")).toBe(
      "/agents?target=workbuddy&section=mcp",
    );
    expect(agentReturnPathFromSearch("?section=mcp&target=workbuddy")).toBe(
      "/agents?target=workbuddy&section=mcp",
    );
  });

  it.each([
    "",
    "?target=workbuddy",
    "?target=unknown&section=mcp",
    "?target=workbuddy&section=settings",
    "?target=workbuddy&section=mcp&path=/tmp",
    "?target=workbuddy&target=codex&section=mcp",
  ])("rejects malformed or extended Agents queries", (search) => {
    expect(agentReturnPathFromSearch(search)).toBeNull();
  });

  it("reads a closed return tuple from a management query with route-owned fields", () => {
    expect(
      agentReturnDescriptorFromManagementSearch(
        "?target=claude-code&agentReturn=workbuddy&agentSection=mcp",
      ),
    ).toEqual({ agentId: "workbuddy", section: "mcp" });
  });

  it.each([
    "",
    "?agentReturn=workbuddy",
    "?agentReturn=unknown&agentSection=mcp",
    "?agentReturn=workbuddy&agentSection=settings",
    "?agentReturn=workbuddy&agentReturn=codex&agentSection=mcp",
    "?agentReturn=workbuddy&agentSection=mcp&agentSection=models",
  ])("rejects malformed management return queries", (search) => {
    expect(agentReturnDescriptorFromManagementSearch(search)).toBeNull();
  });

  it("appends the return tuple without dropping a route-owned query", () => {
    expect(
      appendAgentReturnToPath("/models?target=claude-code", {
        agentId: "workbuddy",
        section: "mcp",
      }),
    ).toBe(
      "/models?target=claude-code&agentReturn=workbuddy&agentSection=mcp",
    );
  });
});
