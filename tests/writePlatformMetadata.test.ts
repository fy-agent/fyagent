import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  EXPECTED_TARGETS,
  buildBuildMetadata,
  type ExpectedTarget,
  type PlatformBuildMetadataRecord,
  type ReleaseIdentity,
} from "../scripts/release/release-contract.mjs";

const repositoryRoot = path.resolve(__dirname, "..");
const writerPath = path.join(
  repositoryRoot,
  "scripts",
  "release",
  "write-platform-metadata.mjs",
);
const sourceSha = "b".repeat(40);
const temporaryRoots: string[] = [];

function temporaryDirectory(): string {
  const root = mkdtempSync(path.join(tmpdir(), "fyagent-platform-metadata-"));
  temporaryRoots.push(root);
  return root;
}

function releaseIdentity(mode: "preflight" | "formal"): ReleaseIdentity {
  return {
    productVersion: "0.3.0",
    tag: "v0.3.0",
    sourceSha,
    repository: "fy-agent/fyagent",
    repositoryId: "1313497021",
    workflowPath: ".github/workflows/release.yml",
    workflowRef:
      mode === "formal"
        ? "fy-agent/fyagent/.github/workflows/release.yml@refs/tags/v0.3.0"
        : "fy-agent/fyagent/.github/workflows/release.yml@refs/heads/dev/laiyongjie",
    workflowSha: sourceSha,
    runId: "123456",
    runAttempt: "2",
    event: mode === "formal" ? "push" : "workflow_dispatch",
    mode,
    ciWorkflowPath: ".github/workflows/ci.yml",
    ciRunId: "987654",
    ciRunAttempt: "3",
  };
}

function writerEnvironment(
  expected: ExpectedTarget,
  mode: "preflight" | "formal" = "preflight",
): NodeJS.ProcessEnv {
  const identity = releaseIdentity(mode);
  return {
    TARGET_GROUP: expected.targetGroup,
    TARGET_PLATFORM: expected.platform,
    TARGET_ARCHITECTURE: expected.architecture,
    REQUESTED_RUNNER_LABEL: expected.requestedRunnerLabel,
    ACTUAL_RUNNER_OS: expected.expectedRunnerOs,
    ACTUAL_RUNNER_ARCH: expected.expectedRunnerArch,
    ACTUAL_NODE_VERSION: "v24.19.0",
    ACTUAL_PNPM_VERSION: "10.12.3",
    ACTUAL_RUST_VERSION: "rustc 1.97.1 (reviewed 2026-08-08)",
    APP_VERSION: identity.productVersion,
    RELEASE_TAG: identity.tag,
    SOURCE_SHA: identity.sourceSha,
    GITHUB_REPOSITORY: identity.repository,
    GITHUB_REPOSITORY_ID: identity.repositoryId,
    GITHUB_WORKFLOW_REF: identity.workflowRef,
    GITHUB_WORKFLOW_SHA: identity.workflowSha,
    GITHUB_RUN_ID: identity.runId,
    GITHUB_RUN_ATTEMPT: identity.runAttempt,
    GITHUB_EVENT_NAME: identity.event,
    RELEASE_MODE: mode,
    EXPECTED_CI_RUN_ID: identity.ciRunId,
    EXPECTED_CI_RUN_ATTEMPT: identity.ciRunAttempt,
  };
}

function expectedRecord(
  expected: ExpectedTarget,
  mode: "preflight" | "formal" = "preflight",
): PlatformBuildMetadataRecord {
  return {
    schema: "fyagent-platform-build/v2",
    targetGroup: expected.targetGroup,
    platform: expected.platform,
    architecture: expected.architecture,
    runner: {
      requestedLabel: expected.requestedRunnerLabel,
      context: {
        os: expected.expectedRunnerOs,
        arch: expected.expectedRunnerArch,
      },
    },
    toolchain: {
      node: "v24.19.0",
      pnpm: "10.12.3",
      rustc: "rustc 1.97.1 (reviewed 2026-08-08)",
    },
    identity: releaseIdentity(mode),
  };
}

