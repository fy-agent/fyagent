import { execFileSync, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  EXPECTED_REPOSITORY_ID,
  EXPECTED_TARGETS,
  buildBuildMetadata,
  type ReleaseIdentity,
} from "../scripts/release/release-contract.mjs";
import {
  WORKFLOW_ONLY_PATHS,
  assertAdmittedWindowsMetadata,
  assertProductJobsPreserved,
  assertPublishedAdmission,
  bindFormalSigningStatus,
  classifyWindowsByteSource,
  productJobFingerprint,
  productTreeHash,
  proveFormalOfflineAdmission,
  readFormalOfflineAdmissionManifest,
  type FormalOfflineAdmissionInput,
  type FormalOfflineAdmissionProof,
  stageMetadataForAggregate,
} from "../scripts/release/windows-formal-offline-admission.mjs";

const ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const ARTIFACT_SOURCE = "4f5e0543bae400e0c4447c10c164cd7e285f01df";
const RELEASE_SOURCE = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PRODUCT_JOB_FINGERPRINT =
  "e425a5d64add60e956a7264fc7569a85c0beced4b229d8e1f23070af8dd27dd4";
const BLOB = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

function sha256(bytes: Buffer): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function workflowYaml(releaseGate: boolean): string {
  const windowsIf = releaseGate
    ? "\n    if: needs.eligibility.outputs.windows_byte_source != 'offline-sealed'"
    : "";
  const formalIf = releaseGate
    ? "needs.eligibility.outputs.release_mode == 'formal' && needs.eligibility.outputs.windows_byte_source != 'offline-sealed' && (github.event_name == 'push' || github.event_name == 'workflow_dispatch')"
    : "needs.eligibility.outputs.release_mode == 'formal' && (github.event_name == 'push' || github.event_name == 'workflow_dispatch')";
  return `name: Release

jobs:
  build-windows:${windowsIf}
    runs-on: windows-2025
  prove-windows-preflight:
    runs-on: windows-2025
  sign-windows-formal:
    if: ${formalIf}
    runs-on: windows-2025
  seal-windows-formal:
    if: ${formalIf}
    runs-on: windows-2025
  build-macos:
    runs-on: macos-15
  tail:
    runs-on: ubuntu-24.04
`;
}

function tree(extra = ""): string {
  return `100644 blob ${BLOB}\tsrc/main.rs\n100644 blob ${"b".repeat(40)}\t.github/workflows/release.yml\n${extra}`;
}

function run(overrides: Record<string, unknown> = {}) {
  return {
    id: 42421033643,
    run_attempt: 1,
    head_sha: ARTIFACT_SOURCE,
    status: "completed",
    conclusion: "success",
    event: "workflow_dispatch",
    name: "Release",
    path: ".github/workflows/release.yml",
    repository: { id: Number(EXPECTED_REPOSITORY_ID) },
    ...overrides,
  };
}

function offlineRun(overrides: Record<string, unknown> = {}) {
  return run({
    id: 42424610543,
    name: "Windows offline sign return",
    path: ".github/workflows/windows-offline-sign-return.yml",
    ...overrides,
  });
}

