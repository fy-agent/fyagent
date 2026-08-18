import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  ATTESTATION_BUNDLE_NAME,
  WINDOWS_SIGNING_STATUS_NAME,
  expectedInstallerNames,
} from "../scripts/release/release-contract.mjs";
import {
  aggregateSigningStatus,
  assertAuthenticodeOnlyMutation,
  createAssetSigningRecord,
  expectedWindowsInstallerName,
  parsePeImage,
  resolveSignerConfiguration,
  transformWindowsCandidate,
  verifySealedWindowsCandidate,
  type RawAuthenticodeEvidence,
  type WindowsSigningArchitecture,
  type WindowsSigningAssetRecord,
} from "../scripts/release/windows-signing.mjs";

const repositoryRoot = path.resolve(__dirname, "..");
const signingScript = path.join(
  repositoryRoot,
  "scripts",
  "release",
  "windows-signing.mjs",
);
const sourceSha = "b".repeat(40);
const version = "0.3.1";
const signerCertificateSha256 = "a".repeat(64);
const temporaryRoots: string[] = [];

function temporaryDirectory(): string {
  const directory = mkdtempSync(path.join(tmpdir(), "fyagent-signing-"));
  temporaryRoots.push(directory);
  return directory;
}

function makePe(machine = 0x014c, length = 513): Buffer {
  const bytes = Buffer.alloc(length);
  const peOffset = 0x80;
  const optionalHeaderOffset = peOffset + 24;
  bytes.write("MZ", 0, "ascii");
  bytes.writeUInt32LE(peOffset, 0x3c);
  bytes.write("PE\0\0", peOffset, "binary");
  bytes.writeUInt16LE(machine, peOffset + 4);
  bytes.writeUInt16LE(1, peOffset + 6);
  bytes.writeUInt16LE(0x00e0, peOffset + 20);
  bytes.writeUInt16LE(0x010b, optionalHeaderOffset);
  bytes.writeUInt32LE(16, optionalHeaderOffset + 92);
  for (let index = 400; index < bytes.length; index += 1) {
    bytes[index] = index % 251;
  }
  return bytes;
}

function appendAuthenticode(
  unsignedBytes: Buffer,
  mutatePrefixAt?: number,
): Buffer {
  const header = parsePeImage(unsignedBytes);
  const certificateOffset = Math.ceil(unsignedBytes.length / 8) * 8;
  const certificateSize = 16;
  const signedBytes = Buffer.alloc(certificateOffset + certificateSize);
  unsignedBytes.copy(signedBytes);
  signedBytes.writeUInt32LE(1, header.checksumOffset);
  signedBytes.writeUInt32LE(certificateOffset, header.securityDirectoryOffset);
  signedBytes.writeUInt32LE(
    certificateSize,
    header.securityDirectoryOffset + 4,
  );
  signedBytes.writeUInt32LE(certificateSize, certificateOffset);
  signedBytes.writeUInt16LE(0x0200, certificateOffset + 4);
  signedBytes.writeUInt16LE(0x0002, certificateOffset + 6);
  signedBytes.fill(0x5a, certificateOffset + 8);
  if (mutatePrefixAt !== undefined) signedBytes[mutatePrefixAt] ^= 0xff;
  return signedBytes;
}

function sha256(bytes: Buffer): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function certificate(
  simpleName: string,
  digest: string,
  enhancedKeyUsageOids: string[],
) {
  return {
    subject: `CN=${simpleName}`,
    simpleName,
    sha256: digest,
    notBefore: "2026-01-01T00:00:00.000Z",
    notAfter: "2028-01-01T00:00:00.000Z",
    enhancedKeyUsageOids,
  };
}

function unsignedEvidence(
  status = "NotSigned",
  change: Partial<RawAuthenticodeEvidence> = {},
): RawAuthenticodeEvidence {
  return {
    schema: "fyagent-authenticode-evidence/v1",
    status,
    publisher: null,
    signerCertificate: null,
    timestampCertificate: null,
    ...change,
  };
}

function signedEvidence(
  change: Partial<RawAuthenticodeEvidence> = {},
): RawAuthenticodeEvidence {
  return {
    schema: "fyagent-authenticode-evidence/v1",
    status: "Valid",
    publisher: "FyAgent Publisher",
    signerCertificate: certificate(
      "FyAgent Publisher",
      signerCertificateSha256,
      ["1.3.6.1.5.5.7.3.3"],
    ),
    timestampCertificate: certificate("Timestamp Authority", "c".repeat(64), [
      "1.3.6.1.5.5.7.3.8",
    ]),
    ...change,
  };
}