function invokeWriter(
  expected: ExpectedTarget,
  options: {
    mode?: "preflight" | "formal";
    mutateEnvironment?: (environment: NodeJS.ProcessEnv) => void;
    outputPath?: string;
  } = {},
) {
  const mode = options.mode ?? "preflight";
  const outputPath =
    options.outputPath ?? path.join(temporaryDirectory(), "platform.json");
  const environment = writerEnvironment(expected, mode);
  options.mutateEnvironment?.(environment);
  const result = spawnSync(process.execPath, [writerPath, outputPath], {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: environment,
  });
  return { outputPath, result };
}

function expectWriterFailure(
  expected: ExpectedTarget,
  mutateEnvironment: (environment: NodeJS.ProcessEnv) => void,
  error: RegExp,
  mode: "preflight" | "formal" = "preflight",
): void {
  const { result } = invokeWriter(expected, { mode, mutateEnvironment });
  expect(result.status).not.toBe(0);
  expect(result.stderr).toMatch(error);
}

function setEnvironmentVariable(
  name: string,
  value: string,
): (environment: NodeJS.ProcessEnv) => void {
  return (environment) => {
    environment[name] = value;
  };
}

afterEach(() => {
  while (temporaryRoots.length > 0) {
    rmSync(temporaryRoots.pop()!, { force: true, recursive: true });
  }
});

