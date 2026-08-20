import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  ATTESTATION_BUNDLE_NAME,
  BUILD_METADATA_NAME,
  CI_WORKFLOW_PATH,
  DOWNLOAD_MANIFEST_NAME,
  EXPECTED_INSTALLERS_BY_TARGET,
  EXPECTED_TARGETS,
  PREFLIGHT_BRANCH,
  RELEASE_BRANCH,
  WINDOWS_SIGNING_FRAGMENTS_BY_TARGET,
  WINDOWS_SIGNING_STATUS_NAME,
  assertExactFileSet,
  assertWindowsBundleVersion,
  buildBuildMetadata,
  expectedAttestationSubjectNames,
  expectedInstallerNames,
  expectedReleaseAttachmentNames,
  type ExpectedTarget,
  type PlatformBuildMetadataRecord,
  type ReleaseIdentity,
} from "../scripts/release/release-contract.mjs";
// @ts-expect-error The workflow executes this dependency-free helper directly.
import * as publicationModule from "../scripts/release/prepare-release-publication.mjs";

const temporaryRoots: string[] = [];
const repositoryRoot = path.resolve(__dirname, "..");
const preTransferRepository = ["NongHua123", "fyagent"].join("/");
const collectorScript = path.join(
  repositoryRoot,
  "scripts",
  "release",
  "collect-workflow-artifacts.mjs",
);
const assembleReleaseAttachments =
  publicationModule.assembleReleaseAttachments as (input: {
    subjectsDirectory: string;
    bundlePath: string;
    outputDirectory: string;
    version: string;
  }) => Promise<Array<{ name: string; size: number; sha256: string }>>;
const verifyDownloadedReleaseAttachments =
  publicationModule.verifyDownloadedReleaseAttachments as (input: {
    sourceDirectory: string;
    downloadedDirectory: string;
    version: string;
  }) => Promise<Array<{ name: string; size: number; sha256: string }>>;

const identity: ReleaseIdentity = {
  productVersion: "0.3.0",
  tag: "v0.3.0",
  sourceSha: "b".repeat(40),
  repository: "fy-agent/fyagent",
  repositoryId: "1313497021",
  workflowPath: ".github/workflows/release.yml",
  workflowRef:
    "fy-agent/fyagent/.github/workflows/release.yml@refs/heads/dev/laiyongjie",
  workflowSha: "b".repeat(40),
  runId: "123456",
  runAttempt: "2",
  event: "workflow_dispatch",
  mode: "preflight",
  ciWorkflowPath: CI_WORKFLOW_PATH,
  ciRunId: "987654",
  ciRunAttempt: "3",
};

function temporaryDirectory(): string {
  const root = mkdtempSync(path.join(tmpdir(), "fyagent-release-assets-"));
  temporaryRoots.push(root);
  return root;
}

function platformMetadataRecord(
  expected: ExpectedTarget,
  metadataIdentity: ReleaseIdentity = identity,
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
    identity: metadataIdentity,
  };
}

function writePlatformMetadata(
  directory: string,
  metadataIdentity: ReleaseIdentity = identity,
): void {
  for (const expected of EXPECTED_TARGETS) {
    writeFileSync(
      path.join(directory, `${expected.targetGroup}.json`),
      `${JSON.stringify(platformMetadataRecord(expected, metadataIdentity), null, 2)}\n`,
    );
  }
}

type MutableRecord = Record<string, unknown>;

function mutatePlatformRecord(
  directory: string,
  targetGroup: string,
  mutate: (record: MutableRecord) => void,
): void {
  const metadataPath = path.join(directory, `${targetGroup}.json`);
  const record = JSON.parse(
    readFileSync(metadataPath, "utf8"),
  ) as MutableRecord;
  mutate(record);
  writeFileSync(metadataPath, `${JSON.stringify(record, null, 2)}\n`);
}

function nestedRecord(record: MutableRecord, key: string): MutableRecord {
  return record[key] as MutableRecord;
}

