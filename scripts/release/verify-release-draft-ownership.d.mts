export interface RecoverableReleaseDraftIdentity {
  readonly releaseId: string;
  readonly runId: string;
  readonly runAttempt: string;
  readonly sourceSha: string;
  readonly createdAt: string;
}

export function inspectRecoverableReleaseDraft(
  releaseValue: unknown,
  expectedTag: string,
  expectedSourceSha: string,
): Readonly<RecoverableReleaseDraftIdentity>;

export function verifyRecoverableReleaseDraft(
  releaseValue: unknown,
  runAttemptValue: unknown,
  jobsValue: unknown,
  expectedTag: string,
  expectedSourceSha: string,
): Readonly<RecoverableReleaseDraftIdentity>;
