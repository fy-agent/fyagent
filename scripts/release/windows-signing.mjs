#!/usr/bin/env node

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { lstatSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  ATTESTATION_BUNDLE_NAME,
  INSTALLER_RULES,
  PRODUCT_NAME,
  WINDOWS_SIGNING_STATUS_NAME,
  expectedInstallerNames,
} from "./release-contract.mjs";

export const WINDOWS_SIGNING_ASSET_SCHEMA = "fyagent-windows-signing-asset/v1";
export const WINDOWS_SIGNING_STATUS_SCHEMA =
  "fyagent-windows-signing-status/v1";
export { ATTESTATION_BUNDLE_NAME, WINDOWS_SIGNING_STATUS_NAME };
const ARCHITECTURES = Object.freeze(["x64", "arm64"]);
const SIGNING_MODE_NAME = "FYAGENT_WINDOWS_SIGNING_MODE";
const SIGNER_REQUIRED_CONFIGURATION_NAMES = Object.freeze([
  "FYAGENT_WINDOWS_SIGNER_ADAPTER",
  "FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER",
  "FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256",
]);
const SIGNER_CREDENTIAL_NAME = "FYAGENT_WINDOWS_SIGNER_CREDENTIAL";
export const CERTUM_SIMPLYSIGN_ADAPTER_NAME =
  "windows-signer-certum-simplysign.ps1";
export const CERTUM_SIMPLYSIGN_PROVIDER = "certum-simplysign";
export const CERTUM_SIMPLYSIGN_TIMESTAMP_URL = "http://time.certum.pl";
const CERTUM_CREDENTIAL_MAX_LENGTH = 8192;
const CERTUM_THUMBPRINT_PATTERN = /^[0-9a-f]{40}$/;
const CERTUM_REQUIRED_CREDENTIAL_KEYS = Object.freeze([
  "certificateSha1",
  "provider",
  "timestampUrl",
]);
const CERTUM_OPTIONAL_CREDENTIAL_KEYS = Object.freeze(["signtoolPath"]);
const SIGNER_CONFIGURATION_NAMES = Object.freeze([
  ...SIGNER_REQUIRED_CONFIGURATION_NAMES,
  SIGNER_CREDENTIAL_NAME,
]);
const STABLE_VERSION_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const SOURCE_SHA_PATTERN = /^[0-9a-f]{40}$/;
const SHA256_PATTERN = /^[0-9a-f]{64}$/;
const OID_PATTERN = /^(?:0|[1-9]\d*)(?:\.(?:0|[1-9]\d*))+$/;
const CODE_SIGNING_EKU = "1.3.6.1.5.5.7.3.3";
const TIMESTAMPING_EKU = "1.3.6.1.5.5.7.3.8";
const SUPPORTED_LAUNCHER_PE_MACHINES = Object.freeze([
  0x014c, // NSIS launchers may remain x86 for either logical product architecture.
  0x8664,
  0xaa64,
]);
const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const authenticodeEvidenceScript = path.join(
  scriptDirectory,
  "windows-signing-evidence.ps1",
);

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertExactKeys(value, expectedKeys, label) {
  assert(
    value !== null && typeof value === "object" && !Array.isArray(value),
    `${label} must be an object`,
  );
  const actual = Object.keys(value).sort();
  const expected = [...expectedKeys].sort();
  assert(
    actual.length === expected.length &&
      actual.every((key, index) => key === expected[index]),
    `${label} must contain exactly these keys: ${expected.join(", ")}`,
  );
}

function assertStableVersion(version) {
  assert(
    typeof version === "string" && STABLE_VERSION_PATTERN.test(version),
    `Invalid stable application version: ${version}`,
  );
}

function assertSourceSha(sourceSha) {
  assert(
    typeof sourceSha === "string" && SOURCE_SHA_PATTERN.test(sourceSha),
    "source SHA must be a lowercase full 40-character Git commit SHA",
  );
}

function assertArchitecture(architecture) {
  assert(
    ARCHITECTURES.includes(architecture),
    `Windows signing architecture must be x64 or arm64; received ${architecture}`,
  );
}

function assertSafeText(value, label, maximumLength = 512) {
  assert(typeof value === "string", `${label} must be a string`);
  assert(value.length > 0, `${label} must not be empty`);
  assert(value.length <= maximumLength, `${label} is too long`);
  assert(value.trim() === value, `${label} must not have outer whitespace`);
  assert(
    !/[\u0000-\u001f\u007f]/.test(value),
    `${label} contains control data`,
  );
}

function assertIsoInstant(value, label) {
  assertSafeText(value, label, 64);
  assert(
    Number.isFinite(Date.parse(value)),
    `${label} must be an ISO-compatible instant`,
  );
}

function assertRegularFile(filePath, label) {
  const entry = lstatSync(filePath);
  assert(entry.isFile(), `${label} must be a regular file`);
  assert(!entry.isSymbolicLink(), `${label} must not be a symbolic link`);
  assert(entry.size > 0, `${label} must not be empty`);
  return realpathSync(filePath);
}

function sha256(buffer) {
  return createHash("sha256").update(buffer).digest("hex");
}

function childEnvironment({ preserveCredential = false } = {}) {
  const environment = { ...process.env };
  delete environment[SIGNING_MODE_NAME];
  for (const name of SIGNER_REQUIRED_CONFIGURATION_NAMES) {
    delete environment[name];
  }
  if (!preserveCredential) {
    delete environment[SIGNER_CREDENTIAL_NAME];
  }
  return environment;
}