describe("write-platform-metadata CLI", () => {
  for (const expected of EXPECTED_TARGETS) {
    it(`writes the exact source-explicit ${expected.targetGroup} record`, () => {
      const { outputPath, result } = invokeWriter(expected);
      expect(result.status, result.stderr).toBe(0);
      expect(JSON.parse(readFileSync(outputPath, "utf8"))).toEqual(
        expectedRecord(expected),
      );
    });
  }

  it.each([
    "TARGET_GROUP",
    "TARGET_PLATFORM",
    "TARGET_ARCHITECTURE",
    "REQUESTED_RUNNER_LABEL",
    "ACTUAL_RUNNER_OS",
    "ACTUAL_RUNNER_ARCH",
    "ACTUAL_NODE_VERSION",
    "ACTUAL_PNPM_VERSION",
    "ACTUAL_RUST_VERSION",
    "EXPECTED_CI_RUN_ID",
    "EXPECTED_CI_RUN_ATTEMPT",
  ])("rejects a missing required %s input", (variable) => {
    expectWriterFailure(
      EXPECTED_TARGETS[0],
      (environment) => delete environment[variable],
      new RegExp(variable),
    );
  });

  it.each([
    [
      "unknown target",
      setEnvironmentVariable("TARGET_GROUP", "freebsd-x64"),
      /Unsupported target group/,
    ],
    [
      "platform contradiction",
      setEnvironmentVariable("TARGET_PLATFORM", "windows"),
      /TARGET_PLATFORM/,
    ],
    [
      "architecture contradiction",
      setEnvironmentVariable("TARGET_ARCHITECTURE", "arm64"),
      /TARGET_ARCHITECTURE/,
    ],
    [
      "requested-label contradiction",
      setEnvironmentVariable("REQUESTED_RUNNER_LABEL", "windows-11-arm"),
      /REQUESTED_RUNNER_LABEL/,
    ],
    [
      "runner OS contradiction",
      setEnvironmentVariable("ACTUAL_RUNNER_OS", "Windows"),
      /ACTUAL_RUNNER_OS/,
    ],
    [
      "runner architecture contradiction",
      setEnvironmentVariable("ACTUAL_RUNNER_ARCH", "X64"),
      /ACTUAL_RUNNER_ARCH/,
    ],
    [
      "undocumented runner architecture",
      setEnvironmentVariable("ACTUAL_RUNNER_ARCH", "UNIVERSAL"),
      /documented GitHub runner architecture/,
    ],
  ] as const)("rejects %s", (_label, mutateEnvironment, error) => {
    expectWriterFailure(EXPECTED_TARGETS[0], mutateEnvironment, error);
  });

  it.each([
    ["macos-universal", EXPECTED_TARGETS[0], "X64"],
    ["windows-x64", EXPECTED_TARGETS[1], "ARM64"],
    ["windows-arm64", EXPECTED_TARGETS[2], "X64"],
  ])(
    "rejects %s paired with the wrong runner architecture",
    (_label, expected, runnerArch) => {
      expectWriterFailure(
        expected,
        (environment) => (environment.ACTUAL_RUNNER_ARCH = runnerArch),
        /ACTUAL_RUNNER_ARCH/,
      );
    },
  );

  it("preserves formal Required-CI identity", () => {
    const expected = EXPECTED_TARGETS[1];
    const { outputPath, result } = invokeWriter(expected, { mode: "formal" });
    expect(result.status, result.stderr).toBe(0);
    expect(JSON.parse(readFileSync(outputPath, "utf8"))).toEqual(
      expectedRecord(expected, "formal"),
    );
  });

  it.each([
    ["preflight", "EXPECTED_CI_RUN_ID", "0"],
    ["preflight", "EXPECTED_CI_RUN_ATTEMPT", "attempt-3"],
    ["formal", "EXPECTED_CI_RUN_ID", "-1"],
    ["formal", "EXPECTED_CI_RUN_ATTEMPT", "3.0"],
  ] as const)(
    "rejects %s metadata with invalid %s",
    (mode, variable, value) => {
      expectWriterFailure(
        EXPECTED_TARGETS[1],
        (environment) => (environment[variable] = value),
        new RegExp(`${variable} must be a positive decimal integer`),
        mode,
      );
    },
  );

  it("does not replace an existing output file", () => {
    const root = temporaryDirectory();
    const outputPath = path.join(root, "platform.json");
    writeFileSync(outputPath, "preserve-me\n");
    const { result } = invokeWriter(EXPECTED_TARGETS[1], { outputPath });
    expect(result.status).not.toBe(0);
    expect(result.stderr).toMatch(/EEXIST|file already exists/i);
    expect(readFileSync(outputPath, "utf8")).toBe("preserve-me\n");
  });

  it("ignores hostile ambient hosted-runner variables", () => {
    const expected = EXPECTED_TARGETS[2];
    const { outputPath, result } = invokeWriter(expected, {
      mutateEnvironment: (environment) => {
        environment.RUNNER_OS = "macOS";
        environment.RUNNER_ARCH = "X64";
        environment.ImageOS = "host-poison";
        environment.ImageVersion = "host-version-poison";
      },
    });
    expect(result.status, result.stderr).toBe(0);
    expect(JSON.parse(readFileSync(outputPath, "utf8"))).toEqual(
      expectedRecord(expected),
    );
    expect(readFileSync(outputPath, "utf8")).not.toContain("poison");
  });

  it("feeds all three writer records into the canonical aggregate", () => {
    const metadataDirectory = temporaryDirectory();
    for (const expected of EXPECTED_TARGETS) {
      const outputPath = path.join(
        metadataDirectory,
        `${expected.targetGroup}.json`,
      );
      const { result } = invokeWriter(expected, { outputPath });
      expect(result.status, result.stderr).toBe(0);
    }

    const metadata = buildBuildMetadata({
      metadataDirectory,
      identity: releaseIdentity("preflight"),
      generatedAt: "2026-08-08T00:00:00.000Z",
    });
    expect(metadata.schema).toBe("fyagent-build-metadata/v2");
    expect(metadata.targets).toHaveLength(3);
    expect(metadata.targets).toEqual(
      EXPECTED_TARGETS.map((expected) => {
        const { identity: _identity, ...target } = expectedRecord(expected);
        return target;
      }),
    );
  });
});
