import { useState } from "react";

import type { ProductDirectoryEntry } from "../../shared/features/directory";
import { convergeSelection } from "../../shared/features/helpers";
import { usePrompts } from "../../shared/features/queries";
import { useFeatures } from "../../shared/features/provider";
import type { PromptAppId } from "../../shared/features/types";
import { FeatureList, FeatureListItem } from "../../shared/ui/FeatureList";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import {
  Button,
  EmptyState,
  InlineNotice,
  Spinner,
} from "../../shared/ui/primitives";

import { AgentSectionHeader } from "./AgentSectionHeader";

function SupportedPromptProjection({
  appId,
  displayName,
}: {
  appId: PromptAppId;
  displayName: string;
}) {
  const { ports } = useFeatures();
  const query = usePrompts(appId);
  const [search, setSearch] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<{
    promptId: string;
    tone: "info" | "warning";
    text: string;
  } | null>(null);
  const prompts = query.data ?? [];
  const normalizedSearch = search.trim().toLocaleLowerCase();
  const filtered = prompts.filter((prompt) =>
    `${prompt.name} ${prompt.description ?? ""} ${prompt.content} ${prompt.id}`
      .toLocaleLowerCase()
      .includes(normalizedSearch),
  );
  const convergedId = convergeSelection(filtered, selectedId);
  const selected = filtered.find((prompt) => prompt.id === convergedId) ?? null;

  const enable = async (promptId: string) => {
    if (pendingId) return;
    setPendingId(promptId);
    setFeedback(null);
    try {
      await ports.prompts.enable(appId, promptId);
      const readback = await query.refetch();
      const authoritative = readback.data?.find(
        (prompt) => prompt.id === promptId,
      );
      if (readback.error || !authoritative?.enabled) {
        throw new Error("prompt readback mismatch");
      }
      setFeedback({
        promptId,
        tone: "info",
        text: `已从真实配置回读：${displayName} 当前使用此提示词。`,
      });
    } catch {
      setFeedback({
        promptId,
        tone: "warning",
        text: "提示词启用未能完成或回读不一致；当前页面不会保留乐观成功状态。",
      });
      await query.refetch();
    } finally {
      setPendingId(null);
    }
  };

  return (
    <>
      <FeatureSearch
        value={search}
        onValueChange={setSearch}
        placeholder="搜索名称、描述、内容或 ID"
        ariaLabel="搜索 Agent 提示词"
        disabled={query.isPending}
      />
      {feedback ? (
        <InlineNotice tone={feedback.tone}>{feedback.text}</InlineNotice>
      ) : null}
      {query.isError && query.data !== undefined ? (
        <InlineNotice tone="warning">
          暂时无法刷新提示词，正在显示已加载结果。
        </InlineNotice>
      ) : null}
      {query.isPending ? (
        <div className="fy-agent-config-loading">
          <Spinner label="正在读取提示词" />
          <span>正在读取 {displayName} 提示词库</span>
        </div>
      ) : query.isError && query.data === undefined ? (
        <EmptyState
          title="无法读取提示词"
          description="当前没有可验证的提示词结果。"
          actions={<Button onClick={() => void query.refetch()}>重试</Button>}
        />
      ) : prompts.length === 0 ? (
        <EmptyState
          title="提示词库为空"
          description={`当前 ${displayName} 没有已管理提示词。`}
        />
      ) : filtered.length === 0 ? (
        <EmptyState title="没有匹配的提示词" description="请调整搜索关键词。" />
      ) : (
        <div className="fy-agent-resource-workspace">
          <div className="fy-feature-panel fy-agent-resource-list-panel">
            <FeatureList id="agent-prompts-list" aria-label="提示词库">
              {filtered.map((prompt) => (
                <FeatureListItem
                  key={prompt.id}
                  selected={prompt.id === selected?.id}
                  onSelect={() => setSelectedId(prompt.id)}
                  title={prompt.name}
                >
                  <span>
                    {prompt.enabled ? "当前启用" : "未启用"} ·{" "}
                    {prompt.description ?? prompt.id}
                  </span>
                </FeatureListItem>
              ))}
            </FeatureList>
          </div>
          {selected ? (
            <div className="fy-feature-panel fy-agent-resource-detail">
              <div className="fy-agent-prompt-detail-heading">
                <div>
                  <div className="fy-feature-detail-title">
                    <h3>{selected.name}</h3>
                  </div>
                  <p>{selected.description ?? "此提示词暂无补充说明。"}</p>
                </div>
                <Button
                  className={
                    selected.enabled ? undefined : "fy-control-button-primary"
                  }
                  disabled={selected.enabled || pendingId === selected.id}
                  onClick={() => void enable(selected.id)}
                >
                  {selected.enabled ? "当前已启用" : "设为启用"}
                </Button>
              </div>
              <pre className="fy-agent-prompt-preview">{selected.content}</pre>
            </div>
          ) : null}
        </div>
      )}
    </>
  );
}

export function AgentPromptsSection({
  entry,
  onOpenManagement,
}: {
  entry: ProductDirectoryEntry;
  onOpenManagement: () => void;
}) {
  return (
    <section className="fy-agent-config-section" aria-label="Agent 提示词配置">
      <AgentSectionHeader
        title="当前提示词"
        actionLabel="进入提示词管理"
        onAction={onOpenManagement}
      />
      {entry.promptAppId ? (
        <SupportedPromptProjection
          appId={entry.promptAppId}
          displayName={entry.displayName}
        />
      ) : (
        <EmptyState title="当前未接入提示词管理" />
      )}
    </section>
  );
}