export function expectedWindowsInstallerName(version, architecture) {
  assertStableVersion(version);
  assertArchitecture(architecture);
  const matchingIndexes = INSTALLER_RULES.flatMap((rule, index) =>
    rule.platform === "windows" && rule.architecture === architecture
      ? [index]
      : [],
  );
  assert(
    matchingIndexes.length === 1,
    `Release contract must define exactly one ${architecture} Windows installer`,
  );
  return expectedInstallerNames(version)[matchingIndexes[0]];
}

export function resolveSignerConfiguration(environment) {
  const hasMode = Object.prototype.hasOwnProperty.call(
    environment,
    SIGNING_MODE_NAME,
  );
  const mode = environment[SIGNING_MODE_NAME];
  // Windows can preserve a cleared process variable as an empty child entry.
  // Provider mode still validates every required own key below.
  const supplied = SIGNER_CONFIGURATION_NAMES.filter(
    (name) =>
      Object.prototype.hasOwnProperty.call(environment, name) &&
      environment[name] !== "",
  );

  if (!hasMode) {
    assert(
      supplied.length === 0,
      `${SIGNING_MODE_NAME} must be provider before defining signer configuration`,
    );
    return null;
  }

  assert(
    mode === "unsigned" || mode === "provider",
    `${SIGNING_MODE_NAME} must be unsigned or provider`,
  );
  if (mode === "unsigned") {
    assert(
      supplied.length === 0,
      "Windows signer configuration must be absent when signing mode is unsigned",
    );
    return null;
  }

  const suppliedRequired = SIGNER_REQUIRED_CONFIGURATION_NAMES.filter((name) =>
    Object.prototype.hasOwnProperty.call(environment, name),
  );
  assert(
    suppliedRequired.length === SIGNER_REQUIRED_CONFIGURATION_NAMES.length,
    `Windows signer configuration is partial; define all of ${SIGNER_REQUIRED_CONFIGURATION_NAMES.join(", ")} or none of them`,
  );

  const adapterPath = environment.FYAGENT_WINDOWS_SIGNER_ADAPTER;
  const expectedPublisher = environment.FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER;
  const expectedCertificateSha256 =
    environment.FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256;
  assertSafeText(adapterPath, "FYAGENT_WINDOWS_SIGNER_ADAPTER", 4096);
  assert(
    path.isAbsolute(adapterPath),
    "FYAGENT_WINDOWS_SIGNER_ADAPTER must be an absolute path",
  );
  assert(
    path.extname(adapterPath).toLowerCase() === ".ps1",
    "FYAGENT_WINDOWS_SIGNER_ADAPTER must reference a PowerShell .ps1 adapter",
  );
  assertSafeText(
    expectedPublisher,
    "FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER",
    256,
  );
  assert(
    typeof expectedCertificateSha256 === "string" &&
      SHA256_PATTERN.test(expectedCertificateSha256),
    "FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256 must be a lowercase SHA-256 digest",
  );
  return {
    adapterPath,
    expectedPublisher,
    expectedCertificateSha256,
  };
}

function windowsAbsoluteSigntoolPath(value) {
  if (typeof value !== "string") return false;
  const drive = /^[A-Za-z]:[\\/]/.test(value);
  const unc = /^\\\\[^\\]+\\/.test(value);
  const segments = value.split(/[\\/]/);
  return (
    (drive || unc) &&
    !segments.includes("..") &&
    path.win32.basename(value).toLowerCase() === "signtool.exe"
  );
}

export function parseCertumSimplySignCredential(value) {
  if (value === undefined || value === null || value === "") {
    throw new Error(
      "FYAGENT_WINDOWS_SIGNER_CREDENTIAL is required; refusing to leave the artifact unsigned",
    );
  }
  assertSafeText(
    value,
    "FYAGENT_WINDOWS_SIGNER_CREDENTIAL",
    CERTUM_CREDENTIAL_MAX_LENGTH,
  );
  let parsed;
  try {
    parsed = JSON.parse(value);
  } catch {
    throw new Error(
      "FYAGENT_WINDOWS_SIGNER_CREDENTIAL must be a JSON object",
    );
  }
  assert(
    parsed !== null && typeof parsed === "object" && !Array.isArray(parsed),
    "FYAGENT_WINDOWS_SIGNER_CREDENTIAL must be a JSON object",
  );
  for (const key of Object.keys(parsed)) {
    if (
      !CERTUM_REQUIRED_CREDENTIAL_KEYS.includes(key) &&
      !CERTUM_OPTIONAL_CREDENTIAL_KEYS.includes(key)
    ) {
      throw new Error(
        `FYAGENT_WINDOWS_SIGNER_CREDENTIAL contains an unexpected field: ${key}`,
      );
    }
  }
  for (const key of CERTUM_REQUIRED_CREDENTIAL_KEYS) {
    if (!Object.prototype.hasOwnProperty.call(parsed, key)) {
      throw new Error(
        `FYAGENT_WINDOWS_SIGNER_CREDENTIAL is missing ${key}`,
      );
    }
  }
  assert(
    parsed.provider === CERTUM_SIMPLYSIGN_PROVIDER,
    "FYAGENT_WINDOWS_SIGNER_CREDENTIAL provider must be certum-simplysign",
  );
  assert(
    parsed.timestampUrl === CERTUM_SIMPLYSIGN_TIMESTAMP_URL,
    "FYAGENT_WINDOWS_SIGNER_CREDENTIAL timestampUrl must be the Certum RFC 3161 endpoint http://time.certum.pl",
  );
  assert(
    typeof parsed.certificateSha1 === "string" &&
      CERTUM_THUMBPRINT_PATTERN.test(parsed.certificateSha1),
    "FYAGENT_WINDOWS_SIGNER_CREDENTIAL certificateSha1 must be a 40-character SHA-1 thumbprint",
  );
  let signtoolPath = null;
  if (Object.prototype.hasOwnProperty.call(parsed, "signtoolPath")) {
    assert(
      windowsAbsoluteSigntoolPath(parsed.signtoolPath),
      "FYAGENT_WINDOWS_SIGNER_CREDENTIAL signtoolPath must be an absolute path to signtool.exe",
    );
    signtoolPath = parsed.signtoolPath;
  }
  return {
    provider: CERTUM_SIMPLYSIGN_PROVIDER,
    timestampUrl: CERTUM_SIMPLYSIGN_TIMESTAMP_URL,
    certificateSha1: parsed.certificateSha1,
    signtoolPath,
    fileDigest: "sha256",
    timestampDigest: "sha256",
  };
}

