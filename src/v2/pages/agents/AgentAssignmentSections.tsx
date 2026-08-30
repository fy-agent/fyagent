import { useState } from "react";

import type { ProductDirectoryEntry } from "../../shared/features/directory";
import {
  buildMcpSearchText,
  buildSkillSearchText,
} from "../../shared/features/helpers";
import { useAuthoritativeAssignmentMutation } from "../../shared/features/authoritative-assignment";
import {
  useInstalledSkills,
  useMcpServers,
} from "../../shared/features/queries";
import { useFeatures } from "../../shared/features/provider";
import type { McpServer } from "../../shared/features/types";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { WorkBuddyTrustDialog } from "../../shared/ui/WorkBuddyTrustDialog";
import {
  Button,
  EmptyState,
  InlineNotice,
  Spinner,
  Switch,
} from "../../shared/ui/primitives";

import { AgentSectionHeader } from "./AgentSectionHeader";

type Feedback = {
  itemId: string;
  tone: "info" | "warning";
  text: string;
};

export function AgentSkillsSection({
  entry,
  onOpenManagement,
}: {
  entry: ProductDirectoryEntry;
  onOpenManagement: () => void;
}) {
  const { ports } = useFeatures();
  const query = useInstalledSkills();
  const [search, setSearch] = useState("");
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const skills = query.data ?? [];
  const normalizedSearch = search.trim().toLocaleLowerCase();
  const filtered = skills.filter((skill) =>
    buildSkillSearchText(skill).toLocaleLowerCase().includes(normalizedSearch),
  );

  const assignment = useAuthoritativeAssignmentMutation({
    mutate: (skillId: string, enabled: boolean) =>
      ports.skills.toggleApp(skillId, entry.assignmentId, enabled),
    reread: async () => {
      const readback = await query.refetch();
      return { data: readback.data, error: readback.error };
    },
    readValue: (snapshot, skillId: string) =>
      Boolean(
        snapshot?.find((skill) => skill.id === skillId)?.apps[
          entry.assignmentId
        ],
      ),
  });

  const toggle = async (skillId: string, enabled: boolean) => {
    setFeedback(null);
    const outcome = await assignment.run(skillId, enabled);
    if (outcome.status === "confirmed") {
      setFeedback({
        itemId: skillId,
        tone: "info",
        text: enabled
          ? `已在 ${entry.displayName} 中启用此 Skill。`
          : `已在 ${entry.displayName} 中停用此 Skill。`,
      });
    } else if (outcome.status === "rejected") {
      setFeedback({
        itemId: skillId,
        tone: "warning",
        text: "无法确认 Skill 设置是否已更新。请刷新后重试。",
      });
    }
  };

  return (
    <section
      className="fy-agent-config-section"
      aria-label={`${entry.displayName} Skills 设置`}
    >
      <AgentSectionHeader
        title="当前 Skills"
        actionLabel="管理 Skills"
        onAction={onOpenManagement}
      />
      <FeatureSearch
        value={search}
        onValueChange={setSearch}
        placeholder="搜索名称、说明或仓库"
        ariaLabel={`搜索 ${entry.displayName} 的 Skills`}
        disabled={query.isPending}
      />
      {feedback ? (
        <InlineNotice tone={feedback.tone}>{feedback.text}</InlineNotice>
      ) : null}
      {assignment.busy ? (
        <InlineNotice tone="info">正在保存 Skill 设置…</InlineNotice>
      ) : null}
      {query.isError && query.data !== undefined ? (
        <InlineNotice tone="warning">
          暂时无法刷新 Skills，正在显示已加载结果。
        </InlineNotice>
      ) : null}
      {query.isPending ? (
        <div className="fy-agent-config-loading">
          <Spinner label="正在读取 Skills" />
          <span>正在读取已安装 Skills</span>
        </div>
      ) : query.isError && query.data === undefined ? (
        <EmptyState
          title="无法读取 Skills"
          description="暂时无法读取已安装的 Skills。请重试。"
          actions={<Button onClick={() => void query.refetch()}>重试</Button>}
        />
      ) : skills.length === 0 ? (
        <EmptyState
          title="还没有可用的 Skill"
          description="请先到 Skills 页面安装或导入 Skill。"
          actions={<Button onClick={onOpenManagement}>管理 Skills</Button>}
        />
      ) : filtered.length === 0 ? (
        <EmptyState title="没有匹配的 Skill" description="请调整搜索关键词。" />
      ) : (
        <div
          className="fy-agent-resource-full-list"
          aria-busy={assignment.busy}
        >
          {filtered.map((skill) => {
            const isAssigned = Boolean(skill.apps[entry.assignmentId]);
            const isPending = assignment.pendingId === skill.id;
            const sourceText =
              skill.repoOwner && skill.repoName
                ? `${skill.repoOwner}/${skill.repoName}`
                : skill.directory;
            return (
              <article
                key={skill.id}
                className="fy-agent-assignment-card"
                data-pending={isPending || undefined}
              >
                <div className="fy-agent-assignment-card-copy">
                  <div className="fy-agent-assignment-card-title">
                    <h3>{skill.name}</h3>
                    {sourceText ? (
                      <span className="fy-agent-assignment-source-badge">
                        {sourceText}
                      </span>
                    ) : null}
                  </div>
                  <p>{skill.description ?? "暂无说明"}</p>
                </div>
                <div className="fy-agent-assignment-card-action">
                  <Switch
                    checked={isAssigned}
                    disabled={assignment.busy}
                    label={`在 ${entry.displayName} 中使用 ${skill.name}`}
                    onCheckedChange={(enabled) =>
                      void toggle(skill.id, enabled)
                    }
                  />
                </div>
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}

function mcpTransport(server: McpServer): string {
  if (server.server.type) return server.server.type;
  return server.server.url ? "http" : "stdio";
}

export function AgentMcpSection({
  entry,
  onOpenManagement,
}: {
  entry: ProductDirectoryEntry;
  onOpenManagement: () => void;
}) {
  const { ports } = useFeatures();
  const query = useMcpServers();
  const [search, setSearch] = useState("");
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const [workbuddyTrustOpen, setWorkbuddyTrustOpen] = useState(false);
  const servers = Object.values(query.data ?? {});
  const normalizedSearch = search.trim().toLocaleLowerCase();
  const filtered = servers.filter((server) =>
    buildMcpSearchText(server).toLocaleLowerCase().includes(normalizedSearch),
  );

  const assignment = useAuthoritativeAssignmentMutation({
    mutate: (serverId: string, enabled: boolean) =>
      ports.mcp.toggleApp(serverId, entry.assignmentId, enabled),
    reread: async () => {
      const readback = await query.refetch();
      return { data: readback.data, error: readback.error };
    },
    readValue: (snapshot, serverId: string) =>
      Boolean(snapshot?.[serverId]?.apps[entry.assignmentId]),
  });

  const toggle = async (serverId: string, enabled: boolean) => {
    setFeedback(null);
    const outcome = await assignment.run(serverId, enabled);
    if (outcome.status === "confirmed") {
      setFeedback({
        itemId: serverId,
        tone: "info",
        text: enabled
          ? `已在 ${entry.displayName} 中启用此 MCP。`
          : `已在 ${entry.displayName} 中停用此 MCP。`,
      });
      if (entry.assignmentId === "workbuddy" && enabled) {
        setWorkbuddyTrustOpen(true);
      }
    } else if (outcome.status === "rejected") {
      setFeedback({
        itemId: serverId,
        tone: "warning",
        text: "无法确认 MCP 设置是否已更新。请刷新后重试。",
      });
    }
  };

  return (
    <section
      className="fy-agent-config-section"
      aria-label={`${entry.displayName} MCP 设置`}
    >
      <AgentSectionHeader
        title="当前 MCP"
        actionLabel="管理 MCP"
        onAction={onOpenManagement}
      />
      <FeatureSearch
        value={search}
        onValueChange={setSearch}
        placeholder="搜索名称、命令、标签或来源"
        ariaLabel={`搜索 ${entry.displayName} 的 MCP`}
        disabled={query.isPending}
      />
      {feedback ? (
        <InlineNotice tone={feedback.tone}>{feedback.text}</InlineNotice>
      ) : null}
      {assignment.busy ? (
        <InlineNotice tone="info">正在保存 MCP 设置…</InlineNotice>
      ) : null}
      {query.isError && query.data !== undefined ? (
        <InlineNotice tone="warning">
          暂时无法刷新 MCP，正在显示已加载结果。
        </InlineNotice>
      ) : null}
      {query.isPending ? (
        <div className="fy-agent-config-loading">
          <Spinner label="正在读取 MCP" />
          <span>正在读取 MCP 配置</span>
        </div>
      ) : query.isError && query.data === undefined ? (
        <EmptyState
          title="无法读取 MCP"
          description="暂时无法读取 MCP 设置。请重试。"
          actions={<Button onClick={() => void query.refetch()}>重试</Button>}
        />
      ) : servers.length === 0 ? (
        <EmptyState
          title="还没有可用的 MCP"
          description="请先到 MCP 页面导入或添加服务器。"
          actions={<Button onClick={onOpenManagement}>管理 MCP</Button>}
        />
      ) : filtered.length === 0 ? (
        <EmptyState title="没有匹配的 MCP" description="请调整搜索关键词。" />
      ) : (
        <div
          className="fy-agent-resource-full-list"
          aria-busy={assignment.busy}
        >
          {filtered.map((server) => {
            const isAssigned = Boolean(server.apps[entry.assignmentId]);
            const isPending = assignment.pendingId === server.id;
            const transport = mcpTransport(server);
            return (
              <article
                key={server.id}
                className="fy-agent-assignment-card"
                data-pending={isPending || undefined}
              >
                <div className="fy-agent-assignment-card-copy">
                  <div className="fy-agent-assignment-card-title">
                    <h3>{server.name}</h3>
                    <span className="fy-agent-assignment-source-badge">
                      {transport}
                    </span>
                    {server.source ? (
                      <span className="fy-agent-assignment-source-badge">
                        {server.source}
                      </span>
                    ) : null}
                  </div>
                  <p>{server.description ?? "暂无说明"}</p>
                </div>
                <div className="fy-agent-assignment-card-action">
                  <Switch
                    checked={isAssigned}
                    disabled={assignment.busy}
                    label={`在 ${entry.displayName} 中使用 ${server.name}`}
                    onCheckedChange={(enabled) =>
                      void toggle(server.id, enabled)
                    }
                  />
                </div>
              </article>
            );
          })}
        </div>
      )}
      <WorkBuddyTrustDialog
        open={workbuddyTrustOpen}
        onOpenChange={setWorkbuddyTrustOpen}
      />
    </section>
  );
}