function writeInstallerArtifacts(directory: string): void {
  const installers = expectedInstallerNames("0.3.0");
  for (const { targetGroup } of EXPECTED_TARGETS) {
    const artifact = path.join(directory, `installers-${targetGroup}`);
    mkdirSync(artifact);
    for (const index of EXPECTED_INSTALLERS_BY_TARGET[targetGroup]) {
      writeFileSync(path.join(artifact, installers[index]), installers[index]);
    }
  }
}

function writeMetadataArtifacts(directory: string): void {
  for (const target of EXPECTED_TARGETS) {
    const artifact = path.join(directory, `metadata-${target.targetGroup}`);
    mkdirSync(artifact);
    writeFileSync(
      path.join(artifact, `${target.targetGroup}.json`),
      `${JSON.stringify(platformMetadataRecord(target))}\n`,
    );
  }
}

function writeSigningArtifacts(directory: string): void {
  for (const [targetGroup, fragmentName] of Object.entries(
    WINDOWS_SIGNING_FRAGMENTS_BY_TARGET,
  )) {
    const artifact = path.join(directory, `signing-${targetGroup}`);
    mkdirSync(artifact);
    writeFileSync(path.join(artifact, fragmentName), fragmentName);
  }
}

afterEach(() => {
  while (temporaryRoots.length > 0) {
    rmSync(temporaryRoots.pop()!, { force: true, recursive: true });
  }
});