function adapterSelectsCertumSimplySign(adapterPath) {
  let bytes;
  try {
    bytes = readFileSync(adapterPath);
  } catch (error) {
    if (error && error.code === "ENOENT") return false;
    throw error;
  }
  const baseName = path.win32.basename(adapterPath).toLowerCase();
  if (baseName === CERTUM_SIMPLYSIGN_ADAPTER_NAME) return true;
  return bytes.equals(
    readFileSync(path.join(scriptDirectory, CERTUM_SIMPLYSIGN_ADAPTER_NAME)),
  );
}

function assertCertumAdapterCredential(adapterPath, environment) {
  if (!adapterSelectsCertumSimplySign(adapterPath)) return;
  const credential = Object.prototype.hasOwnProperty.call(
    environment,
    SIGNER_CREDENTIAL_NAME,
  )
    ? environment[SIGNER_CREDENTIAL_NAME]
    : undefined;
  parseCertumSimplySignCredential(credential);
}

export function parsePeImage(buffer) {
  assert(Buffer.isBuffer(buffer), "Windows installer bytes must be a Buffer");
  assert(buffer.length >= 64, "Windows installer is too small for a PE image");
  assert(
    buffer[0] === 0x4d && buffer[1] === 0x5a,
    "Windows installer is missing the DOS MZ signature",
  );
  const peOffset = buffer.readUInt32LE(0x3c);
  assert(
    peOffset <= buffer.length - 24,
    "Windows installer has an invalid PE header offset",
  );
  assert(
    buffer.subarray(peOffset, peOffset + 4).equals(Buffer.from("PE\0\0")),
    "Windows installer is missing the PE signature",
  );
  const machine = buffer.readUInt16LE(peOffset + 4);
  assert(
    SUPPORTED_LAUNCHER_PE_MACHINES.includes(machine),
    "Windows installer launcher has an unsupported PE Machine value",
  );
  const optionalHeaderSize = buffer.readUInt16LE(peOffset + 20);
  const optionalHeaderOffset = peOffset + 24;
  assert(
    optionalHeaderSize >= 120 &&
      optionalHeaderOffset + optionalHeaderSize <= buffer.length,
    "Windows installer has an invalid PE optional header",
  );
  const magic = buffer.readUInt16LE(optionalHeaderOffset);
  assert(
    magic === 0x10b || magic === 0x20b,
    "Windows installer has an unsupported PE optional-header format",
  );
  const numberOfDirectoriesOffset =
    optionalHeaderOffset + (magic === 0x20b ? 108 : 92);
  const dataDirectoryOffset =
    optionalHeaderOffset + (magic === 0x20b ? 112 : 96);
  const checksumOffset = optionalHeaderOffset + 64;
  assert(
    numberOfDirectoriesOffset + 4 <= optionalHeaderOffset + optionalHeaderSize,
    "Windows installer optional header omits the directory count",
  );
  assert(
    buffer.readUInt32LE(numberOfDirectoriesOffset) >= 5,
    "Windows installer optional header omits the security directory",
  );
  const securityDirectoryOffset = dataDirectoryOffset + 4 * 8;
  assert(
    securityDirectoryOffset + 8 <= optionalHeaderOffset + optionalHeaderSize,
    "Windows installer security directory is outside the optional header",
  );
  return {
    machine,
    checksumOffset,
    securityDirectoryOffset,
    certificateOffset: buffer.readUInt32LE(securityDirectoryOffset),
    certificateSize: buffer.readUInt32LE(securityDirectoryOffset + 4),
  };
}

function assertWinCertificateTable(buffer, offset, size) {
  assert(size >= 8, "Signed PE certificate table is too small");
  assert(size % 8 === 0, "Signed PE certificate table is not 8-byte aligned");
  assert(
    offset + size === buffer.length,
    "Signed PE certificate table must end at the final byte",
  );
  let cursor = offset;
  while (cursor < buffer.length) {
    assert(cursor + 8 <= buffer.length, "WIN_CERTIFICATE header is truncated");
    const length = buffer.readUInt32LE(cursor);
    const revision = buffer.readUInt16LE(cursor + 4);
    const certificateType = buffer.readUInt16LE(cursor + 6);
    assert(length >= 8, "WIN_CERTIFICATE length is invalid");
    assert(
      cursor + length <= buffer.length,
      "WIN_CERTIFICATE payload is truncated",
    );
    assert(revision === 0x0200, "WIN_CERTIFICATE revision must be 2.0");
    assert(
      certificateType === 0x0002,
      "WIN_CERTIFICATE must contain PKCS signed data",
    );
    cursor += Math.ceil(length / 8) * 8;
  }
  assert(cursor === buffer.length, "WIN_CERTIFICATE alignment is invalid");
}

