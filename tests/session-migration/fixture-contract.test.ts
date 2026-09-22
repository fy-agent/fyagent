import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const fixtureRoot = join(process.cwd(), "tests/session-migration/fixtures");

function readJsonLines(name: string): Array<Record<string, unknown>> {
  const text = readFileSync(join(fixtureRoot, name), "utf8");
  expect(text.endsWith("\n")).toBe(true);
  return text
    .trimEnd()
    .split("\n")
    .map((line) => JSON.parse(line) as Record<string, unknown>);
}

function fixtureTexts(rows: Array<Record<string, unknown>>): string[] {
  const texts: string[] = [];
  const visit = (value: unknown): void => {
    if (Array.isArray(value)) {
      value.forEach(visit);
      return;
    }
    if (value === null || typeof value !== "object") return;
    for (const [key, child] of Object.entries(value)) {
      if (key === "text" && typeof child === "string") texts.push(child);
      visit(child);
    }
  };
  rows.forEach(visit);
  return texts;
}

describe("session migration native fixtures", () => {
  it("keeps every fidelity and leak sentinel in the golden source", () => {
    const rows = readJsonLines("codex-golden.jsonl");
    const source = readFileSync(
      join(fixtureRoot, "codex-golden.jsonl"),
      "utf8",
    );
    const texts = fixtureTexts(rows);

    for (const retained of [
      "KITE-728",
      "FINAL-T2-KEEP",
      "AMBER-314",
      "/Users/alice/demo",
      "D:\\work\\demo",
      "🪁",
    ]) {
      expect(texts.some((text) => text.includes(retained))).toBe(true);
    }
    for (const forbidden of [
      "LEAK_TOOL_ARG_91",
      "LEAK_TOOL_OUT_92",
      "LEAK_PROGRESS_93",
      "LEAK_REASONING_94",
      "LEAK_ATTACHMENT_95",
      "LEAK_FILE_96",
    ]) {
      expect(source).toContain(forbidden);
    }
    expect(texts.some((text) => text.includes("\r\n\r\n"))).toBe(true);
  });

  it("represents ambiguous final text without a positional escape hatch", () => {
    const rows = readJsonLines("codex-final-indeterminate.jsonl");
    const visibleAssistants = rows
      .map((row) => row.payload)
      .filter(
        (payload): payload is Record<string, unknown> =>
          payload !== null &&
          typeof payload === "object" &&
          (payload as Record<string, unknown>).role === "assistant" &&
          (payload as Record<string, unknown>).phase === undefined,
      );

    expect(visibleAssistants).toHaveLength(2);
    expect(rows.at(-1)?.payload).toMatchObject({ role: "assistant" });
    expect(rows.at(-1)?.payload).not.toHaveProperty("phase");
  });

  it("preserves consecutive and unfinished user messages as an ordered source", () => {
    const rows = readJsonLines("codex-ordered-users.jsonl");
    const messages = rows
      .map((row) => row.payload)
      .filter(
        (payload): payload is Record<string, unknown> =>
          payload !== null &&
          typeof payload === "object" &&
          (payload as Record<string, unknown>).type === "message",
      );

    expect(messages.map((message) => message.role)).toEqual([
      "user",
      "user",
      "assistant",
      "user",
    ]);
    expect(messages.at(-1)).toMatchObject({ role: "user" });
  });
});
