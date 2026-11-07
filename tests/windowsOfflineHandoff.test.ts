import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import {
  parsePeImage,
  verifySealedWindowsCandidate,
} from "../scripts/release/windows-signing.mjs";
import {
  admitOfflineReturn,
  assertOfflinePublicationGate,
  expectedOfflineReturnPaths,
  validateOfflineDispatch,
} from "../scripts/release/windows-offline-handoff.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const version = "0.4.15";
const sourceSha = "6e05eb2929cab276e0660619b4af9ae79ba2a53c";
const otherSha = "a".repeat(40);
const publisher = "Hangzhou Fuyu Talent Service Co., Ltd.";
const certificateSha256 =
  "df97c8316288564fa123a9be0c239dd3510ff23c59699b7807aa8ac7861ddb53";
const temps: string[] = [];

function sha256(bytes: Buffer): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function writeTemp(bytes: Buffer): string {
  const directory = mkdtempSync(path.join(tmpdir(), "fyagent-offline-"));
  temps.push(directory);
  const file = path.join(
    directory,
    "FyAgent-0.4.15-Windows-x64-setup.exe",
  );
  writeFileSync(file, bytes);
  return file;
}

function makePe(seed: number): Buffer {
  const bytes = Buffer.alloc(513);
  const peOffset = 0x80;
  bytes.write("MZ", 0, "ascii");
  bytes.writeUInt32LE(peOffset, 0x3c);
  bytes.write("PE\0\0", peOffset, "binary");
  bytes.writeUInt16LE(0x014c, peOffset + 4);
  bytes.writeUInt16LE(1, peOffset + 6);
  bytes.writeUInt16LE(0x00e0, peOffset + 20);
  bytes.writeUInt16LE(0x010b, peOffset + 24);
  bytes.writeUInt32LE(16, peOffset + 24 + 92);
  for (let index = 400; index < bytes.length; index += 1) {
    bytes[index] = (index + seed) % 251;
  }
  return bytes;
}

function appendAuthenticode(unsignedBytes: Buffer): Buffer {
  const header = parsePeImage(unsignedBytes);
  const certificateOffset = Math.ceil(unsignedBytes.length / 8) * 8;
  const certificateSize = 16;
  const signedBytes = Buffer.alloc(certificateOffset + certificateSize);
  unsignedBytes.copy(signedBytes);
  signedBytes.writeUInt32LE(1, header.checksumOffset);
  signedBytes.writeUInt32LE(certificateOffset, header.securityDirectoryOffset);
  signedBytes.writeUInt32LE(certificateSize, header.securityDirectoryOffset + 4);
  signedBytes.writeUInt32LE(certificateSize, certificateOffset);
  signedBytes.writeUInt16LE(0x0200, certificateOffset + 4);
  signedBytes.writeUInt16LE(0x0002, certificateOffset + 6);
  signedBytes.fill(0x5a, certificateOffset + 8);
  return signedBytes;
}

function names() {
  const paths = expectedOfflineReturnPaths(version);
  return {
    x64: paths.find((entry) => entry.includes("x64")),
    arm64: paths.find((entry) => entry.includes("arm64")),
  };
}

function policy() {
  return {
    schema: "fyagent-windows-offline-signing-policy/v1",
    publisher,
    certificateSha256,
  };
}

function releaseRun(change: Record<string, unknown> = {}) {
  return {
    id: 42406854270,
    run_attempt: 1,
    head_sha: sourceSha,
    status: "completed",
    conclusion: "success",
    event: "workflow_dispatch",
    name: "Release",
    path: ".github/workflows/release.yml",
    repository: { id: 1313497021 },
    ...change,
  };
}

function pair() {
  const rawX64 = makePe(1);
  const rawArm64 = makePe(2);
  const signedX64 = appendAuthenticode(rawX64);
  const signedArm64 = appendAuthenticode(rawArm64);
  const fileNames = names();
  if (!fileNames.x64 || !fileNames.arm64) {
    throw new Error("missing installer names");
  }
  const manifest = {
    schema: "fyagent-windows-offline-signing-return/v1",
    sourceRunId: "42406854270",
    sourceRunAttempt: "1",
    sourceSha,
    version,
    repositoryId: "1313497021",
    assets: [
      {
        architecture: "x64",
        name: fileNames.x64,
        rawSha256: sha256(rawX64),
        signedSha256: sha256(signedX64),
      },
      {
        architecture: "arm64",
        name: fileNames.arm64,
        rawSha256: sha256(rawArm64),
        signedSha256: sha256(signedArm64),
      },
    ],
  };
  return { rawX64, rawArm64, signedX64, signedArm64, manifest };
}

