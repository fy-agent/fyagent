export type DevReleaseMode = "preflight" | "formal";
export type DevReleaseDispatchMode = "preflight" | "formal" | "formal-repair";

export interface RepositoryIdentity {
  readonly nameWithOwner: string;
  readonly id: string;
}

export interface ReleaseEventIdentity {
  readonly dispatchMode: DevReleaseDispatchMode | null;
  readonly dispatchSourceSha: string | null;
  readonly name: "workflow_dispatch" | "push" | string;
  readonly ref: string;
  readonly refName: string;
  readonly refType: "branch" | "tag" | string;
  readonly sha: string;
}

export interface ReleaseWorkflowIdentity {
  readonly name: string;
  readonly path: string;
  readonly ref: string;
  readonly sha: string;
}

export interface ReleaseCandidateIdentity {
  readonly canonicalVersion: string;
  readonly releaseTag: string;
  readonly sourceSha: string;
}

export interface RemoteDevIdentity {
  readonly name: string;
  readonly ref: string;
  readonly headSha: string;
}

export interface RemoteTagIdentity {
  readonly ref: string;
  readonly refObject: {
    readonly type: string;
    readonly sha: string;
  };
  readonly tagObject: {
    readonly sha: string;
    readonly name: string;
    readonly target: {
      readonly type: string;
      readonly sha: string;
    };
  } | null;
}

export interface DevReleaseEligibilityInput {
  readonly schema: "fyagent-dev-release-eligibility-input/v2";
  readonly repository: RepositoryIdentity;
  readonly event: ReleaseEventIdentity;
  readonly workflow: ReleaseWorkflowIdentity;
  readonly candidate: ReleaseCandidateIdentity;
  readonly remoteDev: RemoteDevIdentity;
  readonly remoteTag: RemoteTagIdentity | null;
}

export interface DevReleaseEligibilityOutput {
  readonly appVersion: string;
  readonly releaseTag: string;
  readonly sourceSha: string;
  readonly workflowSha: string;
  readonly ciRunId: string | null;
  readonly ciRunAttempt: string | null;
  readonly mode: DevReleaseMode;
  readonly repair: boolean;
}

export const DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA: "fyagent-dev-release-eligibility-input/v2";
export const EXPECTED_REPOSITORY: "fy-agent/fyagent";
export const EXPECTED_REPOSITORY_ID: "1313497021";
export const RELEASE_WORKFLOW_NAME: "Release";
export const RELEASE_WORKFLOW_PATH: ".github/workflows/release.yml";
export const CI_WORKFLOW_NAME: "CI";
export const CI_WORKFLOW_PATH: ".github/workflows/ci.yml";
export const PREFLIGHT_WORKFLOW_BRANCH: "main";
export const PREFLIGHT_WORKFLOW_REF: "refs/heads/main";
export const FORMAL_BRANCH: "main";
export const FORMAL_REF: "refs/heads/main";
export const REQUIRED_JOB_NAME: "CI / Required";
export const FORMAL_REPAIR_RELEASE_TAG: "v0.4.15";
export const FORMAL_REPAIR_SOURCE_SHA: "b581222135e3db66e0e6e3fdba908662bc60dcda";
export const FORMAL_REPAIR_PATHS: readonly string[];

export function assertFormalRepairDifference(input: {
  sourceSha: string;
  workflowSha: string;
  ancestor: boolean;
  diffStatus: string;
  tagWorkflow: string;
  executingWorkflow: string;
}): void;

export function evaluateDevReleaseEligibility(
  input: DevReleaseEligibilityInput,
  expectedFrozen?: DevReleaseEligibilityOutput,
): Readonly<DevReleaseEligibilityOutput>;