export function assertAuthenticodeOnlyMutation(unsignedBytes, signedBytes) {
  const before = parsePeImage(unsignedBytes);
  const after = parsePeImage(signedBytes);
  assert(
    before.machine === after.machine,
    "Signing command changed the installer launcher PE Machine value",
  );
  assert(
    before.certificateOffset === 0 && before.certificateSize === 0,
    "Unsigned source PE must not contain an existing certificate table",
  );
  assert(
    signedBytes.length > unsignedBytes.length,
    "Signing command did not append Authenticode bytes",
  );
  const expectedCertificateOffset = Math.ceil(unsignedBytes.length / 8) * 8;
  assert(
    after.certificateOffset === expectedCertificateOffset,
    "Signing command placed the Authenticode table at an unexpected offset",
  );
  assertWinCertificateTable(
    signedBytes,
    after.certificateOffset,
    after.certificateSize,
  );

  const ignoredOffsets = new Set();
  for (
    let index = before.checksumOffset;
    index < before.checksumOffset + 4;
    index += 1
  ) {
    ignoredOffsets.add(index);
  }
  for (
    let index = before.securityDirectoryOffset;
    index < before.securityDirectoryOffset + 8;
    index += 1
  ) {
    ignoredOffsets.add(index);
  }
  for (let index = 0; index < unsignedBytes.length; index += 1) {
    if (ignoredOffsets.has(index)) continue;
    assert(
      unsignedBytes[index] === signedBytes[index],
      "Signing command modified installer bytes outside Authenticode-owned fields",
    );
  }
  for (
    let index = unsignedBytes.length;
    index < after.certificateOffset;
    index += 1
  ) {
    assert(
      signedBytes[index] === 0,
      "Signing command inserted non-zero PE alignment bytes",
    );
  }
}

function normalizeCertificate(value, label) {
  assertExactKeys(
    value,
    [
      "subject",
      "simpleName",
      "sha256",
      "notBefore",
      "notAfter",
      "enhancedKeyUsageOids",
    ],
    label,
  );
  assertSafeText(value.subject, `${label}.subject`, 2048);
  assertSafeText(value.simpleName, `${label}.simpleName`, 256);
  assert(
    typeof value.sha256 === "string" && SHA256_PATTERN.test(value.sha256),
    `${label}.sha256 must be a lowercase SHA-256 digest`,
  );
  assertIsoInstant(value.notBefore, `${label}.notBefore`);
  assertIsoInstant(value.notAfter, `${label}.notAfter`);
  assert(
    Date.parse(value.notBefore) < Date.parse(value.notAfter),
    `${label} validity interval is invalid`,
  );
  assert(
    Array.isArray(value.enhancedKeyUsageOids),
    `${label}.enhancedKeyUsageOids must be an array`,
  );
  const enhancedKeyUsageOids = value.enhancedKeyUsageOids.map((oid) => {
    assert(
      typeof oid === "string" && OID_PATTERN.test(oid),
      `${label} contains a malformed enhanced-key-usage OID`,
    );
    return oid;
  });
  assert(
    new Set(enhancedKeyUsageOids).size === enhancedKeyUsageOids.length &&
      enhancedKeyUsageOids.every(
        (oid, index) => index === 0 || enhancedKeyUsageOids[index - 1] < oid,
      ),
    `${label}.enhancedKeyUsageOids must be sorted and unique`,
  );
  return {
    subject: value.subject,
    simpleName: value.simpleName,
    sha256: value.sha256,
    notBefore: value.notBefore,
    notAfter: value.notAfter,
    enhancedKeyUsageOids,
  };
}

export function normalizeAuthenticodeEvidence(value) {
  assertExactKeys(
    value,
    [
      "schema",
      "status",
      "publisher",
      "signerCertificate",
      "timestampCertificate",
    ],
    "Authenticode evidence",
  );
  assert(
    value.schema === "fyagent-authenticode-evidence/v1",
    "Authenticode evidence schema is unsupported",
  );
  assertSafeText(value.status, "Authenticode status", 64);
  assert(
    value.publisher === null || typeof value.publisher === "string",
    "Authenticode publisher must be a string or null",
  );
  if (typeof value.publisher === "string") {
    assertSafeText(value.publisher, "Authenticode publisher", 256);
  }
  const signerCertificate =
    value.signerCertificate === null
      ? null
      : normalizeCertificate(
          value.signerCertificate,
          "Authenticode signer certificate",
        );
  const timestampCertificate =
    value.timestampCertificate === null
      ? null
      : normalizeCertificate(
          value.timestampCertificate,
          "Authenticode timestamp certificate",
        );
  return {
    status: value.status,
    publisher: value.publisher,
    signerCertificate,
    timestampCertificate,
  };
}

function assertStrictlyUnsigned(evidence) {
  assert(
    evidence.status === "NotSigned",
    `Unsigned Windows asset must report NotSigned; received ${evidence.status}`,
  );
  assert(
    evidence.publisher === null &&
      evidence.signerCertificate === null &&
      evidence.timestampCertificate === null,
    "Unsigned Windows asset must not expose publisher, signer, or timestamp certificate evidence",
  );
}

