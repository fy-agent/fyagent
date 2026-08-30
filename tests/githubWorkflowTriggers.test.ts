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
  it("keeps Required CI on PR, merge queue, and explicit diagnostics only", () => {
    const source = readWorkflow("ci.yml");
    const triggerSection = readHeaderBefore(source, "\npermissions:");

    expect(triggerSection).toBe(
      [
        "name: CI",
        "",
        "on:",
        "  pull_request:",
        "  merge_group:",
        "    types: [checks_requested]",
        "  workflow_dispatch:",
      ].join("\n"),
    );
    expect(triggerSection).not.toMatch(/paths(?:-ignore)?:/);
    expect(source).toContain("name: CI / Required");
    expect(source).toContain("if: always()");
    expect(source).not.toMatch(/^  push:/m);
    expect(source).toContain(
      "cancel-in-progress: ${{ github.event_name != 'workflow_dispatch' }}",
    );
    expect(source).toContain(
      "group: ci-${{ github.workflow }}-${{ github.event_name }}-",
    );
    expect(source).toContain(
      "format('dispatch-{0}-{1}', github.run_id, github.run_attempt)",
    );
  });

  it("keeps branch-push commit policy lightweight and excludes merge-queue refs", () => {
    const source = readWorkflow("commit-convention-push.yml");
    const triggerSection = readHeaderBefore(source, "\npermissions:");

    expect(triggerSection).toBe(
      [
        "name: Commit Convention / Push",
        "",
        "on:",
        "  push:",
        "    branches-ignore:",
        '      - "gh-readonly-queue/**"',
      ].join("\n"),
    );
    expect(source).toContain("name: Commit Convention / Push");
    expect(source).toContain("node scripts/ci/verify-commit-messages.mjs");
    expect(source).toContain("PUSH_BASE_SHA: ${{ github.event.before }}");
    expect(source).toContain("PUSH_HEAD_SHA: ${{ github.sha }}");
    expect(source).toContain(
      "Push before SHA is not a commit in this clone; using head as an empty comparison",
    );
    expect(source).not.toContain("CI / Required");
    expect(source).not.toContain("classify-changes.mjs");
    expect(source).not.toContain("pnpm install");
    expect(source).not.toContain("cargo ");
    expect(source).toContain("runs-on: ubuntu-24.04");
  });

  it("keeps the branch-push policy read-only and pins its actions", () => {
    const source = readWorkflow("commit-convention-push.yml");

    expect(source).toContain("permissions:\n  contents: read");
    expect(source).not.toMatch(
      /^\s+(?:actions|checks|pull-requests|id-token):\s+write/m,
    );
    expect(source).not.toContain("secrets.");
    expect(source).toContain(
      "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1",
    );
    expect(source).toContain(
      "actions/setup-node@820762786026740c76f36085b0efc47a31fe5020 # v7.0.0",
    );
  });

  it("keeps desktop acceptance in the automatic CI path and mock-only boundary", () => {
    const source = readWorkflow("ci.yml");

    expect(source).toContain("commit-convention:");
    expect(source).toContain("name: Commit Convention");
    expect(source).toContain("node scripts/ci/verify-commit-messages.mjs");
    expect(source).toContain("needs: commit-convention");
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

    expect(source).toContain("runs-on: ubuntu-24.04");
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

  it("updates Star History through the public self-hosted action and a dedicated data branch", () => {
    const source = readWorkflow("star-history.yml");
    const triggerSection = readHeaderBefore(source, "\npermissions:");

    expect(triggerSection).toBe(
      [
        "name: Star History",
        "",
        "on:",
        "  schedule:",
        '    - cron: "17 */3 * * *"',
        "  workflow_dispatch:",
        "  watch:",
        "    types: [started]",
      ].join("\n"),
    );
    expect(source).toContain("permissions:\n  contents: read");
    expect(source).toContain(
      "update:\n    name: Update charts\n    runs-on: ubuntu-24.04",
    );
    expect(source).toContain(
      "update:\n    name: Update charts\n    runs-on: ubuntu-24.04\n    timeout-minutes: 10\n    permissions:\n      contents: write",
    );
    expect(source).toContain("branch: star-history");
    expect(source).toContain(
      "token: ${{ secrets.STAR_HISTORY_TOKEN || github.token }}",
    );
    expect(source).toContain(
      "uses: xpzouying/star-history@8b1f26dc5e9a17caa75da9351b688509ef312811",
    );
    expect(source).not.toMatch(/HEAD:refs\/heads\/main/);
    expect(source).not.toContain("actions/upload-artifact");
    expect(source).not.toContain("STAR_HISTORY_SOURCE_SHA");
    expect(source).not.toContain("go run ./cmd/star-history");

    const uses = [...source.matchAll(/^\s+uses:\s+(.+)$/gm)].map(
      (match) => match[1],
    );
    expect(uses).toEqual([
      "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1",
      "xpzouying/star-history@8b1f26dc5e9a17caa75da9351b688509ef312811",
    ]);
  });

  it("keeps frontend labeling scoped to frontend-owned tests", () => {
    const source = fs
      .readFileSync(path.resolve(WORKFLOWS_DIR, "..", "labeler.yml"), "utf8")
      .replace(/\r\n/g, "\n");

    expect(source).not.toContain('          - "tests/**"');
    for (const frontendTests of [
      "tests/components/**",
      "tests/config/**",
      "tests/hooks/**",
      "tests/integration/**",
      "tests/lib/**",
      "tests/msw/**",
      "tests/utils/**",
      "tests/e2e/**",
    ]) {
      expect(source).toContain(`          - "${frontendTests}"`);
    }
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
