import {
  SECRET_USER_ACTION_LABELS_ZH,
  credentialBrowserFixtures,
  type CredentialsSnapshot,
  type SecretCandidateSummary,
  type SecretOwnerCredentialSummary,
  type SecretRefAggregate,
  type SecretUserAction,
} from "@/v2/shared/data/credentials";

export interface CredentialListRow {
  ownerId: string;
  displayName: string;
  summary: SecretOwnerCredentialSummary;
  aggregate?: SecretRefAggregate;
  candidate?: SecretCandidateSummary;
  nextAction: SecretUserAction;
}

export function nextActionFor(
  summary: SecretOwnerCredentialSummary,
  aggregate?: SecretRefAggregate,
  candidate?: SecretCandidateSummary,
): SecretUserAction {
  if (candidate?.state === "verifiedPendingPlan") {
    return candidate.pendingTerminalDisposition
      ? "discardCandidate"
      : "reopenChangePlan";
  }
  if (candidate?.state === "expired") {
    return "refreshSummary";
  }
  if (candidate?.state === "cleanupRequired") {
    return "completeRecovery";
  }
  if (summary.bindingState.state === "legacy") {
    return summary.bindingState.action;
  }
  if (summary.bindingState.state === "unbound") {
    return "chooseBackend";
  }
  if (aggregate?.issue?.action) {
    return aggregate.issue.action;
  }
  return "none";
}

export function buildCredentialRows(
  snapshot: CredentialsSnapshot = credentialBrowserFixtures,
): CredentialListRow[] {
  return snapshot.owners.map((summary) => {
    const ownerId = summary.owner.ownerId;
    const aggregate =
      summary.bindingState.state === "bound"
        ? snapshot.refs.find(
            (item) => item.secretRef === summary.bindingState.secretRef,
          )
        : undefined;
    const candidate = snapshot.candidates.find((item) =>
      item.targetOwners.some((owner) => owner.ownerId === ownerId),
    );
    return {
      ownerId,
      displayName: snapshot.ownerDisplayNames[ownerId] ?? ownerId,
      summary,
      aggregate,
      candidate,
      nextAction: nextActionFor(summary, aggregate, candidate),
    };
  });
}

export function emptyCredentialSnapshot(): CredentialsSnapshot {
  return {
    ...credentialBrowserFixtures,
    owners: [],
    refs: [],
    candidates: [],
  };
}

export const credentialPrototypeRows = buildCredentialRows();

export function nextActionLabel(action: SecretUserAction): string {
  return SECRET_USER_ACTION_LABELS_ZH[action];
}
