import { useState } from "react";

import type { ProductDirectoryEntry } from "../../shared/features/directory";
import {
  useOpenCodeModelSnapshot,
  useProviderSummary,
  useTraeWorkModelIds,
  useWorkBuddyModelIds,
  useWorkBuddyStatus,
} from "../../shared/features/queries";
import type {
  AgentCatalogEntry,
  ProviderSummaryQueryData,
} from "../../shared/features/types";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { EmptyState, InlineNotice, Spinner } from "../../shared/ui/primitives";

import { AgentSectionHeader } from "./AgentSectionHeader";

type ModelObservation = {
  id: string;
  label: string;
  detail: string;
};

function providerObservations(
  data: ProviderSummaryQueryData | undefined,
): ModelObservation[] {
  if (!data) return [];
  return Object.values(data.providers)
    .sort((left, right) => {
      if (left.id === data.currentId) return -1;
      if (right.id === data.currentId) return 1;
      return left.name.localeCompare(right.name);
    })
    .map((provider) => ({
      id: provider.id,
      label: provider.modelId ?? provider.name,
      detail:
        provider.id === data.currentId
          ? `当前 Provider · ${provider.name}`
          : `已配置 Provider · ${provider.name}`,
    }));
}

export function AgentModelsSection({
  entry,
  catalogEntry,
  onOpenManagement,
}: {
  entry: ProductDirectoryEntry;
  catalogEntry: AgentCatalogEntry;
  onOpenManagement: () => void;
}) {
  const [search, setSearch] = useState("");
  const workBuddyStatus = useWorkBuddyStatus(entry.agentId === "workbuddy");
  const workBuddyModels = useWorkBuddyModelIds(entry.agentId === "workbuddy");
  const traeModels = useTraeWorkModelIds(entry.agentId === "trae-work");
  const openCodeModels = useOpenCodeModelSnapshot(entry.agentId === "opencode");
  const grokSummary = useProviderSummary(
    "grokbuild",
    entry.agentId === "grokbuild",
  );
  const codexSummary = useProviderSummary("codex", entry.agentId === "codex");
  const claudeSummary = useProviderSummary(
    "claude",
    entry.agentId === "claude-code",
  );
  const modelCapability = catalogEntry.capabilities.find(
    (candidate) => candidate.id === "models.write",
  );
  const mode = modelCapability?.mode ?? "unverified";
  let observations: ModelObservation[] = [];
  let pending = false;
  let failed = false;

  switch (entry.agentId) {
    case "qoderwork":
      break;
    case "trae-work":
      observations = (traeModels.data?.modelIds ?? []).map((modelId) => ({
        id: modelId,
        label: modelId,
        detail: "TRAE Work CN 已观测模型 · 供应商界面负责写入",
      }));
      pending = traeModels.isPending;
      failed = traeModels.isError;
      break;
    case "workbuddy":
      observations = (workBuddyModels.data?.ids ?? []).map((modelId) => ({
        id: modelId,
        label: modelId,
        detail: "WorkBuddy 已配置模型",
      }));
      pending = workBuddyStatus.isPending || workBuddyModels.isPending;
      failed = workBuddyStatus.isError || workBuddyModels.isError;
      break;
    case "grokbuild":
      observations = providerObservations(grokSummary.data);
      pending = grokSummary.isPending;
      failed = grokSummary.isError;
      break;
    case "codex":
      observations = providerObservations(codexSummary.data);
      pending = codexSummary.isPending;
      failed = codexSummary.isError;
      break;
    case "claude-code":
      observations = providerObservations(claudeSummary.data);
      pending = claudeSummary.isPending;
      failed = claudeSummary.isError;
      break;
    case "opencode":
      observations = (openCodeModels.data?.providers ?? []).flatMap(
        (provider) =>
          provider.modelIds.length > 0
            ? provider.modelIds.map((modelId) => ({
                id: `${provider.id}:${modelId}`,
                label: modelId,
                detail: `OpenCode Provider · ${provider.name}`,
              }))
            : [
                {
                  id: `provider:${provider.id}`,
                  label: provider.name,
                  detail: "OpenCode Provider 已配置，尚未观察到模型 ID",
                },
              ],
      );
      pending = openCodeModels.isPending;
      failed = openCodeModels.isError;
      break;
  }

  const normalizedSearch = search.trim().toLocaleLowerCase();
  const filtered = observations.filter((item) =>
    `${item.label} ${item.detail} ${item.id}`
      .toLocaleLowerCase()
      .includes(normalizedSearch),
  );

  return (
    <section className="fy-agent-config-section" aria-label="Agent 模型配置">
      <AgentSectionHeader
        title="当前模型"
        actionLabel="进入模型管理"
        onAction={onOpenManagement}
      />
      {mode !== "unsupported" ? (
        <FeatureSearch
          value={search}
          onValueChange={setSearch}
          placeholder="搜索模型或 Provider"
          ariaLabel="搜索 Agent 模型"
          disabled={pending}
        />
      ) : null}
      {pending ? (
        <div className="fy-agent-config-loading">
          <Spinner label="正在读取模型状态" />
          <span>正在读取模型状态</span>
        </div>
      ) : failed ? (
        <InlineNotice tone="warning">
          当前模型状态无法读取，请检查网络或配置后重试。
        </InlineNotice>
      ) : mode === "unsupported" ? (
        <EmptyState title="当前官方能力不支持第三方模型配置" />
      ) : observations.length === 0 ? (
        <EmptyState title="尚未观察到模型" />
      ) : filtered.length === 0 ? (
        <EmptyState title="没有匹配的模型" description="请调整搜索关键词。" />
      ) : (
        <div className="fy-agent-models-list">
          {filtered.map((item) => (
            <div key={item.id} className="fy-agent-model-card">
              <div className="fy-agent-model-card-info">
                <h3>{item.label}</h3>
                <p>{item.detail}</p>
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
