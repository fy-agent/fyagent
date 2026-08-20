import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const WORKFLOWS_DIR = path.resolve(__dirname, "..", ".github", "workflows");

function readWorkflow(name: string): string {
  return fs
    .readFileSync(path.join(WORKFLOWS_DIR, name), "utf8")
    .replace(/\r\n/g, "\n");
}

function readHeaderBefore(source: string, marker: string): string {
  const markerIndex = source.indexOf(marker);
  expect(markerIndex).toBeGreaterThan(-1);
  return source.slice(0, markerIndex).trimEnd();
}

describe("GitHub workflow trigger policy", () => {
  it("keeps one required CI surface for PRs, merge queue, dev/main pushes, and diagnostics", () => {
    const source = readWorkflow("ci.yml");
    const triggerSection = readHeaderBefore(source, "\npermissions:");

    expect(triggerSection).toBe(
      [
        "name: CI",
        "",
        "on:",
        "  pull_request:",
        "    branches: [main]",
        "  push:",
        "    branches: [main, dev/laiyongjie]",
        "  merge_group:",
        "    types: [checks_requested]",
        "  workflow_dispatch:",
      ].join("\n"),
    );
    expect(triggerSection).not.toMatch(/paths(?:-ignore)?:/);
    expect(source).toContain("name: CI / Required");
    expect(source).toContain("if: always()");
    expect(source).toContain(
      "cancel-in-progress: ${{ github.event_name != 'workflow_dispatch' }}",
    );
    expect(source).toContain(
      "format('dispatch-{0}-{1}', github.run_id, github.run_attempt)",
    );
  });

  it("keeps desktop acceptance in the automatic CI path and mock-only boundary", () => {
    const source = readWorkflow("ci.yml");

    expect(source).toContain("desktop-acceptance-contract:");
    expect(source).toContain(
      "run: node --throw-deprecation ./node_modules/vitest/vitest.mjs run tests/desktop-acceptance",
    );
    expect(source).toContain(
      "run: node --throw-deprecation scripts/desktop-acceptance/verify-mock-contract.mjs",
    );
    expect(source).not.toContain("run: pnpm test:desktop:mock");
    expect(source).toContain("run: pnpm test:desktop:visual:preflight");
    expect(source).not.toContain("run: pnpm test:e2e");
  });

  it("uses trusted-base automatic labeling and retains numeric manual replay", () => {
    const source = readWorkflow("labeler.yml");
    const triggerSection = readHeaderBefore(source, "\npermissions:");

    expect(triggerSection).toBe(
      [
        "name: Label PRs",
        "",
        "on:",
        "  pull_request_target:",
        "    branches: [main]",
        "    types: [opened, synchronize, reopened]",
        "  workflow_dispatch:",
        "    inputs:",
        "      pr_number:",
        '        description: "Pull request number to label"',
        "        required: true",
        "        type: number",
      ].join("\n"),
    );
    expect(source).toContain(
      "pr-number: ${{ github.event.pull_request.number || inputs.pr_number }}",
    );
    expect(source).toContain("configuration-path: .github/labeler.yml");
  });

  it("does not execute pull-request code or broaden Labeler permissions", () => {
    const source = readWorkflow("labeler.yml");

    expect(source).toContain("runs-on: macos-15");
    expect(source).toContain(
      "permissions:\n  contents: read\n  pull-requests: write",
    );
    expect(source).not.toMatch(/^\s+issues:/m);
    expect(source).not.toMatch(/^\s+(?:id-token|actions|checks):/m);
    expect(source).not.toContain("actions/checkout");
    expect(source).not.toMatch(/^\s+run:/m);
    expect(source).not.toContain("github.event.pull_request.head");
    expect(source).not.toContain("secrets.");
    expect(source).not.toContain("actions/cache");
    expect(source).not.toContain("upload-artifact");
    const uses = [...source.matchAll(/^\s+uses:\s+(.+)$/gm)].map(
      (match) => match[1],
    );
    expect(uses).toEqual([
      "actions/labeler@bf12e9b00b37c5c0ca2b87b79b2daf7891dbda13 # v7.0.0",
    ]);
  });

  it("offers bug reports only for supported desktop operating systems", () => {
    const source = fs
      .readFileSync(
        path.resolve(WORKFLOWS_DIR, "..", "ISSUE_TEMPLATE", "bug_report.yml"),
        "utf8",
      )
      .replace(/\r\n/g, "\n");

    expect(source).toContain(
      [
        "      label: Operating System / 操作系统",
        "      multiple: false",
        "      options:",
        "        - Windows",
        "        - macOS",
        "    validations:",
        "      required: true",
      ].join("\n"),
    );
  });
});
