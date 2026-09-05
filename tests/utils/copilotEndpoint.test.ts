import { describe, expect, it } from "vitest";
import { isCopilotEndpoint } from "@/utils/providerCapabilities";

describe("Copilot endpoint identity hint", () => {
  it.each([
    "https://githubcopilot.com",
    "https://api.githubcopilot.com/v1",
    "https://api.business.githubcopilot.com/chat/completions",
    "https://API.GITHUBCOPILOT.COM/v1",
  ])("recognizes a parsed HTTPS Copilot host: %s", (value) => {
    expect(isCopilotEndpoint(value)).toBe(true);
  });

  it.each([
    "https://githubcopilot.com.example.test/",
    "https://notgithubcopilot.com/",
    "https://example.test/githubcopilot.com",
    "https://example.test/?target=githubcopilot.com",
    "https://githubcopilot.com@example.test/",
    "https://user:password@api.githubcopilot.com/",
    "http://api.githubcopilot.com/",
    "file://githubcopilot.com/",
    "githubcopilot.com",
    "not a URL",
  ])("rejects misleading or unsupported endpoint: %s", (value) => {
    expect(isCopilotEndpoint(value)).toBe(false);
  });
});
