import { describe, expect, it } from "vitest";

import {
  agentReturnPathFromLocationState,
  agentReturnPathFromSearch,
  createAgentReturnLocationState,
} from "@/v2/shared/features/agent-navigation";

describe("Agent return navigation", () => {
  it("derives a return URL from the closed Agent and section tuple", () => {
    expect(
      agentReturnPathFromLocationState(
        createAgentReturnLocationState("workbuddy", "mcp"),
      ),
    ).toBe("/agents?target=workbuddy&section=mcp");
  });

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

  it.each([
    null,
    {},
    { fyagentAgentReturn: { agentId: "unknown", section: "mcp" } },
    { fyagentAgentReturn: { agentId: "workbuddy", section: "settings" } },
    { fyagentAgentReturn: "/agents?target=workbuddy&section=mcp" },
    { path: "/agents?target=workbuddy&section=mcp" },
  ])("rejects arbitrary or malformed location state", (state) => {
    expect(agentReturnPathFromLocationState(state)).toBeNull();
  });
});