function assertExpectedSigned(evidence, signerConfiguration) {
  assert(
    evidence.status === "Valid",
    `Signed Windows asset must report Valid; received ${evidence.status}`,
  );
  assert(
    evidence.publisher === signerConfiguration.expectedPublisher,
    "Signed Windows asset publisher differs from the expected publisher",
  );
  assert(
    evidence.signerCertificate !== null,
    "Signed Windows asset is missing its signer certificate",
  );
  assert(
    evidence.signerCertificate.simpleName ===
      signerConfiguration.expectedPublisher,
    "Signer certificate publisher differs from the expected publisher",
  );
  assert(
    evidence.signerCertificate.sha256 ===
      signerConfiguration.expectedCertificateSha256,
    "Signer certificate SHA-256 differs from the expected certificate policy",
  );
  assert(
    evidence.signerCertificate.enhancedKeyUsageOids.includes(CODE_SIGNING_EKU),
    "Signer certificate does not include the Code Signing EKU",
  );
  assert(
    evidence.timestampCertificate !== null,
    "Signed Windows asset is missing an Authenticode timestamp certificate",
  );
  assert(
    evidence.timestampCertificate.enhancedKeyUsageOids.includes(
      TIMESTAMPING_EKU,
    ),
    "Authenticode timestamp certificate does not include the Time Stamping EKU",
  );
}

function runAuthenticodeProbe(assetPath) {
  assert(
    process.platform === "win32",
    "Authenticode evidence must be collected on a native Windows runner",
  );
  const result = spawnSync(
    "pwsh",
    [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-File",
      authenticodeEvidenceScript,
      "-ArtifactPath",
      assetPath,
    ],
    {
      encoding: "utf8",
      env: childEnvironment(),
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    },
  );
  if (result.error) throw result.error;
  assert(
    result.status === 0,
    `Authenticode evidence probe exited with ${result.status}`,
  );
  let evidence;
  try {
    evidence = JSON.parse(result.stdout.trim());
  } catch {
    throw new Error("Authenticode evidence probe returned malformed JSON");
  }
  return evidence;
}

// The in-repo Certum / SimplySign adapter is
// scripts/release/windows-signer-certum-simplysign.ps1. CI does not execute
// that checkout path: sign-windows-formal base64-decodes the adapter secret
// onto a temporary absolute .ps1 and sets FYAGENT_WINDOWS_SIGNER_ADAPTER.
// Repository variable FYAGENT_WINDOWS_SIGNING_MODE stays unsigned until
// docs/release/release-identity.md is filled in.
function runSignerAdapter({ adapterPath, assetPath, architecture }) {
  assert(
    process.platform === "win32",
    "Windows signing must run on a native Windows runner",
  );
  const resolvedAdapter = assertRegularFile(
    adapterPath,
    "Windows signer adapter",
  );
  const result = spawnSync(
    "pwsh",
    [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-File",
      resolvedAdapter,
      "-ArtifactPath",
      assetPath,
      "-Architecture",
      architecture,
    ],
    {
      env: childEnvironment({ preserveCredential: true }),
      stdio: "ignore",
      windowsHide: true,
    },
  );
  if (result.error) throw result.error;
  assert(
    result.status === 0,
    `Windows signer adapter exited with ${result.status}`,
  );
}

export function createAssetSigningRecord(
  { assetPath, architecture, version, sourceSha, environment = process.env },
  dependencies = {},
) {
  assertStableVersion(version);
  assertSourceSha(sourceSha);
  assertArchitecture(architecture);
  const resolveRegularFile =
    dependencies.resolveRegularFile ?? assertRegularFile;
  const resolvedAsset = resolveRegularFile(assetPath, "Windows installer");
  const expectedName = expectedWindowsInstallerName(version, architecture);
  assert(
    path.basename(resolvedAsset) === expectedName,
    `Windows installer name must be ${expectedName}`,
  );
  const signerConfiguration = resolveSignerConfiguration(environment);
  if (signerConfiguration !== null) {
    assertCertumAdapterCredential(
      signerConfiguration.adapterPath,
      environment,
    );
  }
  const probeAuthenticode =
    dependencies.probeAuthenticode ?? runAuthenticodeProbe;
  const invokeSigner = dependencies.invokeSigner ?? runSignerAdapter;
  const unsignedBytes = readFileSync(resolvedAsset);
  const unsignedPe = parsePeImage(unsignedBytes);
  assert(
    unsignedPe.certificateOffset === 0 && unsignedPe.certificateSize === 0,
    "Unsigned source PE security directory must be empty",
  );
  const beforeEvidence = normalizeAuthenticodeEvidence(
    probeAuthenticode(resolvedAsset),
  );
  assertStrictlyUnsigned(beforeEvidence);

  let finalBytes = unsignedBytes;
  let finalEvidence = beforeEvidence;
  let mode = "unsigned";
  if (signerConfiguration !== null) {
    invokeSigner({
      adapterPath: signerConfiguration.adapterPath,
      assetPath: resolvedAsset,
      architecture,
    });
    const postSigningAsset = resolveRegularFile(
      resolvedAsset,
      "Post-sign Windows installer",
    );
    assert(
      postSigningAsset === resolvedAsset,
      "Windows signer adapter changed the installer real path",
    );
    finalBytes = readFileSync(postSigningAsset);
    assertAuthenticodeOnlyMutation(unsignedBytes, finalBytes);
    finalEvidence = normalizeAuthenticodeEvidence(
      probeAuthenticode(resolvedAsset),
    );
    assertExpectedSigned(finalEvidence, signerConfiguration);
    mode = "signed";
  }

  return {
    schema: WINDOWS_SIGNING_ASSET_SCHEMA,
    product: PRODUCT_NAME,
    version,
    sourceSha,
    mode,
    asset: {
      name: expectedName,
      architecture,
      sizeBytes: finalBytes.length,
      sha256: sha256(finalBytes),
      signature: finalEvidence,
    },
  };
}