function fixture() {
  const rawX64 = Buffer.from("raw-x64-installer");
  const rawArm64 = Buffer.from("raw-arm64-installer");
  const signedX64 = Buffer.from("signed-x64-installer");
  const signedArm64 = Buffer.from("signed-arm64-installer");
  const assets = [
    {
      architecture: "x64",
      name: "FyAgent-0.4.15-Windows-x64-setup.exe",
      rawSha256: sha256(rawX64),
      signedSha256: sha256(signedX64),
    },
    {
      architecture: "arm64",
      name: "FyAgent-0.4.15-Windows-arm64-setup.exe",
      rawSha256: sha256(rawArm64),
      signedSha256: sha256(signedArm64),
    },
  ];
  const listing = tree();
  const manifest = {
    schema: "fyagent-windows-formal-offline-admission/v1",
    version: "0.4.15",
    artifactSourceSha: ARTIFACT_SOURCE,
    productTreeHash: productTreeHash(listing),
    preflightRunId: "42421033643",
    preflightRunAttempt: "1",
    offlineRunId: "42424610543",
    offlineRunAttempt: "1",
    assets,
  };
  const policy = {
    schema: "fyagent-windows-offline-signing-policy/v1",
    publisher: "Hangzhou Fuyu Talent Service Co., Ltd.",
    certificateSha256: "c".repeat(64),
  };
  const binding = {
    schema: "fyagent-windows-offline-sealed-binding/v1",
    product: "FyAgent",
    sourceRunId: manifest.preflightRunId,
    sourceRunAttempt: manifest.preflightRunAttempt,
    sourceSha: ARTIFACT_SOURCE,
    version: "0.4.15",
    repositoryId: EXPECTED_REPOSITORY_ID,
    workflow: ".github/workflows/release.yml",
    publisher: policy.publisher,
    certificateSha256: policy.certificateSha256,
    assets,
  };
  const input = {
    manifest,
    policy,
    releaseSha: RELEASE_SOURCE,
    ancestor: true,
    artifactLsTree: listing,
    releaseLsTree: listing,
    diffStatus: "M\t.github/workflows/release.yml\n",
    artifactWorkflow: workflowYaml(false),
    releaseWorkflow: workflowYaml(true),
    preflightRun: run(),
    offlineRun: offlineRun(),
    binding,
    rawBytesByArchitecture: { x64: rawX64, arm64: rawArm64 },
    signedBytesByArchitecture: { x64: signedX64, arm64: signedArm64 },
  };
  return {
    input: input satisfies FormalOfflineAdmissionInput,
    rawX64,
    rawArm64,
    signedX64,
    signedArm64,
    policy,
  };
}

function prove(change: (input: FormalOfflineAdmissionInput) => void = () => {}) {
  const { input } = fixture();
  change(input);
  return proveFormalOfflineAdmission(input);
}

function formalReleaseIdentity(proof: FormalOfflineAdmissionProof): ReleaseIdentity {
  return {
    productVersion: proof.version,
    tag: `v${proof.version}`,
    sourceSha: proof.releaseSha,
    repository: "fy-agent/fyagent",
    repositoryId: EXPECTED_REPOSITORY_ID,
    workflowPath: ".github/workflows/release.yml",
    workflowRef:
      `fy-agent/fyagent/.github/workflows/release.yml@refs/tags/v${proof.version}`,
    workflowSha: proof.releaseSha,
    runId: "99",
    runAttempt: "1",
    event: "push",
    mode: "formal",
    ciWorkflowPath: ".github/workflows/ci.yml",
    ciRunId: null,
    ciRunAttempt: null,
  };
}

function writeAdmittedPlatformMetadata(
  directory: string,
  proof: FormalOfflineAdmissionProof,
  releaseIdentity: ReleaseIdentity,
) {
  for (const target of EXPECTED_TARGETS) {
    const identity =
      target.platform === "windows"
        ? {
            productVersion: proof.version,
            tag: `v${proof.version}`,
            sourceSha: proof.artifactSourceSha,
            repository: "fy-agent/fyagent",
            repositoryId: EXPECTED_REPOSITORY_ID,
            workflowPath: ".github/workflows/release.yml",
            workflowRef:
              "fy-agent/fyagent/.github/workflows/release.yml@refs/heads/main",
            workflowSha: proof.artifactSourceSha,
            runId: proof.preflightRunId,
            runAttempt: proof.preflightRunAttempt,
            event: "workflow_dispatch",
            mode: "preflight",
            ciWorkflowPath: ".github/workflows/ci.yml",
            ciRunId: null,
            ciRunAttempt: null,
          }
        : releaseIdentity;
    writeFileSync(
      path.join(directory, `${target.targetGroup}.json`),
      `${JSON.stringify({
        schema: "fyagent-platform-build/v3",
        targetGroup: target.targetGroup,
        platform: target.platform,
        architecture: target.architecture,
        runner: {
          requestedLabel: target.requestedRunnerLabel,
          context: {
            os: target.expectedRunnerOs,
            arch: target.expectedRunnerArch,
          },
        },
        toolchain: {
          node: "v24.19.0",
          pnpm: "10.12.3",
          rustc: "rustc 1.97.1 (fixture)",
        },
        nativeToolchain:
          target.platform === "windows"
            ? { visualStudio: "18.11.12328.437", msvc: "14.51.36231" }
            : null,
        identity,
      })}\n`,
    );
  }
}

