import { useState } from "react";

import type { ProductDirectoryEntry } from "../../shared/features/directory";
import { convergeSelection } from "../../shared/features/helpers";
import {
  useOpenCodeModelSnapshot,
  useProviderSummary,
  useTraeWorkModelIds,
  useWorkBuddyModelIds,
  useWorkBuddyStatus,
} from "../../shared/features/queries";
import type {
  AgentCapabilityMode,
  AgentCatalogEntry,
  ProviderSummaryQueryData,
} from "../../shared/features/types";
import { FeatureList, FeatureListItem } from "../../shared/ui/FeatureList";
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

function capabilityCopy(mode: AgentCapabilityMode): string {
  switch (mode) {
    case "direct":
      return "此 Agent 已有原生模型 owner。本页只投影已观测配置，写入继续由模型管理中的既有路径负责。";
    case "assisted":
      return "此 Agent 的模型配置需要在供应商界面完成；本页仅展示可安全读取的观察结果。";
    case "unsupported":
      return "当前官方能力不支持第三方模型配置；本页不会提供可写开关或伪造保存成功。";
    case "unverified":
      return "模型配置能力尚未验证；本页不推断可写，也不显示本地成功状态。";
  }
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
  const [selectedId, setSelectedId] = useState<string | null>(null);
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
        detail: "TRAE Work CN 缓存观察 · 供应商界面负责写入",
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
  const convergedId = convergeSelection(filtered, selectedId);
  const selected = filtered.find((item) => item.id === convergedId) ?? null;

  return (
    <section className="fy-agent-config-section" aria-label="Agent 模型配置">
      <AgentSectionHeader
        title="当前模型"
        description="按 Agent 真实 capability 投影已观测或已配置模型，不创建第二套模型分配状态。"
        actionLabel="进入模型管理"
        onAction={onOpenManagement}
      />
      <InlineNotice tone={mode === "unsupported" ? "warning" : "info"}>
        {capabilityCopy(mode)}
      </InlineNotice>
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
          当前模型状态无法读取；此页不会把未知状态写成“未配置”。
        </InlineNotice>
      ) : mode === "unsupported" ? null : observations.length === 0 ? (
        <EmptyState
          title="尚未观察到模型"
          description="当前读取结果为空；这不等于已证明供应商侧没有配置。"
        />
      ) : filtered.length === 0 ? (
        <EmptyState title="没有匹配的模型" description="请调整搜索关键词。" />
      ) : (
        <div className="fy-agent-resource-workspace">
          <div className="fy-feature-panel fy-agent-resource-list-panel">
            <FeatureList id="agent-model-list" aria-label="模型观察列表">
              {filtered.map((item) => (
                <FeatureListItem
                  key={item.id}
                  selected={item.id === selected?.id}
                  onSelect={() => setSelectedId(item.id)}
                  title={item.label}
                >
                  <span>{item.detail}</span>
                </FeatureListItem>
              ))}
            </FeatureList>
          </div>
          {selected ? (
            <div className="fy-feature-panel fy-agent-resource-detail">
              <div>
                <h3>{selected.label}</h3>
                <p>{selected.detail}</p>
                <span className="fy-agent-resource-meta">能力模式：{mode}</span>
              </div>
              <InlineNotice>
                此处没有模型开关；请进入模型管理使用该 Agent 已有的原生 owner。
              </InlineNotice>
            </div>
          ) : null}
        </div>
      )}
    </section>
  );
}
