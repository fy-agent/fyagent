import { describe, expect, it } from "vitest";
import {
  parseClaudeQuickSetupApplyRequest,
  parseClaudeQuickSetupOutcome,
  parseClaudeQuickSetupPreview,
} from "@/shared/features/claude-quick-setup";

const previewId = "11111111-1111-4111-8111-111111111111";
const preview = {
  contractVersion: 1,
  previewId,
  writeTargets: [
    {
      path: "~/.claude/settings.json",
      backupPath: "~/.claude/settings.json.fyagent.backup",
      exists: false,
    },
  ],
  preservedPaths: ["~/.claude.json"],
  sidecars: [
    {
      target: "claude_settings",
      backupPath: null,
      undoPath: "~/.claude/settings.json.fyagent.undo.json",
    },
  ],
};
describe("Claude private consent DTO", () => {
  it("accepts identity only and rejects new authority or credentials", () => {
    expect(parseClaudeQuickSetupApplyRequest({ previewId })).toEqual({
      previewId,
    });
    for (const extra of [
      { request: {} },
      { apiKey: "secret" },
      { path: "C:/file" },
      { hash: "hash" },
    ]) {
      expect(() =>
        parseClaudeQuickSetupApplyRequest({ previewId, ...extra }),
      ).toThrow();
    }
    expect(() =>
      parseClaudeQuickSetupApplyRequest({
        previewId: "unbounded arbitrary identity",
      }),
    ).toThrow();
  });
  it("requires closed, consistent display-only preview fields", () => {
    expect(parseClaudeQuickSetupPreview(preview)).toEqual(preview);
    for (const invalid of [
      { ...preview, contractVersion: 2 },
      { ...preview, apiKey: "secret" },
      { ...preview, preservedPaths: [preview.writeTargets[0].path] },
      { ...preview, sidecars: [] },
      {
        ...preview,
        sidecars: [{ ...preview.sidecars[0], backupPath: "fake prior backup" }],
      },
      { ...preview, preservedPaths: ["bad\npath"] },
    ])
      expect(() => parseClaudeQuickSetupPreview(invalid)).toThrow();
  });
  it("keeps each file authoritative and rejects upgraded or inconsistent results", () => {
    const outcome = {
      contractVersion: 1,
      overall: "partial",
      providerState: "applied",
      files: [
        { target: "claude_settings", state: "applied" },
        { target: "claude_mcp", state: "conflict" },
      ],
    };
    expect(parseClaudeQuickSetupOutcome(outcome)).toEqual(outcome);
    for (const invalid of [
      { ...outcome, overall: "applied" },
      { ...outcome, overall: "stale" },
      { ...outcome, providerState: "unchanged" },
      { ...outcome, files: [outcome.files[0], outcome.files[0]] },
      { ...outcome, secret: "leak" },
    ])
      expect(() => parseClaudeQuickSetupOutcome(invalid)).toThrow();
  });
});
