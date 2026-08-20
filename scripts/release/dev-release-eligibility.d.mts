export type DevReleaseMode = "preflight" | "formal";

export interface RepositoryIdentity {
  readonly nameWithOwner: string;
  readonly id: string;
}

export interface ReleaseEventIdentity {
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

export interface AnnotatedRemoteTagIdentity {
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
  };
}

export interface CiWorkflowIdentity {
  readonly id: string;
  readonly name: string;
  readonly path: string;
  readonly state: string;
  readonly repository: RepositoryIdentity;
}

export type GitHubActionsStatus =
  | "completed"
  | "in_progress"
  | "pending"
  | "queued"
  | "requested"
  | "waiting";

export type GitHubActionsConclusion =
  | "action_required"
  | "cancelled"
  | "failure"
  | "neutral"
  | "skipped"
  | "stale"
  | "startup_failure"
  | "success"
  | "timed_out"
  | null;

export interface CiRunSummary {
  readonly id: string;
  readonly runNumber: string;
  readonly runAttempt: string;
  readonly checkSuiteId: string;
  readonly repository: RepositoryIdentity;
  readonly headRepository: RepositoryIdentity;
  readonly workflow: {
    readonly id: string;
    readonly name: string;
    readonly path: string;
  };
  readonly event: string;
  readonly headBranch: string;
  readonly headSha: string;
  readonly status: GitHubActionsStatus;
  readonly conclusion: GitHubActionsConclusion;
}

export interface CiJobEvidence {
  readonly id: string;
  readonly name: string;
  readonly runId: string;
  readonly runAttempt: string;
  readonly status: GitHubActionsStatus;
  readonly conclusion: GitHubActionsConclusion;
  readonly checkRunUrl: string;
  readonly htmlUrl: string;
}

export interface CiCheckRunEvidence {
  readonly id: string;
  readonly name: string;
  readonly runId: string;
  readonly runAttempt: string;
  readonly checkSuiteId: string;
  readonly appSlug: string;
  readonly headSha: string;
  readonly status: GitHubActionsStatus;
  readonly conclusion: GitHubActionsConclusion;
  readonly url: string;
  readonly detailsUrl: string;
}

export interface CiAttemptEvidence {
  readonly runId: string;
  readonly runAttempt: string;
  readonly checkSuiteId: string;
  readonly jobs: readonly CiJobEvidence[];
  readonly checkRuns: readonly CiCheckRunEvidence[];
}

export interface DevReleaseEligibilityInput {
  readonly schema: "fyagent-dev-release-eligibility-input/v1";
  readonly repository: RepositoryIdentity;
  readonly event: ReleaseEventIdentity;
  readonly workflow: ReleaseWorkflowIdentity;
  readonly candidate: ReleaseCandidateIdentity;
  readonly remoteDev: RemoteDevIdentity;
  readonly remoteTag: AnnotatedRemoteTagIdentity | null;
  readonly ciWorkflow: CiWorkflowIdentity;
  readonly ciRuns: readonly CiRunSummary[];
  readonly ciEvidence: CiAttemptEvidence;
}

export interface DevReleaseEligibilityOutput {
  readonly appVersion: string;
  readonly releaseTag: string;
  readonly sourceSha: string;
  readonly workflowSha: string;
  readonly ciRunId: string;
  readonly ciRunAttempt: string;
  readonly mode: DevReleaseMode;
}

export const DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA: "fyagent-dev-release-eligibility-input/v1";
export const EXPECTED_REPOSITORY: "fy-agent/fyagent";
export const EXPECTED_REPOSITORY_ID: "1313497021";
export const RELEASE_WORKFLOW_NAME: "Release";
export const RELEASE_WORKFLOW_PATH: ".github/workflows/release.yml";
export const CI_WORKFLOW_NAME: "CI";
export const CI_WORKFLOW_PATH: ".github/workflows/ci.yml";
export const DEV_BRANCH: "dev/laiyongjie";
export const DEV_REF: "refs/heads/dev/laiyongjie";
export const FORMAL_BRANCH: "main";
export const FORMAL_REF: "refs/heads/main";
export const REQUIRED_JOB_NAME: "CI / Required";

export function evaluateDevReleaseEligibility(
  input: DevReleaseEligibilityInput,
  expectedFrozen?: DevReleaseEligibilityOutput,
): Readonly<DevReleaseEligibilityOutput>;
