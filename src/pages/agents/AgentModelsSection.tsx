import { useState } from "react";

import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import { InlineModelProbeButton } from "../../shared/features/controls/InlineModelProbeButton";
import type { ProductDirectoryEntry } from "../../shared/features/directory";
import {
  useOmpConfig,
  useOpenCodeModelSnapshot,
  useProviderSummary,
  useTraeWorkModelIds,
  useWorkBuddyModelIds,
  useWorkBuddyStatus,
} from "../../shared/features/queries";
import type {
  AgentCatalogEntry,
  BatchSetupTargetId,
  ProviderAppId,
  ProviderSummaryQueryData,
} from "../../shared/features/types";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { EmptyState, InlineNotice, Spinner } from "../../shared/ui/primitives";

import { AgentSectionHeader } from "./AgentSectionHeader";

type ModelObservation = {
  id: string;
  label: string;
  detail: string;
  probe?: {
    target: BatchSetupTargetId;
    providerId?: string | null;
    modelId: string;
  };
};

function providerObservations(
  target: "codex" | "claude" | "grokbuild",
  data: ProviderSummaryQueryData | undefined,
): ModelObservation[] {
  if (!data) return [];
  return Object.values(data.providers)
    .sort((left, right) => {
      if (left.id === data.currentId) return -1;
      if (right.id === data.currentId) return 1;
      return left.name.localeCompare(right.name);
    })
    .map((provider) => {
      const resolvedModelId = provider.modelId ?? provider.connection?.modelId;
      return {
        id: provider.id,
        label: resolvedModelId ?? provider.name,
        detail:
          provider.id === data.currentId
            ? `已选方案 · ${provider.name}`
            : `已保存方案 · ${provider.name}`,
        ...(resolvedModelId
          ? {
              probe: {
                target,
                providerId: provider.id,
                modelId: resolvedModelId,
              },
            }
          : {}),
      };
    });
}

