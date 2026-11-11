import type { ReleaseIdentity } from "./release-contract.mjs";

export interface FormalOfflineAdmissionAsset {
  architecture: string;
  name: string;
  rawSha256: string;
  signedSha256: string;
}

export interface FormalOfflineAdmissionManifest {
  schema: string;
  version: string;
  artifactSourceSha: string;
  productTreeHash: string;
  preflightRunId: string;
  preflightRunAttempt: string;
  offlineRunId: string;
  offlineRunAttempt: string;
  assets: FormalOfflineAdmissionAsset[];
}

export interface FormalOfflineAdmissionProof {
  schema: string;
  version: string;
  releaseSha: string;
  artifactSourceSha: string;
  productTreeHash: string;
  diffPaths: string[];
  preflightRunId: string;
  preflightRunAttempt: string;
  offlineRunId: string;
  offlineRunAttempt: string;
  publisher: string;
  certificateSha256: string;
  assets: FormalOfflineAdmissionAsset[];
}

export interface FormalOfflineAdmissionInput {
  manifest: FormalOfflineAdmissionManifest;
  policy: {
    schema: string;
    publisher: string;
    certificateSha256: string;
  };
  releaseSha: string;
  ancestor: boolean;
  artifactLsTree: string;
  releaseLsTree: string;
  diffStatus: string;
  artifactWorkflow: string;
  releaseWorkflow: string;
  preflightRun: object;
  offlineRun: object;
  binding: object;
  rawBytesByArchitecture: { x64: Buffer; arm64: Buffer };
  signedBytesByArchitecture: { x64: Buffer; arm64: Buffer };
}

export const WORKFLOW_ONLY_PATHS: readonly string[];

export function readFormalOfflineAdmissionManifest(
  value: object,
): FormalOfflineAdmissionManifest;

export function classifyWindowsByteSource(input: {
  mode: string;
  version: string;
  manifest: object | null;
}): "offline-sealed" | "rebuild";

export function productTreeHash(lsTreeText: string): string;

export function productJobFingerprint(workflowText: string): string;

export function assertProductJobsPreserved(
  artifactWorkflow: string,
  releaseWorkflow: string,
): void;

export function proveFormalOfflineAdmission(
  input: FormalOfflineAdmissionInput,
): FormalOfflineAdmissionProof;

export function assertAdmittedWindowsMetadata(
  sourceDirectory: string,
  proof: FormalOfflineAdmissionProof,
): void;

export function stageMetadataForAggregate(input: {
  sourceDirectory: string;
  destinationDirectory: string;
  proof: FormalOfflineAdmissionProof;
  releaseIdentity: ReleaseIdentity;
}): string;

export function bindFormalSigningStatus(
  status: {
    schema?: string;
    product?: string;
    version?: string;
    sourceSha?: string;
    mode?: string;
    assets?: Array<{
      architecture?: string;
      name?: string;
      sha256?: string;
      sourceSha?: string;
      signature?: {
        status?: string;
        publisher?: string | null;
        signerCertificate?: { sha256?: string } | null;
      };
    }>;
  },
  proof: FormalOfflineAdmissionProof,
): {
  schema: string;
  product: string;
  version: string;
  sourceSha: string;
  mode: "signed";
  offlineAdmission: FormalOfflineAdmissionProof;
  assets: Array<{
    sourceSha?: string;
    architecture?: string;
    name?: string;
    sha256?: string;
    signature?: {
      status?: string;
      publisher?: string | null;
      signerCertificate?: { sha256?: string } | null;
    };
  }>;
};

export function assertPublishedAdmission(input: {
  proof: FormalOfflineAdmissionProof;
  signingStatus: object;
  buildMetadata: { offlineAdmission?: FormalOfflineAdmissionProof };
  signedBytesByArchitecture: { x64: Buffer; arm64: Buffer };
}): true;
