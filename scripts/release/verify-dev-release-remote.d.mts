import type {
  DevReleaseEligibilityInput,
  DevReleaseEligibilityOutput,
} from "./dev-release-eligibility.mjs";

export interface DevReleaseRemoteContext {
  readonly token: string;
  readonly apiBase: string;
  readonly repository: string;
  readonly repositoryId: string;
  readonly eventName: string;
  readonly ref: string;
  readonly refName: string;
  readonly refType: string;
  readonly eventSha: string;
  readonly workflowName: string;
  readonly workflowRef: string;
  readonly workflowSha: string;
  readonly appVersion: string;
  readonly releaseTag: string;
  readonly sourceSha: string;
  readonly dispatchMode?: "preflight" | "formal" | string | null;
  readonly dispatchSourceSha?: string | null;
}

export interface FetchOptions {
  readonly fetchImpl?: typeof fetch;
}

export function contextFromEnvironment(
  env?: NodeJS.ProcessEnv,
): DevReleaseRemoteContext;

export function collectDevReleaseRemoteEvidence(
  context: DevReleaseRemoteContext,
  options?: FetchOptions,
): Promise<DevReleaseEligibilityInput>;

export function verifyDevReleaseRemote(
  context: DevReleaseRemoteContext,
  options?: FetchOptions & {
    readonly expectedFrozen?: DevReleaseEligibilityOutput;
  },
): Promise<{
  readonly evidence: DevReleaseEligibilityInput;
  readonly result: Readonly<DevReleaseEligibilityOutput>;
}>;
