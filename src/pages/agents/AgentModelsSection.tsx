import { useState } from "react";
import { useNavigate } from "react-router-dom";

import { appendAgentReturnToPath } from "../../shared/features/agent-navigation";
import { CopyablePath } from "../../shared/features/controls/CopyablePath";
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
  ProviderAppId,
  ProviderSummaryQueryData,
} from "../../shared/features/types";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { EmptyState, InlineNotice, Spinner } from "../../shared/ui/primitives";
import { Button } from "../../shared/ui/Button";

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
          ? `已选方案 · ${provider.name}`
          : `已保存方案 · ${provider.name}`,
    }));
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
            <p>尚未测试连接。</p>
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
  const navigate = useNavigate();
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
                  detail: "Provider 已连接，但未返回模型 ID",
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
    <section
      className="fy-agent-config-section"
      aria-label={`${entry.displayName} 模型设置`}
    >
      <AgentSectionHeader
        title={providerApp ? "模型配置" : "当前模型"}
        actionLabel="管理模型"
        onAction={onOpenManagement}
      />
      {entry.agentId === "grokbuild" ? (
        <>
          <InlineNotice tone="info">
            要让 Claude Code 或 Codex 使用 SuperGrok，请在账号与认证保存 Grok
            账号，再到目标软件的模型管理选择账号和模型。
          </InlineNotice>
          <div className="fy-agent-action-row">
            {(["claude", "codex"] as const).map((target) => (
              <Button
                key={target}
                onClick={() =>
                  navigate(
                    appendAgentReturnToPath(`/models?target=${target}`, {
                      agentId: entry.agentId,
                      section: "models",
                    }),
                  )
                }
              >
                {target === "claude"
                  ? "为 Claude Code 设置订阅"
                  : "为 Codex 设置订阅"}
              </Button>
            ))}
          </div>
        </>
      ) : null}
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
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