function signerEnvironment(
  change: Record<string, string | undefined> = {},
): Record<string, string | undefined> {
  return {
    FYAGENT_WINDOWS_SIGNING_MODE: "provider",
    FYAGENT_WINDOWS_SIGNER_ADAPTER: path.resolve(
      temporaryDirectory(),
      "provider.ps1",
    ),
    FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER: "FyAgent Publisher",
    FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256: signerCertificateSha256,
    ...change,
  };
}

function writeAsset(
  directory: string,
  architecture: WindowsSigningArchitecture,
  bytes = makePe(),
): string {
  const assetPath = path.join(
    directory,
    expectedWindowsInstallerName(version, architecture),
  );
  writeFileSync(assetPath, bytes);
  return assetPath;
}

function stripEvidenceSchema(evidence: RawAuthenticodeEvidence) {
  const { schema: _schema, ...signature } = evidence;
  return signature;
}

function fragment(
  architecture: WindowsSigningArchitecture,
  bytes: Buffer,
  mode: "unsigned" | "signed" = "unsigned",
  evidence: RawAuthenticodeEvidence = unsignedEvidence(),
): WindowsSigningAssetRecord {
  return {
    schema: "fyagent-windows-signing-asset/v1",
    product: "FyAgent",
    version,
    sourceSha,
    mode,
    asset: {
      name: expectedWindowsInstallerName(version, architecture),
      architecture,
      sizeBytes: bytes.length,
      sha256: sha256(bytes),
      signature: stripEvidenceSchema(evidence),
    },
  };
}

function writeAggregateInputs(
  root: string,
  x64Fragment: WindowsSigningAssetRecord,
  arm64Fragment: WindowsSigningAssetRecord,
  x64Bytes: Buffer,
  arm64Bytes: Buffer,
) {
  const assetsDirectory = path.join(root, "assets");
  mkdirSync(assetsDirectory);
  writeFileSync(
    path.join(root, "windows-signing-x64.json"),
    `${JSON.stringify(x64Fragment)}\n`,
  );
  writeFileSync(
    path.join(root, "windows-signing-arm64.json"),
    `${JSON.stringify(arm64Fragment)}\n`,
  );
  writeFileSync(path.join(assetsDirectory, x64Fragment.asset.name), x64Bytes);
  writeFileSync(
    path.join(assetsDirectory, arm64Fragment.asset.name),
    arm64Bytes,
  );
  return {
    x64StatusPath: path.join(root, "windows-signing-x64.json"),
    arm64StatusPath: path.join(root, "windows-signing-arm64.json"),
    assetsDirectory,
    version,
    sourceSha,
  };
}

afterEach(() => {
  while (temporaryRoots.length > 0) {
    rmSync(temporaryRoots.pop()!, { recursive: true, force: true });
  }
});

