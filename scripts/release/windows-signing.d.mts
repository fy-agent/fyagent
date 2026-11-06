export type WindowsSigningArchitecture = "x64" | "arm64";
export type WindowsSigningMode = "unsigned" | "signed";

export interface CertificateEvidence {
  subject: string;
  simpleName: string;
  sha256: string;
  notBefore: string;
  notAfter: string;
  enhancedKeyUsageOids: string[];
}

export interface AuthenticodeEvidence {
  status: string;
  publisher: string | null;
  signerCertificate: CertificateEvidence | null;
  timestampCertificate: CertificateEvidence | null;
}

export interface RawAuthenticodeEvidence extends AuthenticodeEvidence {
  schema: "fyagent-authenticode-evidence/v1";
}

export interface WindowsSigningAsset {
  name: string;
  architecture: WindowsSigningArchitecture;
  sizeBytes: number;
  sha256: string;
  signature: AuthenticodeEvidence;
}

export interface WindowsSigningAssetRecord {
  schema: "fyagent-windows-signing-asset/v1";
  product: "FyAgent";
  version: string;
  sourceSha: string;
  mode: WindowsSigningMode;
  asset: WindowsSigningAsset;
}

export interface WindowsSigningStatusAsset extends WindowsSigningAsset {
  sourceSha: string;
  attestation: {
    bundle: "artifact-attestation.sigstore.json";
    subjectName: string;
    subjectDigest: string;
  };
}

export interface WindowsSigningStatus {
  schema: "fyagent-windows-signing-status/v1";
  product: "FyAgent";
  version: string;
  sourceSha: string;
  mode: WindowsSigningMode;
  assets: WindowsSigningStatusAsset[];
}

export interface SignerConfiguration {
  adapterPath: string;
  expectedPublisher: string;
  expectedCertificateSha256: string;
}

export interface CertumSimplySignCredential {
  provider: "certum-simplysign";
  timestampUrl: "http://time.certum.pl";
  certificateSha1: string;
  signtoolPath: string | null;
  fileDigest: "sha256";
  timestampDigest: "sha256";
}

export const WINDOWS_SIGNING_ASSET_SCHEMA: "fyagent-windows-signing-asset/v1";
export const WINDOWS_SIGNING_STATUS_SCHEMA: "fyagent-windows-signing-status/v1";
export const WINDOWS_SIGNING_STATUS_NAME: "signing-status.json";
export const ATTESTATION_BUNDLE_NAME: "artifact-attestation.sigstore.json";

export function expectedWindowsInstallerName(
  version: string,
  architecture: WindowsSigningArchitecture,
): string;

export function resolveSignerConfiguration(
  environment: Record<string, string | undefined>,
): SignerConfiguration | null;

export const CERTUM_SIMPLYSIGN_ADAPTER_NAME: "windows-signer-certum-simplysign.ps1";
export const CERTUM_SIMPLYSIGN_PROVIDER: "certum-simplysign";
export const CERTUM_SIMPLYSIGN_TIMESTAMP_URL: "http://time.certum.pl";

export function parseCertumSimplySignCredential(
  value: string | undefined | null,
): CertumSimplySignCredential;

export function parsePeImage(buffer: Buffer): {
  machine: number;
  checksumOffset: number;
  securityDirectoryOffset: number;
  certificateOffset: number;
  certificateSize: number;
};

export function assertAuthenticodeOnlyMutation(
  unsignedBytes: Buffer,
  signedBytes: Buffer,
): void;

export function normalizeAuthenticodeEvidence(
  value: unknown,
): AuthenticodeEvidence;

export function createAssetSigningRecord(
  input: {
    assetPath: string;
    architecture: WindowsSigningArchitecture;
    version: string;
    sourceSha: string;
    environment?: Record<string, string | undefined>;
  },
  dependencies?: {
    probeAuthenticode?: (assetPath: string) => RawAuthenticodeEvidence;
    resolveRegularFile?: (filePath: string, label: string) => string;
    invokeSigner?: (input: {
      adapterPath: string;
      assetPath: string;
      architecture: WindowsSigningArchitecture;
    }) => void;
  },
): WindowsSigningAssetRecord;

export function transformWindowsCandidate(
  input: {
    assetPath: string;
    architecture: WindowsSigningArchitecture;
    version: string;
    sourceSha: string;
    environment?: Record<string, string | undefined>;
  },
  dependencies?: {
    probeAuthenticode?: (assetPath: string) => RawAuthenticodeEvidence;
    resolveRegularFile?: (filePath: string, label: string) => string;
    invokeSigner?: (input: {
      adapterPath: string;
      assetPath: string;
      architecture: WindowsSigningArchitecture;
    }) => void;
  },
): void;

export function verifySealedWindowsCandidate(
  input: {
    rawAssetPath: string;
    candidateAssetPath: string;
    architecture: WindowsSigningArchitecture;
    version: string;
    sourceSha: string;
    mode: "unsigned" | "provider";
    expectedPublisher?: string;
    expectedCertificateSha256?: string;
  },
  dependencies?: {
    probeAuthenticode?: (assetPath: string) => RawAuthenticodeEvidence;
    resolveRegularFile?: (filePath: string, label: string) => string;
  },
): WindowsSigningAssetRecord;

export function aggregateSigningStatus(input: {
  x64StatusPath: string;
  arm64StatusPath: string;
  assetsDirectory: string;
  version: string;
  sourceSha: string;
}): WindowsSigningStatus;

export function runWindowsSigningCli(
  argumentsList?: string[],
  environment?: Record<string, string | undefined>,
): void;