describe("formal offline Windows admission", () => {
  it("pins v0.4.15 to the sealed preflight and keeps the application tree", () => {
    const manifest = readFormalOfflineAdmissionManifest(
      JSON.parse(
        readFileSync(
          path.join(ROOT, "docs/release/windows-formal-offline-admission.json"),
          "utf8",
        ),
      ),
    );
    expect(manifest.version).toBe("0.4.15");
    expect(manifest.artifactSourceSha).toBe(ARTIFACT_SOURCE);
    expect(manifest.preflightRunId).toBe("42421033643");
    expect(manifest.preflightRunAttempt).toBe("1");
    expect(manifest.offlineRunId).toBe("42424610543");
    expect(manifest.offlineRunAttempt).toBe("1");
    expect(manifest.assets.map((asset) => asset.signedSha256)).toEqual([
      "c5c798c36e5e947f9c5a950d604c9f3504f0f0be62e2e0c0d6bfb620116e1507",
      "62481dc757706e748c26e716dc5c3ac1fd69b7af5e136087c08db2db68ace843",
    ]);
    const headTree = execSync("git -c core.quotePath=false ls-tree -r HEAD", {
      cwd: ROOT,
      encoding: "utf8",
      maxBuffer: 8 * 1024 * 1024,
    });
    expect(productTreeHash(headTree)).toBe(manifest.productTreeHash);
    const quotedTree = execSync("git -c core.quotePath=true ls-tree -r HEAD", {
      cwd: ROOT,
      encoding: "utf8",
      maxBuffer: 8 * 1024 * 1024,
    });
    expect(() => productTreeHash(quotedTree)).toThrow(
      /git ls-tree path must be unquoted/,
    );
    expect(productJobFingerprint(readFileSync(
      path.join(ROOT, ".github/workflows/release.yml"),
      "utf8",
    ))).toBe(PRODUCT_JOB_FINGERPRINT);
    expect(classifyWindowsByteSource({
      mode: "formal",
      version: "0.4.15",
      manifest,
    })).toBe("offline-sealed");
    expect(classifyWindowsByteSource({
      mode: "preflight",
      version: "0.4.15",
      manifest,
    })).toBe("rebuild");
    expect(classifyWindowsByteSource({
      mode: "formal",
      version: "0.4.15",
      manifest: null,
    })).toBe("rebuild");
  });

  it("keeps the delta from the sealed source inside the workflow allowlist", () => {
    let changed: string[] = [];
    try {
      execSync(`git cat-file -e ${ARTIFACT_SOURCE}^{commit}`, {
        cwd: ROOT,
        stdio: "ignore",
      });
      changed = execSync(`git diff --name-only ${ARTIFACT_SOURCE}`, {
        cwd: ROOT,
        encoding: "utf8",
      }).split("\n").filter(Boolean);
    } catch {
      changed = [];
    }
    const untracked = execSync("git ls-files --others --exclude-standard", {
      cwd: ROOT,
      encoding: "utf8",
    }).split("\n").filter(Boolean);
    for (const candidate of [...changed, ...untracked]) {
      expect(WORKFLOW_ONLY_PATHS, candidate).toContain(candidate);
    }
    expect(WORKFLOW_ONLY_PATHS).not.toContain("src-tauri/Cargo.toml");
    expect(WORKFLOW_ONLY_PATHS).not.toContain("scripts/release/windows-signing.mjs");
  });

  it("admits matching sealed bytes and preserves product build jobs", () => {
    const proof = prove();
    expect(proof.releaseSha).toBe(RELEASE_SOURCE);
    expect(proof.artifactSourceSha).toBe(ARTIFACT_SOURCE);
    expect(proof.publisher).toBe("Hangzhou Fuyu Talent Service Co., Ltd.");
    expect(proof.diffPaths).toEqual([".github/workflows/release.yml"]);
    assertProductJobsPreserved(workflowYaml(false), workflowYaml(true));
  });

  it.each<[string, (input: FormalOfflineAdmissionInput) => void, RegExp]>([
    ["missing raw bytes", (input) => {
      input.rawBytesByArchitecture.x64 = Buffer.alloc(0);
    }, /x64 raw installer must be non-empty bytes/],
    ["tampered signed bytes", (input) => {
      input.signedBytesByArchitecture.arm64 = Buffer.from("tampered");
    }, /arm64 signed installer SHA-256 does not match/],
    ["wrong preflight run", (input) => {
      input.preflightRun = run({ id: 42421033644 });
    }, /Preflight run does not match/],
    ["wrong preflight attempt", (input) => {
      input.preflightRun = run({ run_attempt: 2 });
    }, /Preflight run does not match/],
    ["wrong artifact source", (input) => {
      input.preflightRun = run({ head_sha: "c".repeat(40) });
    }, /Preflight run does not match/],
    ["failed preflight", (input) => {
      input.preflightRun = run({ conclusion: "failure" });
    }, /Preflight run must be a completed successful run/],
    ["wrong workflow", (input) => {
      input.preflightRun = run({ name: "CI", path: ".github/workflows/ci.yml" });
    }, /Preflight run does not match/],
    ["wrong repository", (input) => {
      input.offlineRun = offlineRun({ repository: { id: 1 } });
    }, /Offline seal run belongs to a different repository/],
    ["wrong offline attempt", (input) => {
      input.offlineRun = offlineRun({ run_attempt: 3 });
    }, /Offline seal run does not match/],
    ["unrelated source", (input) => {
      input.ancestor = false;
    }, /not an ancestor/],
    ["application byte drift", (input) => {
      input.releaseLsTree = tree(`100644 blob ${"d".repeat(40)}\tsrc/other.rs\n`);
    }, /Application bytes differ/],
    ["cross-source path", (input) => {
      input.diffStatus = "M\tsrc/main.rs\n";
    }, /non-workflow path: src\/main.rs/],
    ["missing sealed binding", (input) => {
      input.binding = { schema: "other" };
    }, /sealed binding keys must be exactly/],
  ])("rejects %s", (_label, change, error) => {
    expect(() => prove(change)).toThrow(error);
  });

  it("rejects an unsigned candidate and a product-job edit", () => {
    expect(() =>
      readFormalOfflineAdmissionManifest({
        ...fixture().input.manifest,
        assets: fixture().input.manifest.assets.map((asset) => ({
          ...asset,
          signedSha256: asset.rawSha256,
        })),
      }),
    ).toThrow(/unsigned output cannot enter a formal release/);
    const edited = workflowYaml(true).replace(
      "    runs-on: macos-15\n",
      "    runs-on: macos-15\n    steps:\n      - run: echo changed\n",
    );
    expect(() => assertProductJobsPreserved(workflowYaml(false), edited)).toThrow(
      /Product build job build-macos changed/,
    );
  });

  it("rejects unsigned status and mismatched published bytes", () => {
    const proof = prove();
    const signed = {
      schema: "fyagent-windows-signing-status/v1",
      product: "FyAgent",
      version: proof.version,
      sourceSha: proof.artifactSourceSha,
      mode: "unsigned",
      assets: proof.assets.map((asset) => ({
        ...asset,
        sha256: asset.signedSha256,
        sourceSha: proof.artifactSourceSha,
        signature: { status: "NotSigned", publisher: null },
      })),
    };
    expect(() => bindFormalSigningStatus(signed, proof)).toThrow(
      /Unsigned Windows signing status cannot enter/,
    );
    const valid = bindFormalSigningStatus(
      {
        ...signed,
        mode: "signed",
        assets: signed.assets.map((asset) => ({
          ...asset,
          signature: {
            status: "Valid",
            publisher: proof.publisher,
            signerCertificate: { sha256: proof.certificateSha256 },
          },
        })),
      },
      proof,
    );
    expect(valid.sourceSha).toBe(RELEASE_SOURCE);
    expect(valid.assets[0]?.sourceSha).toBe(ARTIFACT_SOURCE);
    const signedBytesByArchitecture = fixture().input.signedBytesByArchitecture;
    expect(() =>
      assertPublishedAdmission({
        proof,
        signingStatus: valid,
        buildMetadata: { offlineAdmission: proof },
        signedBytesByArchitecture: {
          ...signedBytesByArchitecture,
          x64: Buffer.from("swapped"),
        },
      }),
    ).toThrow(/x64 published installer SHA-256 does not match/);
    expect(
      assertPublishedAdmission({
        proof,
        signingStatus: valid,
        buildMetadata: { offlineAdmission: proof },
        signedBytesByArchitecture,
      }),
    ).toBe(true);
  });

  it("keeps preflight Windows metadata bound to the artifact run", () => {
    const proof = prove();
    const releaseIdentity = formalReleaseIdentity(proof);
    const directory = mkdtempSync(path.join(tmpdir(), "fyagent-admission-"));
    const staged = path.join(
      tmpdir(),
      `fyagent-admission-staged-${process.pid}-${Date.now()}`,
    );
    try {
      writeAdmittedPlatformMetadata(directory, proof, releaseIdentity);
      assertAdmittedWindowsMetadata(directory, proof);
      stageMetadataForAggregate({
        sourceDirectory: directory,
        destinationDirectory: staged,
        proof,
        releaseIdentity,
      });
      const metadata = buildBuildMetadata({
        metadataDirectory: staged,
        identity: releaseIdentity,
        generatedAt: "2026-11-11T08:57:00.000Z",
      });
      expect(metadata.sourceSha).toBe(proof.releaseSha);
      expect(metadata.targets).toHaveLength(3);
      const wrong = path.join(directory, "windows-x64.json");
      const parsed = JSON.parse(readFileSync(wrong, "utf8"));
      parsed.identity.runId = "1";
      writeFileSync(wrong, `${JSON.stringify(parsed)}\n`);
      expect(() => assertAdmittedWindowsMetadata(directory, proof)).toThrow(
        /windows-x64 metadata does not belong to the admitted preflight run/,
      );
    } finally {
      rmSync(directory, { recursive: true, force: true });
      rmSync(staged, { recursive: true, force: true });
    }
  });

  it("stages admitted metadata into the empty directory mkdtemp creates", () => {
    const proof = prove();
    const releaseIdentity = formalReleaseIdentity(proof);
    const root = mkdtempSync(path.join(tmpdir(), "fyagent-admitted-aggregate-"));
    const directory = path.join(root, "platform-metadata");
    const proofPath = path.join(root, "admission-proof.json");
    const output = path.join(root, "build-metadata.json");
    const staged = mkdtempSync(path.join(tmpdir(), "fyagent-admitted-metadata-"));
    try {
      mkdirSync(directory);
      writeAdmittedPlatformMetadata(directory, proof, releaseIdentity);
      writeFileSync(proofPath, `${JSON.stringify(proof)}\n`);
      stageMetadataForAggregate({
        sourceDirectory: directory,
        destinationDirectory: staged,
        proof,
        releaseIdentity,
      });
      expect(readdirSync(staged).sort()).toEqual([
        "macos-universal.json",
        "windows-arm64.json",
        "windows-x64.json",
      ]);
      const stagedWindows = JSON.parse(
        readFileSync(path.join(staged, "windows-x64.json"), "utf8"),
      );
      expect(stagedWindows.identity).toEqual(releaseIdentity);
      const sourceWindows = JSON.parse(
        readFileSync(path.join(directory, "windows-x64.json"), "utf8"),
      );
      expect(sourceWindows.identity.sourceSha).toBe(proof.artifactSourceSha);
      expect(sourceWindows.identity.mode).toBe("preflight");
      const metadata = buildBuildMetadata({
        metadataDirectory: staged,
        identity: releaseIdentity,
        generatedAt: "2026-11-11T08:57:00.000Z",
      });
      expect(metadata.sourceSha).toBe(proof.releaseSha);
      execFileSync(
        process.execPath,
        [
          path.join(ROOT, "scripts/release/generate-build-metadata.mjs"),
          directory,
          releaseIdentity.productVersion,
          releaseIdentity.tag,
          releaseIdentity.sourceSha,
          releaseIdentity.repository,
          releaseIdentity.repositoryId,
          releaseIdentity.runId,
          releaseIdentity.runAttempt,
          releaseIdentity.event,
          releaseIdentity.mode,
          releaseIdentity.workflowRef,
          releaseIdentity.workflowSha,
          "",
          "",
          "2026-11-11T08:57:00.000Z",
          output,
        ],
        {
          env: {
            ...process.env,
            FYAGENT_WINDOWS_OFFLINE_PROOF: proofPath,
          },
          encoding: "utf8",
        },
      );
      const aggregate = JSON.parse(readFileSync(output, "utf8"));
      expect(aggregate.schema).toBe("fyagent-build-metadata/v2");
      expect(aggregate.sourceSha).toBe(proof.releaseSha);
      expect(aggregate.offlineAdmission).toEqual(proof);
      expect(aggregate.targets).toHaveLength(3);
    } finally {
      rmSync(root, { recursive: true, force: true });
      rmSync(staged, { recursive: true, force: true });
    }
  });

  it("refuses a non-empty staging directory without changing its files", () => {
    const proof = prove();
    const releaseIdentity = formalReleaseIdentity(proof);
    const directory = mkdtempSync(path.join(tmpdir(), "fyagent-admission-"));
    const staged = mkdtempSync(path.join(tmpdir(), "fyagent-admitted-metadata-"));
    const occupied = path.join(staged, "windows-x64.json");
    const occupiedBytes = Buffer.from('{"keep":true}\n');
    try {
      writeAdmittedPlatformMetadata(directory, proof, releaseIdentity);
      writeFileSync(occupied, occupiedBytes);
      writeFileSync(path.join(staged, "extra.txt"), "extra");
      expect(() =>
        stageMetadataForAggregate({
          sourceDirectory: directory,
          destinationDirectory: staged,
          proof,
          releaseIdentity,
        }),
      ).toThrow(/Staged metadata directory already exists/);
      expect(readFileSync(occupied)).toEqual(occupiedBytes);
      expect(readFileSync(path.join(staged, "extra.txt"), "utf8")).toBe("extra");
      expect(readdirSync(staged).sort()).toEqual(["extra.txt", "windows-x64.json"]);
      const sourceWindows = JSON.parse(
        readFileSync(path.join(directory, "windows-x64.json"), "utf8"),
      );
      expect(sourceWindows.identity.mode).toBe("preflight");
      expect(sourceWindows.identity.sourceSha).toBe(proof.artifactSourceSha);
    } finally {
      rmSync(directory, { recursive: true, force: true });
      rmSync(staged, { recursive: true, force: true });
    }
  });

  it("refuses a staging path that is already a file", () => {
    const proof = prove();
    const releaseIdentity = formalReleaseIdentity(proof);
    const directory = mkdtempSync(path.join(tmpdir(), "fyagent-admission-"));
    const root = mkdtempSync(path.join(tmpdir(), "fyagent-admitted-metadata-"));
    const staged = path.join(root, "occupied");
    const occupiedBytes = Buffer.from("not-a-directory");
    try {
      writeAdmittedPlatformMetadata(directory, proof, releaseIdentity);
      writeFileSync(staged, occupiedBytes);
      expect(() =>
        stageMetadataForAggregate({
          sourceDirectory: directory,
          destinationDirectory: staged,
          proof,
          releaseIdentity,
        }),
      ).toThrow(/Staged metadata directory already exists/);
      expect(readFileSync(staged)).toEqual(occupiedBytes);
    } finally {
      rmSync(directory, { recursive: true, force: true });
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("classifies a missing manifest as rebuild and rejects an unknown command", () => {
    const script = path.join(
      ROOT,
      "scripts/release/windows-formal-offline-admission.mjs",
    );
    const result = execFileSync(
      process.execPath,
      [script, "classify", "--manifest-absent", "yes", "--version", "0.4.15", "--mode", "formal", "--github-output", path.join(tmpdir(), "fyagent-admission-output.txt")],
      { encoding: "utf8" },
    );
    expect(result).toBe("");
    expect(() =>
      execFileSync(process.execPath, [script, "not-a-command"], {
        encoding: "utf8",
        stdio: "pipe",
      }),
    ).toThrow(/Usage: windows-formal-offline-admission.mjs/);
  });
});