type Input = ReturnType<typeof pair>;

function admit(
  input: Input,
  change: {
    manifest?: object;
    run?: ReturnType<typeof releaseRun>;
    treePaths?: string[];
    rawX64?: Buffer;
    signedX64?: Buffer;
    sourceSha?: string;
    sourceRunId?: string;
    sourceRunAttempt?: string;
  } = {},
) {
  return admitOfflineReturn({
    policy: policy(),
    returnManifest: change.manifest ?? input.manifest,
    treePaths: change.treePaths ?? expectedOfflineReturnPaths(version),
    run: change.run ?? releaseRun(),
    rawBytesByArchitecture: {
      x64: change.rawX64 ?? input.rawX64,
      arm64: input.rawArm64,
    },
    signedBytesByArchitecture: {
      x64: change.signedX64 ?? input.signedX64,
      arm64: input.signedArm64,
    },
    version,
    sourceSha: change.sourceSha ?? sourceSha,
    sourceRunId: change.sourceRunId ?? "42406854270",
    sourceRunAttempt: change.sourceRunAttempt ?? "1",
  });
}

afterEach(() => {
  for (const directory of temps.splice(0)) {
    rmSync(directory, { recursive: true });
  }
});

describe("Windows offline signing handoff", () => {
  it("pins the public certificate policy committed for the seal", () => {
    const committed = JSON.parse(
      readFileSync(
        path.join(root, "docs/release/windows-offline-signing-policy.json"),
        "utf8",
      ),
    );
    expect(committed).toEqual(policy());
  });

  it("admits one Release run and keeps the trusted publisher pin", () => {
    const input = pair();
    const binding = admit(input);
    expect(binding.publisher).toBe(publisher);
    expect(binding.certificateSha256).toBe(certificateSha256);
    expect(binding.sourceRunId).toBe("42406854270");
    expect(binding.assets.map((asset) => asset.rawSha256)).toEqual([
      sha256(input.rawX64),
      sha256(input.rawArm64),
    ]);
  });

  it.each([
    ["wrong run id", { sourceRunId: "42406854271" }],
    ["wrong attempt", { sourceRunAttempt: "2" }],
    ["wrong commit", { sourceSha: otherSha }],
  ] as const)("rejects a return bound to the %s", (_label, change) => {
    expect(() => admit(pair(), change)).toThrow(/does not match/);
  });

  it("rejects raw bytes copied from the other architecture", () => {
    const input = pair();
    const manifest = structuredClone(input.manifest);
    const x64 = manifest.assets[0];
    const arm64 = manifest.assets[1];
    if (!x64 || !arm64) throw new Error("missing assets");
    x64.rawSha256 = sha256(input.signedArm64);
    expect(() =>
      admit(input, { manifest, rawX64: input.signedArm64 }),
    ).toThrow(/crossed/);
  });

  it("rejects an unsigned candidate with the raw SHA-256", () => {
    const input = pair();
    const manifest = structuredClone(input.manifest);
    const x64 = manifest.assets[0];
    if (!x64) throw new Error("missing x64");
    x64.signedSha256 = x64.rawSha256;
    expect(() => admit(input, { manifest, signedX64: input.rawX64 })).toThrow(
      /unsigned output cannot complete/,
    );
  });

  it("rejects an extra file and a release URL", () => {
    const input = pair();
    expect(() =>
      admit(input, {
        treePaths: [...expectedOfflineReturnPaths(version), "notes.md"],
      }),
    ).toThrow(/tree must be exactly/);
    expect(() =>
      admit(input, {
        manifest: {
          ...input.manifest,
          notes: "https://github.com/fy-agent/fyagent/releases/download/v0.4.15/x",
        },
      }),
    ).toThrow(/keys must be exactly/);
  });

  it("rejects a run that is not the Release workflow", () => {
    const input = pair();
    expect(() =>
      admit(input, {
        run: releaseRun({
          path: ".github/workflows/windows-offline-sign-return.yml",
        }),
      }),
    ).toThrow(/must come from the Release workflow/);
    expect(() =>
      admit(input, { run: releaseRun({ conclusion: "failure" }) }),
    ).toThrow(/completed successful/);
  });

  it("rejects dispatch ids that are not the bound decimals and shas", () => {
    expect(() =>
      validateOfflineDispatch({
        sourceRunId: "42406854270;rm",
        sourceRunAttempt: "1",
        sourceSha,
        returnSha: otherSha,
      }),
    ).toThrow(/source_run_id/);
    expect(() =>
      validateOfflineDispatch({
        sourceRunId: "42406854270",
        sourceRunAttempt: "1",
        sourceSha,
        returnSha: sourceSha,
      }),
    ).toThrow(/separate commit/);
  });

  it("seals the admitted bytes and refuses an unsigned status", () => {
    const input = pair();
    const binding = admit(input);
    let probes = 0;
    const record = verifySealedWindowsCandidate(
      {
        rawAssetPath: writeTemp(input.rawX64),
        candidateAssetPath: writeTemp(input.signedX64),
        architecture: "x64",
        version,
        sourceSha,
        mode: "provider",
        expectedPublisher: binding.publisher,
        expectedCertificateSha256: binding.certificateSha256,
      },
      {
        probeAuthenticode: () => {
          probes += 1;
          if (probes === 1) {
            return {
              schema: "fyagent-authenticode-evidence/v1",
              status: "NotSigned",
              publisher: null,
              signerCertificate: null,
              timestampCertificate: null,
            };
          }
          return {
            schema: "fyagent-authenticode-evidence/v1",
            status: "Valid",
            publisher,
            signerCertificate: {
              subject: `CN=${publisher}`,
              simpleName: publisher,
              sha256: certificateSha256,
              notBefore: "2026-01-01T00:00:00.000Z",
              notAfter: "2027-11-02T23:59:59.000Z",
              enhancedKeyUsageOids: ["1.3.6.1.5.5.7.3.3"],
            },
            timestampCertificate: {
              subject: "CN=Timestamp Authority",
              simpleName: "Timestamp Authority",
              sha256: "c".repeat(64),
              notBefore: "2026-01-01T00:00:00.000Z",
              notAfter: "2028-01-01T00:00:00.000Z",
              enhancedKeyUsageOids: ["1.3.6.1.5.5.7.3.8"],
            },
          };
        },
      },
    );
    expect(record.mode).toBe("signed");
    expect(record.asset.sha256).toBe(sha256(input.signedX64));
    expect(() =>
      assertOfflinePublicationGate(
        {
          product: "FyAgent",
          version,
          sourceSha,
          mode: "unsigned",
          assets: [],
        },
        binding,
      ),
    ).toThrow(/Unsigned Windows signing status cannot complete/);
  });

  it("keeps the workflow on trusted main, raw artifacts, and the existing seal", () => {
    const workflow = readFileSync(
      path.join(root, ".github/workflows/windows-offline-sign-return.yml"),
      "utf8",
    );
    expect(workflow).toContain("github.ref == 'refs/heads/main'");
    expect(workflow).toContain("name: raw-windows-x64");
    expect(workflow).toContain("name: raw-windows-arm64");
    expect(workflow).toContain(
      "run-id: ${{ steps.dispatch.outputs.source_run_id }}",
    );
    expect(workflow).toContain("windows-signing.mjs verify-sealed");
    expect(workflow).toContain("--mode provider");
    expect(workflow).toContain("--expected-publisher");
    expect(workflow).toContain("--expected-certificate-sha256");
    expect(workflow).toContain("windows-offline-handoff.mjs assert-status");
    expect(workflow).not.toContain("secrets.");
    expect(workflow).not.toContain("FYAGENT_WINDOWS_SIGNER_CREDENTIAL");
    expect(workflow).not.toContain("gh release");
    expect(workflow).not.toContain("releases/download");
    expect(workflow).not.toContain("make_latest");
  });
});