describe("Windows signing adapter configuration", () => {
  it("uses the shared release names as its single source of truth", () => {
    expect(expectedWindowsInstallerName(version, "x64")).toBe(
      expectedInstallerNames(version).find((name) =>
        name.endsWith("-Windows-x64-setup.exe"),
      ),
    );
    expect(expectedWindowsInstallerName(version, "arm64")).toBe(
      expectedInstallerNames(version).find((name) =>
        name.endsWith("-Windows-arm64-setup.exe"),
      ),
    );
    expect(WINDOWS_SIGNING_STATUS_NAME).toBe("signing-status.json");
    expect(ATTESTATION_BUNDLE_NAME).toBe("artifact-attestation.sigstore.json");
  });

  it("selects unsigned mode when provider inputs are absent or cleared", () => {
    expect(resolveSignerConfiguration({})).toBeNull();
    expect(
      resolveSignerConfiguration({ FYAGENT_WINDOWS_SIGNING_MODE: "unsigned" }),
    ).toBeNull();
    expect(
      resolveSignerConfiguration({
        FYAGENT_WINDOWS_SIGNING_MODE: "unsigned",
        FYAGENT_WINDOWS_SIGNER_ADAPTER: "",
        FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER: "",
        FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256: "",
        FYAGENT_WINDOWS_SIGNER_CREDENTIAL: "",
      }),
    ).toBeNull();
    expect(() =>
      resolveSignerConfiguration({
        FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER: "FyAgent Publisher",
      }),
    ).toThrow(/SIGNING_MODE must be provider/);
    expect(() =>
      resolveSignerConfiguration({
        FYAGENT_WINDOWS_SIGNER_CREDENTIAL: "opaque-provider-secret",
      }),
    ).toThrow(/SIGNING_MODE must be provider/);
    expect(() =>
      resolveSignerConfiguration({
        FYAGENT_WINDOWS_SIGNER_ADAPTER: "provider.ps1",
        FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER: "FyAgent Publisher",
        FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256: "a".repeat(64),
      }),
    ).toThrow(/SIGNING_MODE must be provider/);
    expect(() =>
      resolveSignerConfiguration({
        FYAGENT_WINDOWS_SIGNING_MODE: "provider",
        FYAGENT_WINDOWS_SIGNER_ADAPTER: "",
        FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER: "",
        FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256: "",
      }),
    ).toThrow(/must not be empty/);
    expect(() =>
      resolveSignerConfiguration({ FYAGENT_WINDOWS_SIGNING_MODE: "provider" }),
    ).toThrow(/configuration is partial/);
    expect(() =>
      resolveSignerConfiguration({
        FYAGENT_WINDOWS_SIGNING_MODE: "provider",
        FYAGENT_WINDOWS_SIGNER_ADAPTER: path.resolve("provider.ps1"),
      }),
    ).toThrow(/configuration is partial/);
    expect(() =>
      resolveSignerConfiguration({
        FYAGENT_WINDOWS_SIGNING_MODE: "unsigned",
        FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER: "FyAgent Publisher",
      }),
    ).toThrow(/must be absent/);
    expect(() =>
      resolveSignerConfiguration({
        FYAGENT_WINDOWS_SIGNING_MODE: "unsigned",
        FYAGENT_WINDOWS_SIGNER_CREDENTIAL: "opaque-provider-secret",
      }),
    ).toThrow(/must be absent/);
    expect(() =>
      resolveSignerConfiguration({
        ...signerEnvironment(),
        FYAGENT_WINDOWS_SIGNING_MODE: "unsigned",
      }),
    ).toThrow(/must be absent/);
    expect(() =>
      resolveSignerConfiguration({ FYAGENT_WINDOWS_SIGNING_MODE: "" }),
    ).toThrow(/must be unsigned or provider/);
    expect(() =>
      resolveSignerConfiguration({ FYAGENT_WINDOWS_SIGNING_MODE: " provider" }),
    ).toThrow(/must be unsigned or provider/);
  });

  it.each([
    [
      "relative adapter",
      { FYAGENT_WINDOWS_SIGNER_ADAPTER: "provider.ps1" },
      /absolute path/,
    ],
    [
      "non-PowerShell adapter",
      { FYAGENT_WINDOWS_SIGNER_ADAPTER: path.resolve("provider.exe") },
      /PowerShell .ps1/,
    ],
    [
      "publisher outer whitespace",
      { FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER: " FyAgent Publisher" },
      /outer whitespace/,
    ],
    [
      "uppercase certificate digest",
      {
        FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256: "A".repeat(64),
      },
      /lowercase SHA-256/,
    ],
  ])("rejects malformed %s configuration", (_label, change, error) => {
    expect(() => resolveSignerConfiguration(signerEnvironment(change))).toThrow(
      error,
    );
  });
});