function ProviderLiveConfiguration({
  app,
  data,
  failed,
}: {
  app: ProviderAppId;
  data: ProviderSummaryQueryData | undefined;
  failed: boolean;
}) {
  const live = data?.live;
  const matching = live?.target === app;
  const unknown = failed || !matching || live?.state === "unreadable";
  const configured = !unknown && live?.state === "configured";
  return (
    <section className="fy-agent-model-card" aria-label="实际配置文件">
      <div className="fy-agent-model-card-info">
        <h3>实际配置文件</h3>
        {unknown ? (
          <InlineNotice tone="warning">
            实际配置状态未知。无法读取当前配置文件，请在软件中检查或稍后重试。
          </InlineNotice>
        ) : configured ? (
          <>
            <p>实际配置文件已配置</p>
            <p>模型：{live.connection.modelId ?? "未指定模型"}</p>
            <p>服务地址：{live.connection.baseUrl ?? "未指定服务地址"}</p>
          </>
        ) : (
          <p>
            实际配置尚未配置。
            {live?.state === "missing" ? "配置文件尚不存在。" : ""}
          </p>
        )}
        {(!live || matching) && (data?.writeTargets.length ?? 0) > 0 ? (
          <div>
            <p>配置文件位置</p>
            {data!.writeTargets.map(({ path }) => (
              <CopyablePath key={path} value={path} label="配置文件路径" />
            ))}
          </div>
        ) : null}
      </div>
    </section>
  );
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
  const ompConfig = useOmpConfig(entry.agentId === "omp");
  const grokSummary = useProviderSummary(
    "grokbuild",
    entry.agentId === "grokbuild",
  );
  const codexSummary = useProviderSummary("codex", entry.agentId === "codex");
  const claudeSummary = useProviderSummary(
    "claude",
    entry.agentId === "claude-code",
  );
  const providerApp =
    entry.agentId === "claude-code"
      ? "claude"
      : entry.agentId === "codex" || entry.agentId === "grokbuild"
        ? entry.agentId
        : null;
  const providerQuery =
    providerApp === "claude"
      ? claudeSummary
      : providerApp === "codex"
        ? codexSummary
        : providerApp === "grokbuild"
          ? grokSummary
          : null;
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
        detail: "已在 TRAE Work CN 中配置；请在 TRAE Work CN 中修改",
      }));
      pending = traeModels.isPending;
      failed = traeModels.isError;
      break;
    case "workbuddy":
      observations = (workBuddyModels.data?.ids ?? []).map((modelId) => ({
        id: modelId,
        label: modelId,
        detail: "WorkBuddy 已配置模型",
        probe: { target: "workbuddy", modelId },
      }));
      pending = workBuddyStatus.isPending || workBuddyModels.isPending;
      failed = workBuddyStatus.isError || workBuddyModels.isError;
      break;
    case "grokbuild":
      observations = providerObservations("grokbuild", grokSummary.data);
      pending = grokSummary.isPending;
      failed = grokSummary.isError;
      break;
    case "codex":
      observations = providerObservations("codex", codexSummary.data);
      pending = codexSummary.isPending;
      failed = codexSummary.isError;
      break;
    case "claude-code":
      observations = providerObservations("claude", claudeSummary.data);
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
                probe: {
                  target: "opencode" as const,
                  providerId: provider.id,
                  modelId,
                },
              }))
            : [
                {
                  id: `provider:${provider.id}`,
                  label: provider.name,
                  detail: "Provider 已连接，但未返回模型 ID",
                },
              ],
      );
      pending = openCodeModels.isPending;
      failed = openCodeModels.isError;
      break;
    case "omp":
      observations = (ompConfig.data?.providers ?? []).flatMap((provider) =>
        provider.models.length > 0
          ? provider.models.map((model) => {
              const selector = `${provider.id}/${model.id}`;
              const isDefault = ompConfig.data?.defaultModel === selector;
              return {
                id: selector,
                label: model.name ? `${model.name} (${model.id})` : model.id,
                detail: `${isDefault ? "默认模型 · " : ""}omp Provider · ${provider.id}`,
                probe: {
                  target: "omp" as const,
                  providerId: provider.id,
                  modelId: model.id,
                },
              };
            })
          : [
              {
                id: `provider:${provider.id}`,
                label: provider.id,
                detail: "omp Provider 已创建，尚未配置模型 ID",
              },
            ],
      );
      pending = ompConfig.isPending;
      failed = ompConfig.isError;
      break;
  }

  const normalizedSearch = search.trim().toLocaleLowerCase();
  const filtered = observations.filter((item) =>
    `${item.label} ${item.detail} ${item.id}`
      .toLocaleLowerCase()
      .includes(normalizedSearch),
  );

  return (
    <section
      className="fy-agent-config-section"
      aria-label={`${entry.displayName} 模型设置`}
    >
      <AgentSectionHeader
        title={providerApp ? "模型配置" : "当前模型"}
        actionLabel="管理模型"
        onAction={onOpenManagement}
      />
      {providerApp && !pending ? (
        <>
          <ProviderLiveConfiguration
            app={providerApp}
            data={providerQuery?.data}
            failed={failed}
          />
          <h3>FyAgent 已保存方案</h3>
          <p>已选方案记录在 FyAgent 中，与实际配置文件分别显示。</p>
        </>
      ) : null}
      {mode !== "unsupported" ? (
        <FeatureSearch
          value={search}
          onValueChange={setSearch}
          placeholder="搜索模型或 Provider"
          ariaLabel={`搜索 ${entry.displayName} 的模型`}
          disabled={pending}
        />
      ) : null}
      {pending ? (
        <div className="fy-agent-config-loading">
          <Spinner label="正在读取模型状态" />
          <span>正在读取模型状态</span>
        </div>
      ) : failed && (!providerApp || !providerQuery?.data) ? (
        <InlineNotice tone="warning">
          {providerApp
            ? "FyAgent 已保存方案暂时无法读取，请稍后重试。"
            : "当前模型状态无法读取，请检查配置后重试。"}
        </InlineNotice>
      ) : mode === "unsupported" ? (
        <EmptyState title="此应用不支持在 FyAgent 中配置第三方模型" />
      ) : observations.length === 0 ? (
        <EmptyState
          title={
            providerApp ? "FyAgent 还没有保存方案" : "还没有找到已配置的模型"
          }
        />
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
              {item.probe ? (
                <InlineModelProbeButton
                  target={item.probe.target}
                  providerId={item.probe.providerId}
                  modelId={item.probe.modelId}
                />
              ) : null}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
