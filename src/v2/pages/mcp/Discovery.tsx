import { useMemo, useState } from "react";

import {
  MCP_CATALOG,
  MCP_CATALOG_FILTERS,
  MCP_MATURITY_LABEL,
  MCP_PRIVILEGE_LABEL,
  MCP_PROVENANCE_LABEL,
  catalogInstallModeLabel,
  catalogRequiresConfig,
  catalogSearchText,
  catalogTransportLabel,
  matchesCatalogRecipe,
  type McpCatalogFilterId,
  type McpCatalogItem,
  type McpInstallValues,
} from "./catalog";
import { InstallDialog } from "./InstallDialog";
import { mcpInstallDestination } from "../../shared/features/helpers";
import { currentMcpLaunchPlatform } from "../../shared/features/mcpLaunch";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { InstallTargetDialog } from "../../shared/features/controls/InstallTargetDialog";
import {
  Badge,
  Button,
  ConfirmDialog,
  EmptyState,
} from "../../shared/ui/primitives";
import type { McpServer, McpTargetId } from "../../shared/features/types";

const REQUIREMENT_LABEL: Record<
  McpCatalogItem["requirements"][number],
  string
> = {
  none: "无需本地运行时",
  node: "需要 Node.js / npx",
  uv: "需要 uv / uvx",
};

function requirementText(item: McpCatalogItem): string {
  return item.requirements
    .map((requirement) => REQUIREMENT_LABEL[requirement])
    .join(" · ");
}