export function transformWindowsCandidate(
  { assetPath, architecture, version, sourceSha, environment = process.env },
  dependencies = {},
) {
  assertStableVersion(version);
  assertSourceSha(sourceSha);
  assertArchitecture(architecture);
  const resolveRegularFile =
    dependencies.resolveRegularFile ?? assertRegularFile;
  const resolvedAsset = resolveRegularFile(
    assetPath,
    "Untrusted formal Windows candidate",
  );
  const expectedName = expectedWindowsInstallerName(version, architecture);
  assert(
    path.basename(resolvedAsset) === expectedName,
    `Windows installer name must be ${expectedName}`,
  );
  const signerConfiguration = resolveSignerConfiguration(environment);
  if (signerConfiguration !== null) {
    assertCertumAdapterCredential(
      signerConfiguration.adapterPath,
      environment,
    );
  }
  const probeAuthenticode =
    dependencies.probeAuthenticode ?? runAuthenticodeProbe;
  const invokeSigner = dependencies.invokeSigner ?? runSignerAdapter;
  const unsignedBytes = readFileSync(resolvedAsset);
  const unsignedPe = parsePeImage(unsignedBytes);
  assert(
    unsignedPe.certificateOffset === 0 && unsignedPe.certificateSize === 0,
    "Unsigned source PE security directory must be empty",
  );
  assertStrictlyUnsigned(
    normalizeAuthenticodeEvidence(probeAuthenticode(resolvedAsset)),
  );
  const postProbeAsset = resolveRegularFile(
    resolvedAsset,
    "Post-probe untrusted formal Windows candidate",
  );
  assert(
    postProbeAsset === resolvedAsset &&
      readFileSync(postProbeAsset).equals(unsignedBytes),
    "Unsigned formal Windows candidate changed during initial proof",
  );
  if (signerConfiguration !== null) {
    invokeSigner({
      adapterPath: signerConfiguration.adapterPath,
      assetPath: resolvedAsset,
      architecture,
    });
  }
}

function assertSealedVerificationPolicy({
  mode,
  expectedPublisher,
  expectedCertificateSha256,
}) {
  assert(
    mode === "unsigned" || mode === "provider",
    "Formal Windows verification mode must be unsigned or provider",
  );
  if (mode === "unsigned") {
    assert(
      expectedPublisher === undefined &&
        expectedCertificateSha256 === undefined,
      "Unsigned formal Windows verification must not define provider policy",
    );
    return null;
  }
  assertSafeText(expectedPublisher, "Expected Windows signing publisher", 256);
  assert(
    typeof expectedCertificateSha256 === "string" &&
      SHA256_PATTERN.test(expectedCertificateSha256),
    "Expected Windows signing certificate must be a lowercase SHA-256 digest",
  );
  return { expectedPublisher, expectedCertificateSha256 };
}

export function verifySealedWindowsCandidate(
  {
    rawAssetPath,
    candidateAssetPath,
    architecture,
    version,
    sourceSha,
    mode,
    expectedPublisher,
    expectedCertificateSha256,
  },
  dependencies = {},
) {
  assertStableVersion(version);
  assertSourceSha(sourceSha);
  assertArchitecture(architecture);
  const signingPolicy = assertSealedVerificationPolicy({
    mode,
    expectedPublisher,
    expectedCertificateSha256,
  });
  const resolveRegularFile =
    dependencies.resolveRegularFile ?? assertRegularFile;
  const probeAuthenticode =
    dependencies.probeAuthenticode ?? runAuthenticodeProbe;
  const expectedName = expectedWindowsInstallerName(version, architecture);
  const resolvedRaw = resolveRegularFile(
    rawAssetPath,
    "Frozen raw Windows candidate",
  );
  const resolvedCandidate = resolveRegularFile(
    candidateAssetPath,
    "Formal Windows candidate",
  );
  assert(
    resolvedRaw !== resolvedCandidate,
    "Frozen raw and formal Windows candidates must be distinct files",
  );
  for (const [resolvedPath, label] of [
    [resolvedRaw, "Frozen raw Windows candidate"],
    [resolvedCandidate, "Formal Windows candidate"],
  ]) {
    assert(
      path.basename(resolvedPath) === expectedName,
      `${label} name must be ${expectedName}`,
    );
  }

  const rawBytes = readFileSync(resolvedRaw);
  const rawPe = parsePeImage(rawBytes);
  assert(
    rawPe.certificateOffset === 0 && rawPe.certificateSize === 0,
    "Frozen raw Windows candidate security directory must be empty",
  );
  assertStrictlyUnsigned(
    normalizeAuthenticodeEvidence(probeAuthenticode(resolvedRaw)),
  );
  assert(
    resolveRegularFile(
      resolvedRaw,
      "Post-probe frozen raw Windows candidate",
    ) === resolvedRaw && readFileSync(resolvedRaw).equals(rawBytes),
    "Frozen raw Windows candidate changed during independent verification",
  );

  const candidateBytes = readFileSync(resolvedCandidate);
  parsePeImage(candidateBytes);
  let finalEvidence;
  let recordMode;
  if (mode === "unsigned") {
    assert(
      candidateBytes.equals(rawBytes),
      "Unsigned formal Windows candidate must be byte-identical to the frozen raw candidate",
    );
    finalEvidence = normalizeAuthenticodeEvidence(
      probeAuthenticode(resolvedCandidate),
    );
    assertStrictlyUnsigned(finalEvidence);
    recordMode = "unsigned";
  } else {
    assertAuthenticodeOnlyMutation(rawBytes, candidateBytes);
    finalEvidence = normalizeAuthenticodeEvidence(
      probeAuthenticode(resolvedCandidate),
    );
    assertExpectedSigned(finalEvidence, signingPolicy);
    recordMode = "signed";
  }
  assert(
    resolveRegularFile(
      resolvedCandidate,
      "Post-probe formal Windows candidate",
    ) === resolvedCandidate &&
      readFileSync(resolvedCandidate).equals(candidateBytes),
    "Formal Windows candidate changed during independent verification",
  );

  return {
    schema: WINDOWS_SIGNING_ASSET_SCHEMA,
    product: PRODUCT_NAME,
    version,
    sourceSha,
    mode: recordMode,
    asset: {
      name: expectedName,
      architecture,
      sizeBytes: candidateBytes.length,
      sha256: sha256(candidateBytes),
      signature: finalEvidence,
    },
  };
}

