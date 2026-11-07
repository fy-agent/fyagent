export interface OfflineSigningPolicy {
  schema: "fyagent-windows-offline-signing-policy/v1";
  publisher: string;
  certificateSha256: string;
}

export interface OfflineReturnAsset {
  architecture: "x64" | "arm64";
  name: string;
  rawSha256: string;
  signedSha256: string;
}

export interface OfflineReturnManifest {
  schema: "fyagent-windows-offline-signing-return/v1";
  sourceRunId: string;
  sourceRunAttempt: string;
  sourceSha: string;
  version: string;
  repositoryId: string;
  assets: OfflineReturnAsset[];
}

export interface OfflineSealedBinding {
  schema: "fyagent-windows-offline-sealed-binding/v1";
  product: "FyAgent";
  sourceRunId: string;
  sourceRunAttempt: string;
  sourceSha: string;
  version: string;
  repositoryId: string;
  workflow: string;
  publisher: string;
  certificateSha256: string;
  assets: OfflineReturnAsset[];
}

export interface NormalizedReleaseRun {
  id: string;
  runAttempt: string;
  headSha: string;
  status: unknown;
  conclusion: unknown;
  event: unknown;
  name: unknown;
  path: unknown;
  repositoryId: string;
}

export function expectedOfflineReturnPaths(version: string): string[];

export function readOfflineSigningPolicy(value: object): OfflineSigningPolicy;

export function readOfflineReturnManifest(
  value: object,
  version: string,
): OfflineReturnManifest;

export function assertOfflineReturnTree(paths: readonly string[], version: string): void;

export function normalizeReleaseRun(run: object): NormalizedReleaseRun;

export function admitOfflineReturn(input: {
  policy: object;
  returnManifest: object;
  treePaths: readonly string[];
  run: object;
  rawBytesByArchitecture: { x64: Buffer; arm64: Buffer };
  signedBytesByArchitecture: { x64: Buffer; arm64: Buffer };
  version: string;
  sourceSha: string;
  sourceRunId: string;
  sourceRunAttempt: string;
}): OfflineSealedBinding;

export function assertOfflinePublicationGate(
  status: {
    product?: string;
    version?: string;
    sourceSha?: string;
    mode?: string;
    assets?: Array<{
      architecture?: string;
      name?: string;
      sha256?: string;
      signature?: {
        status?: string;
        publisher?: string;
        signerCertificate?: { sha256?: string } | null;
      };
    }>;
  },
  binding: OfflineSealedBinding,
): boolean;

export function validateOfflineDispatch(input: {
  sourceRunId: string;
  sourceRunAttempt: string;
  sourceSha: string;
  returnSha: string;
}): {
  source_run_id: string;
  source_run_attempt: string;
  source_sha: string;
  return_sha: string;
};