export function McpDiscovery({
  servers,
  busy,
  defaultTarget,
  onInstall,
  onPickTarget,
  onViewInstalled,
}: {
  servers: readonly McpServer[];
  busy: boolean;
  defaultTarget: McpTargetId;
  onInstall: (server: McpServer) => Promise<boolean>;
  onPickTarget: (target: McpTargetId) => void;
  onViewInstalled: (id: string) => void;
}) {
  const platform = currentMcpLaunchPlatform();
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<McpCatalogFilterId>("all");
  const [dialogItem, setDialogItem] = useState<McpCatalogItem | null>(null);
  const [overwrite, setOverwrite] = useState(false);
  const [pendingTarget, setPendingTarget] = useState<{
    item: McpCatalogItem;
    overwrite: boolean;
  } | null>(null);
  const [confirmItem, setConfirmItem] = useState<McpCatalogItem | null>(null);
  const [installingId, setInstallingId] = useState<string | null>(null);
  const installedById = useMemo(
    () => new Map(servers.map((server) => [server.id, server])),
    [servers],
  );

  const items = useMemo(() => {
    const query = search.trim().toLocaleLowerCase();
    return MCP_CATALOG.filter((item) => {
      if (category === "ready" && catalogRequiresConfig(item)) return false;
      if (category === "configure" && !catalogRequiresConfig(item)) {
        return false;
      }
      if (!query) return true;
      return catalogSearchText(item).includes(query);
    });
  }, [category, search]);

  const closeDialog = () => {
    setDialogItem(null);
    setOverwrite(false);
  };

  const installBuilt = async (server: McpServer, target: McpTargetId) => {
    setInstallingId(server.id);
    try {
      onPickTarget(target);
      const installed = await onInstall(server);
      if (installed) closeDialog();
    } finally {
      setInstallingId(null);
    }
  };

  const installWithValues = (
    item: McpCatalogItem,
    values: McpInstallValues,
    apps: readonly McpTargetId[],
    replaceExisting: boolean,
  ) => {
    if (!item.installable) {
      throw new Error(item.disabledReason ?? "暂未开放安装");
    }
    const existing = installedById.get(item.id);
    if (existing && !replaceExisting) {
      throw new Error("该 MCP 已存在");
    }
    const target = apps[0];
    if (!target) {
      throw new Error("请选择至少一个 Agent");
    }
    void installBuilt(item.build(values, [target], platform), target);
  };

  const startInstall = (item: McpCatalogItem, replaceExisting: boolean) => {
    if (!item.installable) return;
    const existing = installedById.get(item.id);
    if (existing && !replaceExisting) return;
    if (item.fields.length > 0) {
      setOverwrite(replaceExisting);
      setDialogItem(item);
      return;
    }
    setPendingTarget({ item, overwrite: replaceExisting });
  };

  return (
    <div className="fy-feature-discovery-scroll">
      <div className="fy-feature-toolbar">
        <FeatureSearch
          ariaLabel="搜索精选 MCP"
          placeholder="搜索名称、描述、标签或厂商"
          value={search}
          onValueChange={setSearch}
        />
        <select
          className="fy-control-select"
          aria-label="分类筛选"
          value={category}
          onChange={(event) =>
            setCategory(event.target.value as McpCatalogFilterId)
          }
        >
          {MCP_CATALOG_FILTERS.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.label}
            </option>
          ))}
        </select>
      </div>
      {items.length === 0 ? (
        <EmptyState
          title="没有匹配的精选 MCP"
          description="试试其他关键词或分类。凭据不会参与搜索。"
        />
      ) : (
        <div className="fy-feature-grid" aria-label="精选 MCP">
          {items.map((item) => {
            const existing = installedById.get(item.id);
            const sameRecipe =
              item.installable && existing
                ? matchesCatalogRecipe(item, existing, platform)
                : false;
            const pending = busy || installingId === item.id;
            const maturityLabel = MCP_MATURITY_LABEL[item.maturity];
            return (
              <article key={item.id} className="fy-feature-card">
                <header className="fy-mcp-card-meta">
                  <h3>{item.name}</h3>
                  <Badge tone="accent">{catalogInstallModeLabel(item)}</Badge>
                  <Badge>{MCP_PROVENANCE_LABEL[item.provenance]}</Badge>
                  <Badge>{catalogTransportLabel(item)}</Badge>
                  {item.privilege && (
                    <Badge
                      tone={item.privilege === "read" ? "neutral" : "warning"}
                    >
                      {MCP_PRIVILEGE_LABEL[item.privilege]}
                    </Badge>
                  )}
                  {maturityLabel && (
                    <Badge tone="warning">{maturityLabel}</Badge>
                  )}
                </header>
                <p>{item.description}</p>
                <p className="fy-mcp-card-note">
                  {requirementText(item)} · 认证：{item.authLabel}
                </p>
                {item.risk && <p className="fy-mcp-card-note">{item.risk}</p>}
                {item.disabledReason && (
                  <p className="fy-mcp-card-note">{item.disabledReason}</p>
                )}
                <footer>
                  {!item.installable ? (
                    <>
                      <Button disabled>暂未开放安装</Button>
                      {existing && (
                        <Button onClick={() => onViewInstalled(item.id)}>
                          查看
                        </Button>
                      )}
                    </>
                  ) : existing && sameRecipe ? (
                    <>
                      <Button disabled>已安装</Button>
                      <Button onClick={() => onViewInstalled(item.id)}>
                        查看
                      </Button>
                    </>
                  ) : existing ? (
                    <>
                      <Button disabled>已存在</Button>
                      <Button onClick={() => onViewInstalled(item.id)}>
                        查看
                      </Button>
                      <Button
                        className="fy-control-button-primary"
                        disabled={pending}
                        onClick={() => setConfirmItem(item)}
                      >
                        重新配置
                      </Button>
                    </>
                  ) : (
                    <Button
                      className="fy-control-button-primary"
                      disabled={pending}
                      onClick={() => startInstall(item, false)}
                    >
                      {pending
                        ? "安装中…"
                        : item.fields.length > 0
                          ? "配置并安装"
                          : "安装"}
                    </Button>
                  )}
                  {item.docs && (
                    <ExternalLinkButton url={item.docs}>
                      文档
                    </ExternalLinkButton>
                  )}
                  {!item.docs && item.homepage && (
                    <ExternalLinkButton url={item.homepage}>
                      主页
                    </ExternalLinkButton>
                  )}
                </footer>
              </article>
            );
          })}
        </div>
      )}
      {dialogItem && (
        <InstallDialog
          key={`${dialogItem.id}:${overwrite ? "overwrite" : "new"}`}
          item={dialogItem}
          busy={busy || installingId === dialogItem.id}
          overwrite={overwrite}
          defaultTarget={defaultTarget}
          onClose={closeDialog}
          onInstall={(values, apps) =>
            installWithValues(dialogItem, values, apps, overwrite)
          }
        />
      )}
      {pendingTarget && (
        <InstallTargetDialog
          key={`${pendingTarget.item.id}:${pendingTarget.overwrite ? "overwrite" : "new"}`}
          title={
            pendingTarget.overwrite
              ? `重新配置 ${pendingTarget.item.name}`
              : `安装 ${pendingTarget.item.name}`
          }
          busy={busy || installingId === pendingTarget.item.id}
          defaultTarget={defaultTarget}
          confirmVerb={pendingTarget.overwrite ? "确认覆盖安装" : "确认安装"}
          pathForTarget={(target) => mcpInstallDestination(target, platform)}
          onCancel={() => setPendingTarget(null)}
          onConfirm={(target) => {
            const { item } = pendingTarget;
            setPendingTarget(null);
            void installBuilt(item.build({}, [target], platform), target);
          }}
        />
      )}
      <ConfirmDialog
        open={confirmItem !== null}
        title={`重新配置 ${confirmItem?.name ?? "MCP"}`}
        description="将覆盖现有配置。已填写的密钥以外的手动修改不会保留。"
        pending={busy}
        onCancel={() => setConfirmItem(null)}
        onConfirm={() => {
          const item = confirmItem;
          setConfirmItem(null);
          if (item) startInstall(item, true);
        }}
      />
    </div>
  );
}