function normalizeAssetFragment(
  value,
  expectedArchitecture,
  version,
  sourceSha,
) {
  assertExactKeys(
    value,
    ["schema", "product", "version", "sourceSha", "mode", "asset"],
    `${expectedArchitecture} Windows signing fragment`,
  );
  assert(
    value.schema === WINDOWS_SIGNING_ASSET_SCHEMA,
    `${expectedArchitecture} Windows signing fragment schema is unsupported`,
  );
  assert(value.product === PRODUCT_NAME, "Windows signing product drifted");
  assert(value.version === version, "Windows signing version drifted");
  assert(value.sourceSha === sourceSha, "Windows signing source SHA drifted");
  assert(
    value.mode === "unsigned" || value.mode === "signed",
    "Windows signing mode must be unsigned or signed",
  );
  assertExactKeys(
    value.asset,
    ["name", "architecture", "sizeBytes", "sha256", "signature"],
    `${expectedArchitecture} Windows signing asset`,
  );
  assert(
    value.asset.name ===
      expectedWindowsInstallerName(version, expectedArchitecture),
    `${expectedArchitecture} Windows installer name drifted`,
  );
  assert(
    value.asset.architecture === expectedArchitecture,
    `${expectedArchitecture} Windows signing architecture drifted`,
  );
  assert(
    Number.isSafeInteger(value.asset.sizeBytes) && value.asset.sizeBytes > 0,
    `${expectedArchitecture} Windows installer size is invalid`,
  );
  assert(
    typeof value.asset.sha256 === "string" &&
      SHA256_PATTERN.test(value.asset.sha256),
    `${expectedArchitecture} Windows installer SHA-256 is invalid`,
  );
  const signature = normalizeAuthenticodeEvidence({
    schema: "fyagent-authenticode-evidence/v1",
    ...value.asset.signature,
  });
  if (value.mode === "unsigned") {
    assertStrictlyUnsigned(signature);
  } else {
    assert(
      signature.status === "Valid" &&
        typeof signature.publisher === "string" &&
        signature.signerCertificate !== null &&
        signature.timestampCertificate !== null,
      `${expectedArchitecture} signed Windows evidence is incomplete`,
    );
    assert(
      signature.publisher === signature.signerCertificate.simpleName,
      `${expectedArchitecture} signed Windows publisher is inconsistent`,
    );
    assert(
      signature.signerCertificate.enhancedKeyUsageOids.includes(
        CODE_SIGNING_EKU,
      ),
      `${expectedArchitecture} signer certificate lacks the Code Signing EKU`,
    );
    assert(
      signature.timestampCertificate.enhancedKeyUsageOids.includes(
        TIMESTAMPING_EKU,
      ),
      `${expectedArchitecture} timestamp certificate lacks the Time Stamping EKU`,
    );
  }
  return {
    mode: value.mode,
    asset: {
      name: value.asset.name,
      architecture: expectedArchitecture,
      sizeBytes: value.asset.sizeBytes,
      sha256: value.asset.sha256,
      signature,
    },
  };
}

function readJson(filePath, label) {
  const resolved = assertRegularFile(filePath, label);
  try {
    return JSON.parse(readFileSync(resolved, "utf8"));
  } catch {
    throw new Error(`${label} contains malformed JSON`);
  }
}