describe("Windows signing asset evidence", () => {
  it("records strict unsigned evidence without paths or certificate material", () => {
    const directory = temporaryDirectory();
    const assetPath = writeAsset(directory, "x64");
    const record = createAssetSigningRecord(
      {
        assetPath,
        architecture: "x64",
        version,
        sourceSha,
        environment: {},
      },
      { probeAuthenticode: () => unsignedEvidence() },
    );

    expect(record).toMatchObject({
      schema: "fyagent-windows-signing-asset/v1",
      product: "FyAgent",
      version,
      sourceSha,
      mode: "unsigned",
      asset: {
        name: expectedWindowsInstallerName(version, "x64"),
        architecture: "x64",
        signature: {
          status: "NotSigned",
          publisher: null,
          signerCertificate: null,
          timestampCertificate: null,
        },
      },
    });
    expect(JSON.stringify(record)).not.toContain(directory);
    expect(Object.keys(record.asset.signature).sort()).toEqual([
      "publisher",
      "signerCertificate",
      "status",
      "timestampCertificate",
    ]);
  });

  it("exposes an optional opaque credential only to the provider adapter child", () => {
    const source = readFileSync(signingScript, "utf8");
    const providerEnvironment = signerEnvironment();
    expect(
      resolveSignerConfiguration({
        ...providerEnvironment,
        FYAGENT_WINDOWS_SIGNER_CREDENTIAL: "opaque-provider-secret",
      }),
    ).toEqual(resolveSignerConfiguration(providerEnvironment));
    expect(source).toContain("env: childEnvironment(),");
    expect(source).toContain(
      "env: childEnvironment({ preserveCredential: true }),",
    );
    expect(source).toContain("delete environment[SIGNER_CREDENTIAL_NAME]");
  });

  it("accepts an x86 NSIS launcher for a logical arm64 installer", () => {
    const directory = temporaryDirectory();
    const assetPath = writeAsset(directory, "arm64", makePe(0x014c));
    const record = createAssetSigningRecord(
      {
        assetPath,
        architecture: "arm64",
        version,
        sourceSha,
        environment: {},
      },
      { probeAuthenticode: () => unsignedEvidence() },
    );
    expect(record.asset.architecture).toBe("arm64");
    expect(parsePeImage(readFileSync(assetPath)).machine).toBe(0x014c);
  });

  it("rejects residual PE security-directory data in strict unsigned mode", () => {
    const directory = temporaryDirectory();
    const bytes = makePe();
    const header = parsePeImage(bytes);
    bytes.writeUInt32LE(512, header.securityDirectoryOffset);
    bytes.writeUInt32LE(8, header.securityDirectoryOffset + 4);
    const assetPath = writeAsset(directory, "x64", bytes);
    expect(() =>
      createAssetSigningRecord(
        {
          assetPath,
          architecture: "x64",
          version,
          sourceSha,
          environment: {},
        },
        { probeAuthenticode: () => unsignedEvidence() },
      ),
    ).toThrow(/security directory must be empty/);
  });

  it.each(["HashMismatch", "UnknownError", "Valid"])(
    "hard-fails unsigned mode when Authenticode reports %s",
    (status) => {
      const directory = temporaryDirectory();
      const assetPath = writeAsset(directory, "x64");
      expect(() =>
        createAssetSigningRecord(
          {
            assetPath,
            architecture: "x64",
            version,
            sourceSha,
            environment: {},
          },
          { probeAuthenticode: () => unsignedEvidence(status) },
        ),
      ).toThrow(/must report NotSigned/);
    },
  );

  it("rejects signer or timestamp certificates in unsigned mode", () => {
    const directory = temporaryDirectory();
    const assetPath = writeAsset(directory, "x64");
    expect(() =>
      createAssetSigningRecord(
        {
          assetPath,
          architecture: "x64",
          version,
          sourceSha,
          environment: {},
        },
        {
          probeAuthenticode: () =>
            unsignedEvidence("NotSigned", {
              signerCertificate: signedEvidence().signerCertificate,
            }),
        },
      ),
    ).toThrow(/must not expose publisher, signer, or timestamp/);
  });

  it("signs through the provider adapter and verifies publisher, policy, timestamp, and bytes", () => {
    const directory = temporaryDirectory();
    const assetPath = writeAsset(directory, "x64");
    let probeCount = 0;
    const record = createAssetSigningRecord(
      {
        assetPath,
        architecture: "x64",
        version,
        sourceSha,
        environment: signerEnvironment({
          PROVIDER_API_TOKEN: "credential-must-not-be-recorded",
        }),
      },
      {
        probeAuthenticode: () =>
          probeCount++ === 0 ? unsignedEvidence() : signedEvidence(),
        invokeSigner: ({ assetPath: pathToSign, architecture }) => {
          expect(architecture).toBe("x64");
          const unsignedBytes = readFileSync(pathToSign);
          writeFileSync(pathToSign, appendAuthenticode(unsignedBytes));
        },
      },
    );

    expect(record.mode).toBe("signed");
    expect(record.asset.signature).toMatchObject({
      status: "Valid",
      publisher: "FyAgent Publisher",
      signerCertificate: { sha256: signerCertificateSha256 },
      timestampCertificate: {
        enhancedKeyUsageOids: ["1.3.6.1.5.5.7.3.8"],
      },
    });
    expect(record.asset.sha256).toBe(sha256(readFileSync(assetPath)));
    expect(JSON.stringify(record)).not.toContain(
      "credential-must-not-be-recorded",
    );
    expect(JSON.stringify(record)).not.toContain("provider.ps1");
  });

  it("lets the secret-bearing producer transform once without trusting post-provider bytes", () => {
    const directory = temporaryDirectory();
    const assetPath = writeAsset(directory, "x64");
    let probeCount = 0;
    let signerCount = 0;
    transformWindowsCandidate(
      {
        assetPath,
        architecture: "x64",
        version,
        sourceSha,
        environment: signerEnvironment(),
      },
      {
        probeAuthenticode: () => {
          probeCount += 1;
          return unsignedEvidence();
        },
        invokeSigner: ({ assetPath: pathToSign }) => {
          signerCount += 1;
          writeFileSync(
            pathToSign,
            appendAuthenticode(readFileSync(pathToSign)),
          );
        },
      },
    );

    expect(probeCount).toBe(1);
    expect(signerCount).toBe(1);
    expect(
      parsePeImage(readFileSync(assetPath)).certificateSize,
    ).toBeGreaterThan(0);
  });

  it("keeps an explicitly unsigned formal candidate unchanged after Windows clears provider keys", () => {
    const directory = temporaryDirectory();
    const assetPath = writeAsset(directory, "x64");
    const originalBytes = readFileSync(assetPath);
    let signerCount = 0;

    transformWindowsCandidate(
      {
        assetPath,
        architecture: "x64",
        version,
        sourceSha,
        environment: {
          FYAGENT_WINDOWS_SIGNING_MODE: "unsigned",
          FYAGENT_WINDOWS_SIGNER_ADAPTER: "",
          FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER: "",
          FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256: "",
          FYAGENT_WINDOWS_SIGNER_CREDENTIAL: "",
        },
      },
      {
        probeAuthenticode: () => unsignedEvidence(),
        invokeSigner: () => {
          signerCount += 1;
        },
      },
    );

    expect(signerCount).toBe(0);
    expect(readFileSync(assetPath)).toEqual(originalBytes);
  });

  it("rejects command failure and Authenticode-only byte violations", () => {
    const commandFailureRoot = temporaryDirectory();
    const commandFailureAsset = writeAsset(commandFailureRoot, "x64");
    expect(() =>
      createAssetSigningRecord(
        {
          assetPath: commandFailureAsset,
          architecture: "x64",
          version,
          sourceSha,
          environment: signerEnvironment(),
        },
        {
          probeAuthenticode: () => unsignedEvidence(),
          invokeSigner: () => {
            throw new Error("provider failed");
          },
        },
      ),
    ).toThrow(/provider failed/);

    const mutationRoot = temporaryDirectory();
    const mutationAsset = writeAsset(mutationRoot, "x64");
    expect(() =>
      createAssetSigningRecord(
        {
          assetPath: mutationAsset,
          architecture: "x64",
          version,
          sourceSha,
          environment: signerEnvironment(),
        },
        {
          probeAuthenticode: () => unsignedEvidence(),
          invokeSigner: ({ assetPath: pathToSign }) => {
            writeFileSync(
              pathToSign,
              appendAuthenticode(readFileSync(pathToSign), 450),
            );
          },
        },
      ),
    ).toThrow(/modified installer bytes outside Authenticode-owned fields/);
  });

  it("revalidates the post-sign file and rejects a changed real path", () => {
    const directory = temporaryDirectory();
    const assetPath = writeAsset(directory, "x64");
    const replacementPath = path.join(directory, "replacement.exe");
    let resolutionCount = 0;
    expect(() =>
      createAssetSigningRecord(
        {
          assetPath,
          architecture: "x64",
          version,
          sourceSha,
          environment: signerEnvironment(),
        },
        {
          resolveRegularFile: (filePath) =>
            resolutionCount++ === 0
              ? path.resolve(filePath)
              : path.resolve(replacementPath),
          probeAuthenticode: () => unsignedEvidence(),
          invokeSigner: ({ assetPath: pathToSign }) => {
            writeFileSync(
              replacementPath,
              appendAuthenticode(readFileSync(pathToSign)),
            );
          },
        },
      ),
    ).toThrow(/changed the installer real path/);
    expect(resolutionCount).toBe(2);
  });

  it.each([
    [
      "HashMismatch",
      () => signedEvidence({ status: "HashMismatch" }),
      /must report Valid/,
    ],
    [
      "publisher mismatch",
      () => signedEvidence({ publisher: "Unexpected Publisher" }),
      /publisher differs/,
    ],
    [
      "certificate mismatch",
      () =>
        signedEvidence({
          signerCertificate: certificate("FyAgent Publisher", "d".repeat(64), [
            "1.3.6.1.5.5.7.3.3",
          ]),
        }),
      /certificate SHA-256 differs/,
    ],
    [
      "missing timestamp",
      () => signedEvidence({ timestampCertificate: null }),
      /missing an Authenticode timestamp/,
    ],
    [
      "wrong timestamp policy",
      () =>
        signedEvidence({
          timestampCertificate: certificate(
            "Timestamp Authority",
            "c".repeat(64),
            ["1.3.6.1.5.5.7.3.3"],
          ),
        }),
      /does not include the Time Stamping EKU/,
    ],
  ])("hard-fails signed mode on %s", (_label, finalEvidence, error) => {
    const directory = temporaryDirectory();
    const assetPath = writeAsset(directory, "x64");
    let probeCount = 0;
    expect(() =>
      createAssetSigningRecord(
        {
          assetPath,
          architecture: "x64",
          version,
          sourceSha,
          environment: signerEnvironment(),
        },
        {
          probeAuthenticode: () =>
            probeCount++ === 0 ? unsignedEvidence() : finalEvidence(),
          invokeSigner: ({ assetPath: pathToSign }) => {
            writeFileSync(
              pathToSign,
              appendAuthenticode(readFileSync(pathToSign)),
            );
          },
        },
      ),
    ).toThrow(error);
  });

  it("accepts only Authenticode-owned PE changes", () => {
    const unsignedBytes = makePe();
    expect(() =>
      assertAuthenticodeOnlyMutation(
        unsignedBytes,
        appendAuthenticode(unsignedBytes),
      ),
    ).not.toThrow();
    expect(() =>
      assertAuthenticodeOnlyMutation(unsignedBytes, unsignedBytes),
    ).toThrow(/did not append/);
    const changedMachine = appendAuthenticode(unsignedBytes);
    changedMachine.writeUInt16LE(0x8664, 0x80 + 4);
    expect(() =>
      assertAuthenticodeOnlyMutation(unsignedBytes, changedMachine),
    ).toThrow(/changed the installer launcher PE Machine/);
  });
});