describe("release asset and metadata contract", () => {
  it("freezes the exact three native runner targets", () => {
    expect(EXPECTED_TARGETS).toEqual([
      {
        targetGroup: "macos-universal",
        platform: "macos",
        architecture: "universal",
        requestedRunnerLabel: "macos-15",
        expectedRunnerOs: "macOS",
        expectedRunnerArch: "ARM64",
      },
      {
        targetGroup: "windows-x64",
        platform: "windows",
        architecture: "x64",
        requestedRunnerLabel: "windows-2025",
        expectedRunnerOs: "Windows",
        expectedRunnerArch: "X64",
      },
      {
        targetGroup: "windows-arm64",
        platform: "windows",
        architecture: "arm64",
        requestedRunnerLabel: "windows-11-arm",
        expectedRunnerOs: "Windows",
        expectedRunnerArch: "ARM64",
      },
    ]);
  });

  it("freezes four installers, seven subjects, and eight attachments", () => {
    const installers = expectedInstallerNames("0.3.0");
    expect(PREFLIGHT_BRANCH).toBe("dev/laiyongjie");
    expect(RELEASE_BRANCH).toBe("main");
    expect(installers).toEqual([
      "FyAgent-0.3.0-macOS.dmg",
      "FyAgent-0.3.0-macOS.zip",
      "FyAgent-0.3.0-Windows-x64-setup.exe",
      "FyAgent-0.3.0-Windows-arm64-setup.exe",
    ]);
    expect(expectedAttestationSubjectNames("0.3.0")).toEqual([
      ...installers,
      DOWNLOAD_MANIFEST_NAME,
      BUILD_METADATA_NAME,
      WINDOWS_SIGNING_STATUS_NAME,
    ]);
    expect(expectedAttestationSubjectNames("0.3.0")).toHaveLength(7);
    expect(expectedReleaseAttachmentNames("0.3.0")).toEqual([
      ...expectedAttestationSubjectNames("0.3.0"),
      ATTESTATION_BUNDLE_NAME,
    ]);
    expect(expectedReleaseAttachmentNames("0.3.0")).toHaveLength(8);
  });

  it("fails closed when a canonical version cannot fit NSIS fixed-file fields", () => {
    expect(() => assertWindowsBundleVersion("65535.65535.65535")).not.toThrow();
    expect(() => assertWindowsBundleVersion("65536.0.0")).toThrow(
      /Windows NSIS version components must be between 0 and 65535/u,
    );
    expect(() => assertWindowsBundleVersion("9007199254740993.0.0")).toThrow(
      /Windows NSIS version components must be between 0 and 65535/u,
    );
  });

  it("rejects directories, missing names, and unapproved ancillary files", () => {
    const directory = temporaryDirectory();
    for (const name of expectedInstallerNames("0.3.0")) {
      writeFileSync(path.join(directory, name), name);
    }
    mkdirSync(path.join(directory, "nested"));
    expect(() =>
      assertExactFileSet(
        directory,
        expectedInstallerNames("0.3.0"),
        "installers",
      ),
    ).toThrow(/Only regular files are allowed/);
  });

  it("collects three isolated installer artifacts without allowing overwrite", () => {
    const root = temporaryDirectory();
    const downloads = path.join(root, "downloads");
    const output = path.join(root, "installers");
    mkdirSync(downloads);
    writeInstallerArtifacts(downloads);
    execFileSync(
      process.execPath,
      [collectorScript, "installers", downloads, output, "0.3.0"],
      { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
    );
    expect(readdirSync(output).sort()).toEqual(
      expectedInstallerNames("0.3.0").sort(),
    );
  });

  it("collects three isolated metadata artifacts", () => {
    const root = temporaryDirectory();
    const downloads = path.join(root, "downloads");
    const output = path.join(root, "metadata");
    mkdirSync(downloads);
    writeMetadataArtifacts(downloads);
    execFileSync(
      process.execPath,
      [collectorScript, "metadata", downloads, output, "0.3.0"],
      { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
    );
    expect(readdirSync(output).sort()).toEqual(
      EXPECTED_TARGETS.map(({ targetGroup }) => `${targetGroup}.json`).sort(),
    );
  });

  it("collects exactly two private Windows signing fragments", () => {
    const root = temporaryDirectory();
    const downloads = path.join(root, "downloads");
    const output = path.join(root, "signing-fragments");
    mkdirSync(downloads);
    writeSigningArtifacts(downloads);
    execFileSync(
      process.execPath,
      [collectorScript, "signing", downloads, output, "0.3.0"],
      { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
    );
    expect(readdirSync(output).sort()).toEqual(
      Object.values(WINDOWS_SIGNING_FRAGMENTS_BY_TARGET).sort(),
    );
  });

  it("rejects extra signing fragments before aggregation", () => {
    const root = temporaryDirectory();
    const downloads = path.join(root, "downloads");
    mkdirSync(downloads);
    writeSigningArtifacts(downloads);
    writeFileSync(
      path.join(downloads, "signing-windows-x64", "unexpected.json"),
      "unexpected",
    );
    expect(() =>
      execFileSync(
        process.execPath,
        [
          collectorScript,
          "signing",
          downloads,
          path.join(root, "signing-fragments"),
          "0.3.0",
        ],
        { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
      ),
    ).toThrow(/signing-windows-x64 artifact must contain exactly 1 files/);
  });

  it("rejects duplicate or misplaced installers before flattening artifacts", () => {
    const root = temporaryDirectory();
    const downloads = path.join(root, "downloads");
    mkdirSync(downloads);
    writeInstallerArtifacts(downloads);
    writeFileSync(
      path.join(downloads, "installers-windows-x64", "FyAgent-0.3.0-macOS.dmg"),
      "duplicate",
    );
    expect(() =>
      execFileSync(
        process.execPath,
        [
          collectorScript,
          "installers",
          downloads,
          path.join(root, "installers"),
          "0.3.0",
        ],
        { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
      ),
    ).toThrow(/installers-windows-x64 artifact must contain exactly 1 files/);
  });

  it("aggregates exactly three identity-bound platform records", () => {
    const directory = temporaryDirectory();
    writePlatformMetadata(directory);
    const metadata = buildBuildMetadata({
      metadataDirectory: directory,
      identity,
      generatedAt: "2026-08-08T00:00:00.000Z",
    });
    expect(metadata).toMatchObject({
      schema: "fyagent-build-metadata/v2",
      product: "FyAgent",
      version: "0.3.0",
      tag: "v0.3.0",
      sourceSha: "b".repeat(40),
      repository: {
        nameWithOwner: "fy-agent/fyagent",
        id: "1313497021",
      },
      workflow: {
        path: ".github/workflows/release.yml",
        runId: "123456",
        runAttempt: "2",
        event: "workflow_dispatch",
        mode: "preflight",
        ref: "fy-agent/fyagent/.github/workflows/release.yml@refs/heads/dev/laiyongjie",
        sha: "b".repeat(40),
      },
      requiredCi: {
        path: ".github/workflows/ci.yml",
        runId: "987654",
        runAttempt: "3",
        job: "CI / Required",
        conclusion: "success",
      },
    });
    expect(metadata.targets).toHaveLength(3);
    expect(metadata.targets.map(({ targetGroup }) => targetGroup)).toEqual(
      EXPECTED_TARGETS.map(({ targetGroup }) => targetGroup),
    );
    for (const target of metadata.targets) {
      expect(Object.keys(target).sort()).toEqual(
        [
          "schema",
          "targetGroup",
          "platform",
          "architecture",
          "runner",
          "toolchain",
        ].sort(),
      );
      expect("identity" in target).toBe(false);
    }
  });

  it("records the exact Required CI binding for preflight and formal metadata", () => {
    const formalIdentity: ReleaseIdentity = {
      ...identity,
      workflowRef:
        "fy-agent/fyagent/.github/workflows/release.yml@refs/tags/v0.3.0",
      event: "push",
      mode: "formal",
    };
    for (const candidate of [identity, formalIdentity]) {
      const directory = temporaryDirectory();
      writePlatformMetadata(directory, candidate);
      expect(
        buildBuildMetadata({
          metadataDirectory: directory,
          identity: candidate,
          generatedAt: "2026-08-08T00:00:00.000Z",
        }).requiredCi,
      ).toEqual({
        path: ".github/workflows/ci.yml",
        runId: "987654",
        runAttempt: "3",
        job: "CI / Required",
        conclusion: "success",
      });
    }
  });

  it("accepts another canonical stable version and binds its formal tag ref", () => {
    const generalizedIdentity: ReleaseIdentity = {
      ...identity,
      productVersion: "12.34.56",
      tag: "v12.34.56",
      workflowRef:
        "fy-agent/fyagent/.github/workflows/release.yml@refs/tags/v12.34.56",
      event: "push",
      mode: "formal",
    };
    const directory = temporaryDirectory();
    writePlatformMetadata(directory, generalizedIdentity);
    const metadata = buildBuildMetadata({
      metadataDirectory: directory,
      identity: generalizedIdentity,
      generatedAt: "2026-08-08T00:00:00.000Z",
    });
    expect(metadata.version).toBe("12.34.56");
    expect(metadata.tag).toBe("v12.34.56");
    expect(expectedInstallerNames(metadata.version)).toHaveLength(4);
  });

  it.each([
    [
      "repository",
      { repository: preTransferRepository },
      /Repository identity drifted/,
    ],
    ["repository id", { repositoryId: "42" }, /Repository ID drifted/],
    [
      "workflow",
      { workflowPath: ".github/workflows/other.yml" },
      /workflow path drifted/,
    ],
    [
      "preflight workflow ref",
      {
        workflowRef:
          "fy-agent/fyagent/.github/workflows/release.yml@refs/heads/other",
      },
      /Preflight must use the trusted dev\/laiyongjie workflow ref/,
    ],
    [
      "CI workflow",
      { ciWorkflowPath: "other.yml" },
      /CI workflow path drifted/,
    ],
    ["CI run", { ciRunId: "0" }, /ciRunId must be numeric/],
    ["source SHA", { sourceSha: "c".repeat(39) }, /full 40-character/],
  ])("rejects %s identity drift", (_label, change, error) => {
    const directory = temporaryDirectory();
    writePlatformMetadata(directory);
    expect(() =>
      buildBuildMetadata({
        metadataDirectory: directory,
        identity: { ...identity, ...change },
        generatedAt: "2026-08-08T00:00:00.000Z",
      }),
    ).toThrow(error);
  });

  it.each([
    ["record root", (record: MutableRecord) => (record.unexpected = true)],
    [
      "runner",
      (record: MutableRecord) =>
        (nestedRecord(record, "runner").unexpected = true),
    ],
    [
      "runner context",
      (record: MutableRecord) =>
        (nestedRecord(nestedRecord(record, "runner"), "context").unexpected =
          true),
    ],
    [
      "toolchain",
      (record: MutableRecord) =>
        (nestedRecord(record, "toolchain").unexpected = true),
    ],
    [
      "identity",
      (record: MutableRecord) =>
        (nestedRecord(record, "identity").unexpected = true),
    ],
  ])("rejects unknown keys at the %s level", (_label, mutate) => {
    const directory = temporaryDirectory();
    writePlatformMetadata(directory);
    mutatePlatformRecord(directory, "windows-x64", mutate);
    expect(() =>
      buildBuildMetadata({
        metadataDirectory: directory,
        identity,
        generatedAt: "2026-08-08T00:00:00.000Z",
      }),
    ).toThrow(/must contain exactly these keys/);
  });

  it.each([
    [
      "runner OS drift",
      (record: MutableRecord) =>
        (nestedRecord(nestedRecord(record, "runner"), "context").os = "macOS"),
      /runner context OS drifted/,
    ],
    [
      "runner architecture drift",
      (record: MutableRecord) =>
        (nestedRecord(nestedRecord(record, "runner"), "context").arch =
          "ARM64"),
      /runner context architecture drifted/,
    ],
    [
      "requested runner label drift",
      (record: MutableRecord) =>
        (nestedRecord(record, "runner").requestedLabel = "macos-15"),
      /requested runner label drifted/,
    ],
    [
      "old schema",
      (record: MutableRecord) =>
        (record.schema = ["fyagent-platform-build", "v1"].join("/")),
      /Invalid platform metadata schema/,
    ],
    [
      "tool version drift",
      (record: MutableRecord) =>
        (nestedRecord(record, "toolchain").node = "v0.0.0"),
      /Node version drifted/,
    ],
  ])("rejects %s", (_label, mutate, error) => {
    const directory = temporaryDirectory();
    writePlatformMetadata(directory);
    mutatePlatformRecord(directory, "windows-x64", mutate);
    expect(() =>
      buildBuildMetadata({
        metadataDirectory: directory,
        identity,
        generatedAt: "2026-08-08T00:00:00.000Z",
      }),
    ).toThrow(error);
  });

  it("assembles exactly eight attachments and verifies re-downloaded bytes", async () => {
    const root = temporaryDirectory();
    const subjects = path.join(root, "subjects");
    const attachments = path.join(root, "attachments");
    const downloaded = path.join(root, "downloaded");
    const bundle = path.join(root, ATTESTATION_BUNDLE_NAME);
    mkdirSync(subjects);
    mkdirSync(downloaded);
    for (const name of expectedAttestationSubjectNames("0.3.0")) {
      writeFileSync(path.join(subjects, name), `subject:${name}`);
    }
    writeFileSync(bundle, "sigstore-bundle");

    const described = await assembleReleaseAttachments({
      subjectsDirectory: subjects,
      bundlePath: bundle,
      outputDirectory: attachments,
      version: "0.3.0",
    });
    expect(described.map(({ name }) => name)).toEqual(
      expectedReleaseAttachmentNames("0.3.0"),
    );
    expect(described).toHaveLength(8);

    for (const name of expectedReleaseAttachmentNames("0.3.0")) {
      copyFileSync(path.join(attachments, name), path.join(downloaded, name));
    }
    await expect(
      verifyDownloadedReleaseAttachments({
        sourceDirectory: attachments,
        downloadedDirectory: downloaded,
        version: "0.3.0",
      }),
    ).resolves.toHaveLength(8);

    writeFileSync(
      path.join(downloaded, expectedInstallerNames("0.3.0")[0]),
      "drift",
    );
    await expect(
      verifyDownloadedReleaseAttachments({
        sourceDirectory: attachments,
        downloadedDirectory: downloaded,
        version: "0.3.0",
      }),
    ).rejects.toThrow(/differ from the verified local payload/);
  });
});