export function aggregateSigningStatus({
  x64StatusPath,
  arm64StatusPath,
  assetsDirectory,
  version,
  sourceSha,
}) {
  assertStableVersion(version);
  assertSourceSha(sourceSha);
  const fragments = ARCHITECTURES.map((architecture) => {
    const statusPath = architecture === "x64" ? x64StatusPath : arm64StatusPath;
    return normalizeAssetFragment(
      readJson(statusPath, `${architecture} Windows signing status`),
      architecture,
      version,
      sourceSha,
    );
  });
  assert(
    fragments[0].mode === fragments[1].mode,
    "Windows x64 and arm64 signing modes are inconsistent",
  );
  if (fragments[0].mode === "signed") {
    const x64Signature = fragments[0].asset.signature;
    const arm64Signature = fragments[1].asset.signature;
    assert(
      x64Signature.publisher === arm64Signature.publisher,
      "Windows x64 and arm64 publishers are inconsistent",
    );
    assert(
      x64Signature.signerCertificate.sha256 ===
        arm64Signature.signerCertificate.sha256,
      "Windows x64 and arm64 signer certificates are inconsistent",
    );
    assert(
      JSON.stringify(x64Signature.signerCertificate) ===
        JSON.stringify(arm64Signature.signerCertificate),
      "Windows x64 and arm64 signer certificate policies are inconsistent",
    );
  }

  const assets = fragments.map(({ asset }) => {
    const installerPath = path.join(assetsDirectory, asset.name);
    const resolvedInstaller = assertRegularFile(
      installerPath,
      `${asset.architecture} Windows installer`,
    );
    const bytes = readFileSync(resolvedInstaller);
    parsePeImage(bytes);
    assert(
      bytes.length === asset.sizeBytes,
      `${asset.architecture} Windows installer size differs from native signing evidence`,
    );
    assert(
      sha256(bytes) === asset.sha256,
      `${asset.architecture} Windows installer SHA-256 differs from native signing evidence`,
    );
    return {
      name: asset.name,
      architecture: asset.architecture,
      sizeBytes: asset.sizeBytes,
      sha256: asset.sha256,
      sourceSha,
      attestation: {
        bundle: ATTESTATION_BUNDLE_NAME,
        subjectName: asset.name,
        subjectDigest: `sha256:${asset.sha256}`,
      },
      signature: asset.signature,
    };
  });

  return {
    schema: WINDOWS_SIGNING_STATUS_SCHEMA,
    product: PRODUCT_NAME,
    version,
    sourceSha,
    mode: fragments[0].mode,
    assets,
  };
}

function parseOptions(argumentsList) {
  const options = new Map();
  for (let index = 0; index < argumentsList.length; index += 2) {
    const name = argumentsList[index];
    const value = argumentsList[index + 1];
    assert(
      typeof name === "string" && name.startsWith("--") && value !== undefined,
      "Windows signing CLI options must be --name value pairs",
    );
    assert(!options.has(name), `Duplicate Windows signing option: ${name}`);
    options.set(name, value);
  }
  return options;
}

function requireExactOptions(options, names) {
  const actual = [...options.keys()].sort();
  const expected = [...names].sort();
  assert(
    actual.length === expected.length &&
      actual.every((name, index) => name === expected[index]),
    `Expected exactly these Windows signing options: ${expected.join(", ")}`,
  );
}

export function runWindowsSigningCli(
  argumentsList = process.argv.slice(2),
  environment = process.env,
) {
  const [command, ...optionArguments] = argumentsList;
  const options = parseOptions(optionArguments);
  if (command === "asset") {
    requireExactOptions(options, [
      "--asset",
      "--architecture",
      "--version",
      "--source-sha",
      "--output",
    ]);
    const record = createAssetSigningRecord({
      assetPath: options.get("--asset"),
      architecture: options.get("--architecture"),
      version: options.get("--version"),
      sourceSha: options.get("--source-sha"),
      environment,
    });
    writeFileSync(
      options.get("--output"),
      `${JSON.stringify(record, null, 2)}\n`,
      { flag: "wx" },
    );
    return;
  }
  if (command === "transform") {
    requireExactOptions(options, [
      "--asset",
      "--architecture",
      "--version",
      "--source-sha",
    ]);
    transformWindowsCandidate({
      assetPath: options.get("--asset"),
      architecture: options.get("--architecture"),
      version: options.get("--version"),
      sourceSha: options.get("--source-sha"),
      environment,
    });
    return;
  }
  if (command === "verify-sealed") {
    const mode = options.get("--mode");
    const commonOptions = [
      "--raw",
      "--candidate",
      "--architecture",
      "--version",
      "--source-sha",
      "--mode",
      "--output",
    ];
    requireExactOptions(
      options,
      mode === "provider"
        ? [
            ...commonOptions,
            "--expected-publisher",
            "--expected-certificate-sha256",
          ]
        : commonOptions,
    );
    const record = verifySealedWindowsCandidate({
      rawAssetPath: options.get("--raw"),
      candidateAssetPath: options.get("--candidate"),
      architecture: options.get("--architecture"),
      version: options.get("--version"),
      sourceSha: options.get("--source-sha"),
      mode,
      expectedPublisher: options.get("--expected-publisher"),
      expectedCertificateSha256: options.get("--expected-certificate-sha256"),
    });
    writeFileSync(
      options.get("--output"),
      `${JSON.stringify(record, null, 2)}\n`,
      { flag: "wx" },
    );
    return;
  }
  if (command === "aggregate") {
    requireExactOptions(options, [
      "--x64-status",
      "--arm64-status",
      "--assets-directory",
      "--version",
      "--source-sha",
      "--output",
    ]);
    const status = aggregateSigningStatus({
      x64StatusPath: options.get("--x64-status"),
      arm64StatusPath: options.get("--arm64-status"),
      assetsDirectory: options.get("--assets-directory"),
      version: options.get("--version"),
      sourceSha: options.get("--source-sha"),
    });
    writeFileSync(
      options.get("--output"),
      `${JSON.stringify(status, null, 2)}\n`,
      { flag: "wx" },
    );
    return;
  }
  throw new Error(
    "Usage: windows-signing.mjs asset ... | transform ... | verify-sealed ... | aggregate ...",
  );
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    runWindowsSigningCli();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