describe("fresh formal Windows verification and sealing", () => {
  function formalPair(
    candidateBytes: Buffer,
    architecture: WindowsSigningArchitecture = "x64",
  ) {
    const rawRoot = temporaryDirectory();
    const candidateRoot = temporaryDirectory();
    return {
      rawAssetPath: writeAsset(rawRoot, architecture),
      candidateAssetPath: writeAsset(
        candidateRoot,
        architecture,
        candidateBytes,
      ),
      architecture,
      version,
      sourceSha,
    };
  }

  it("re-proves byte-identical unsigned bytes and emits the trusted fragment", () => {
    const unsignedBytes = makePe();
    const input = formalPair(unsignedBytes);
    const probedPaths: string[] = [];
    const record = verifySealedWindowsCandidate(
      { ...input, mode: "unsigned" },
      {
        probeAuthenticode: (assetPath) => {
          probedPaths.push(assetPath);
          return unsignedEvidence();
        },
      },
    );

    expect(probedPaths).toEqual([
      realpathSync(input.rawAssetPath),
      realpathSync(input.candidateAssetPath),
    ]);
    expect(record).toEqual(
      fragment("x64", unsignedBytes, "unsigned", unsignedEvidence()),
    );
  });

  it("independently proves an Authenticode-only provider result and public policy", () => {
    const rawBytes = makePe();
    const signedBytes = appendAuthenticode(rawBytes);
    const input = formalPair(signedBytes);
    let probeCount = 0;
    const record = verifySealedWindowsCandidate(
      {
        ...input,
        mode: "provider",
        expectedPublisher: "FyAgent Publisher",
        expectedCertificateSha256: signerCertificateSha256,
      },
      {
        probeAuthenticode: () =>
          probeCount++ === 0 ? unsignedEvidence() : signedEvidence(),
      },
    );

    expect(record).toEqual(
      fragment("x64", signedBytes, "signed", signedEvidence()),
    );
  });

  it("rejects unsigned drift and provider mutations outside Authenticode fields", () => {
    const rawBytes = makePe();
    const unsignedDrift = Buffer.from(rawBytes);
    unsignedDrift[450] ^= 0xff;
    expect(() =>
      verifySealedWindowsCandidate(
        { ...formalPair(unsignedDrift), mode: "unsigned" },
        { probeAuthenticode: () => unsignedEvidence() },
      ),
    ).toThrow(/byte-identical/);

    expect(() =>
      verifySealedWindowsCandidate(
        {
          ...formalPair(appendAuthenticode(rawBytes, 450)),
          mode: "provider",
          expectedPublisher: "FyAgent Publisher",
          expectedCertificateSha256: signerCertificateSha256,
        },
        {
          probeAuthenticode: (() => {
            let count = 0;
            return () =>
              count++ === 0 ? unsignedEvidence() : signedEvidence();
          })(),
        },
      ),
    ).toThrow(/outside Authenticode-owned fields/);
  });

  it("rejects incomplete public policy and bytes changed during fresh verification", () => {
    const rawBytes = makePe();
    const signedBytes = appendAuthenticode(rawBytes);
    expect(() =>
      verifySealedWindowsCandidate(
        {
          ...formalPair(signedBytes),
          mode: "provider",
          expectedPublisher: "FyAgent Publisher",
        },
        { probeAuthenticode: () => unsignedEvidence() },
      ),
    ).toThrow(/lowercase SHA-256/);

    const changing = formalPair(signedBytes);
    let probeCount = 0;
    expect(() =>
      verifySealedWindowsCandidate(
        {
          ...changing,
          mode: "provider",
          expectedPublisher: "FyAgent Publisher",
          expectedCertificateSha256: signerCertificateSha256,
        },
        {
          probeAuthenticode: (assetPath) => {
            if (probeCount++ === 0) return unsignedEvidence();
            writeFileSync(
              assetPath,
              Buffer.concat([signedBytes, Buffer.from([0])]),
            );
            return signedEvidence();
          },
        },
      ),
    ).toThrow(/changed during independent verification/);
  });
});

