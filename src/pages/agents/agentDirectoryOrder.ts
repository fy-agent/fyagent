import {
  AGENT_CATALOG_IDS,
  AGENT_DISPLAY_ORDER,
  agentDirectoryPriority,
  isOfficialOnlyAgent,
  type AgentCatalogId,
} from "../../shared/features/directory";

import {
  isAgentExistenceProven,
  type AgentDirectoryScanView,
} from "./agentDirectoryScanProjection";

export type AgentDirectoryOrderBucket =
  | "installed_domestic"
  | "installed_other"
  | "unresolved"
  | "not_installed"
  | "official_only_installed"
  | "official_only_other";

const BUCKET_RANK: Record<AgentDirectoryOrderBucket, number> = {
  installed_domestic: 0,
  installed_other: 1,
  unresolved: 2,
  not_installed: 3,
  official_only_installed: 4,
  official_only_other: 5,
};

function canonicalIndex(agentId: AgentCatalogId): number {
  const index = AGENT_DISPLAY_ORDER.indexOf(agentId);
  return index === -1 ? AGENT_DISPLAY_ORDER.length : index;
}

export function classifyAgentDirectoryOrderBucket(
  agentId: AgentCatalogId,
  scan: AgentDirectoryScanView,
): AgentDirectoryOrderBucket {
  if (isOfficialOnlyAgent(agentId)) {
    if (scan.currentFailureIds.includes(agentId)) {
      return "official_only_other";
    }
    const installState = scan.results[agentId]?.installState;
    return isAgentExistenceProven(installState)
      ? "official_only_installed"
      : "official_only_other";
  }

  if (scan.currentFailureIds.includes(agentId)) {
    return "unresolved";
  }

  const installState = scan.results[agentId]?.installState;
  if (isAgentExistenceProven(installState)) {
    return agentDirectoryPriority(agentId) === "domestic"
      ? "installed_domestic"
      : "installed_other";
  }
  if (installState === "not_installed") {
    return "not_installed";
  }
  return "unresolved";
}

export function orderAgentDirectoryEntries<T extends { id: AgentCatalogId }>(
  entries: readonly T[],
  scan: AgentDirectoryScanView,
): T[] {
  return [...entries].sort((left, right) => {
    const bucketDelta =
      BUCKET_RANK[classifyAgentDirectoryOrderBucket(left.id, scan)] -
      BUCKET_RANK[classifyAgentDirectoryOrderBucket(right.id, scan)];
    if (bucketDelta !== 0) return bucketDelta;
    return canonicalIndex(left.id) - canonicalIndex(right.id);
  });
}

export function committedAgentDirectoryOrderIds(
  scan: AgentDirectoryScanView,
): AgentCatalogId[] {
  return orderAgentDirectoryEntries(
    AGENT_CATALOG_IDS.map((id) => ({ id })),
    scan,
  ).map((entry) => entry.id);
}

export function nextCommittedAgentDirectoryOrderIds(
  scan: AgentDirectoryScanView,
  previousCommittedIds: readonly AgentCatalogId[] | null | undefined,
): AgentCatalogId[] | null {
  if (scan.status !== "complete") {
    return previousCommittedIds ? [...previousCommittedIds] : null;
  }
  return committedAgentDirectoryOrderIds(scan);
}

export function applyCommittedAgentDirectoryOrder<
  T extends { id: AgentCatalogId },
>(
  entries: readonly T[],
  committedOrderIds: readonly AgentCatalogId[] | null | undefined,
): T[] {
  if (!committedOrderIds || committedOrderIds.length === 0) {
    return [...entries].sort(
      (left, right) => canonicalIndex(left.id) - canonicalIndex(right.id),
    );
  }

  const remaining = new Map(entries.map((entry) => [entry.id, entry]));
  const ordered: T[] = [];
  for (const id of committedOrderIds) {
    const entry = remaining.get(id);
    if (!entry) continue;
    ordered.push(entry);
    remaining.delete(id);
  }
  const rest = [...remaining.values()].sort(
    (left, right) => canonicalIndex(left.id) - canonicalIndex(right.id),
  );
  ordered.push(...rest);
  return ordered;
}