describe("Windows signing status aggregation", () => {
  it("binds both native fragments to final bytes and public attestation references", () => {
    const root = temporaryDirectory();
    const x64Bytes = makePe();
    const arm64Bytes = makePe(0x014c, 521);
    const input = writeAggregateInputs(
      root,
      fragment("x64", x64Bytes),
      fragment("arm64", arm64Bytes),
      x64Bytes,
      arm64Bytes,
    );
    const status = aggregateSigningStatus(input);

    expect(status).toMatchObject({
      schema: "fyagent-windows-signing-status/v1",
      product: "FyAgent",
      version,
      sourceSha,
      mode: "unsigned",
    });
    expect(status.assets.map(({ architecture }) => architecture)).toEqual([
      "x64",
      "arm64",
    ]);
    for (const asset of status.assets) {
      expect(asset.sourceSha).toBe(sourceSha);
      expect(asset.attestation).toEqual({
        bundle: ATTESTATION_BUNDLE_NAME,
        subjectName: asset.name,
        subjectDigest: `sha256:${asset.sha256}`,
      });
      expect(asset.signature).toMatchObject({
        status: "NotSigned",
        signerCertificate: null,
        timestampCertificate: null,
      });
    }
    expect(JSON.stringify(status)).not.toContain(root);
  });

  it("runs the aggregate CLI hermetically and refuses to overwrite status", () => {
    const root = temporaryDirectory();
    const x64Bytes = makePe();
    const arm64Bytes = makePe();
    const input = writeAggregateInputs(
      root,
      fragment("x64", x64Bytes),
      fragment("arm64", arm64Bytes),
      x64Bytes,
      arm64Bytes,
    );
    const output = path.join(root, WINDOWS_SIGNING_STATUS_NAME);
    const args = [
      signingScript,
      "aggregate",
      "--x64-status",
      input.x64StatusPath,
      "--arm64-status",
      input.arm64StatusPath,
      "--assets-directory",
      input.assetsDirectory,
      "--version",
      version,
      "--source-sha",
      sourceSha,
      "--output",
      output,
    ];
    execFileSync(process.execPath, args, {
      cwd: repositoryRoot,
      stdio: "pipe",
    });
    expect(JSON.parse(readFileSync(output, "utf8"))).toMatchObject({
      schema: "fyagent-windows-signing-status/v1",
      mode: "unsigned",
    });
    expect(() =>
      execFileSync(process.execPath, args, {
        cwd: repositoryRoot,
        stdio: "pipe",
      }),
    ).toThrow();
  });

  it("rejects mode, publisher, signer, source, schema, and byte inconsistencies", () => {
    const signedBytes = appendAuthenticode(makePe());
    const unsignedBytes = makePe();
    const signedX64 = fragment("x64", signedBytes, "signed", signedEvidence());
    const signedArm64 = fragment(
      "arm64",
      signedBytes,
      "signed",
      signedEvidence(),
    );

    const modeRoot = temporaryDirectory();
    expect(() =>
      aggregateSigningStatus(
        writeAggregateInputs(
          modeRoot,
          fragment("x64", unsignedBytes),
          signedArm64,
          unsignedBytes,
          signedBytes,
        ),
      ),
    ).toThrow(/signing modes are inconsistent/);

    const publisherRoot = temporaryDirectory();
    const differentPublisher = fragment(
      "arm64",
      signedBytes,
      "signed",
      signedEvidence({
        publisher: "Other Publisher",
        signerCertificate: certificate(
          "Other Publisher",
          signerCertificateSha256,
          ["1.3.6.1.5.5.7.3.3"],
        ),
      }),
    );
    expect(() =>
      aggregateSigningStatus(
        writeAggregateInputs(
          publisherRoot,
          signedX64,
          differentPublisher,
          signedBytes,
          signedBytes,
        ),
      ),
    ).toThrow(/publishers are inconsistent/);

    const certificateRoot = temporaryDirectory();
    const differentCertificate = fragment(
      "arm64",
      signedBytes,
      "signed",
      signedEvidence({
        signerCertificate: certificate("FyAgent Publisher", "d".repeat(64), [
          "1.3.6.1.5.5.7.3.3",
        ]),
      }),
    );
    expect(() =>
      aggregateSigningStatus(
        writeAggregateInputs(
          certificateRoot,
          signedX64,
          differentCertificate,
          signedBytes,
          signedBytes,
        ),
      ),
    ).toThrow(/signer certificates are inconsistent/);

    const policyRoot = temporaryDirectory();
    const differentPolicy = fragment(
      "arm64",
      signedBytes,
      "signed",
      signedEvidence({
        signerCertificate: certificate(
          "FyAgent Publisher",
          signerCertificateSha256,
          ["1.2.3.4", "1.3.6.1.5.5.7.3.3"],
        ),
      }),
    );
    expect(() =>
      aggregateSigningStatus(
        writeAggregateInputs(
          policyRoot,
          signedX64,
          differentPolicy,
          signedBytes,
          signedBytes,
        ),
      ),
    ).toThrow(/certificate policies are inconsistent/);

    const sourceRoot = temporaryDirectory();
    const wrongSource = fragment("arm64", unsignedBytes);
    wrongSource.sourceSha = "c".repeat(40);
    expect(() =>
      aggregateSigningStatus(
        writeAggregateInputs(
          sourceRoot,
          fragment("x64", unsignedBytes),
          wrongSource,
          unsignedBytes,
          unsignedBytes,
        ),
      ),
    ).toThrow(/source SHA drifted/);

    const schemaRoot = temporaryDirectory();
    const extraKey = fragment(
      "arm64",
      unsignedBytes,
    ) as WindowsSigningAssetRecord & {
      unexpected?: boolean;
    };
    extraKey.unexpected = true;
    expect(() =>
      aggregateSigningStatus(
        writeAggregateInputs(
          schemaRoot,
          fragment("x64", unsignedBytes),
          extraKey,
          unsignedBytes,
          unsignedBytes,
        ),
      ),
    ).toThrow(/must contain exactly these keys/);

    const bytesRoot = temporaryDirectory();
    expect(() =>
      aggregateSigningStatus(
        writeAggregateInputs(
          bytesRoot,
          fragment("x64", unsignedBytes),
          fragment("arm64", unsignedBytes),
          Buffer.from(unsignedBytes).fill(0, 450, 451),
          unsignedBytes,
        ),
      ),
    ).toThrow(/SHA-256 differs from native signing evidence/);
  });
});
