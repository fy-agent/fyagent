import { useEffect, useState } from "react";
import { useLocation, useNavigate, useSearchParams } from "react-router-dom";

import { getAgentBrand } from "../../shared/assets/agents";
import { CodexDesktopInstallerPanel } from "../../shared/codex-desktop/CodexDesktopInstallerPanel";
import { PRODUCT_DIRECTORY } from "../../shared/features/directory";
import { convergeSelection } from "../../shared/features/helpers";
import { useAgentCatalog } from "../../shared/features/queries";
import type {
  AgentCapabilityId,
  AgentCatalogEntry,
  AgentCatalogId,
} from "../../shared/features/types";
import { AGENT_CATALOG_IDS } from "../../shared/features/types";
import {
  Button,
  EmptyState,
  InlineNotice,
  Spinner,
} from "../../shared/ui/primitives";
import {
  BrandIconFrame,
  CatalogDetail,
  CatalogList,
  CatalogListItem,
  CatalogMasterDetail,
  CatalogOfficialLinks,
  CatalogRail,
} from "../../shared/ui/catalog";

import { getAgentIntro } from "./intros";
import "./Page.css";

const MODEL_TARGET_BY_CATALOG_ID = Object.fromEntries(
  PRODUCT_DIRECTORY.map((entry) => [entry.agentId, entry.modelTarget]),
) as Readonly<
  Record<AgentCatalogId, (typeof PRODUCT_DIRECTORY)[number]["modelTarget"]>
>;

function capability(entry: AgentCatalogEntry, id: AgentCapabilityId) {
  return entry.capabilities.find((candidate) => candidate.id === id);
}

function AgentDetail({ entry }: { entry: AgentCatalogEntry }) {
  const navigate = useNavigate();
  const modelTarget = MODEL_TARGET_BY_CATALOG_ID[entry.id];
  const productCapability = capability(entry, "product.open");
  const modelWrite = capability(entry, "models.write");
  const skillsRead = capability(entry, "skills.read");
  const skillsWrite = capability(entry, "skills.write");
  const mcpWrite = capability(entry, "mcp.write");
  const showModelsJump = modelWrite?.mode === "direct";
  const showSkillsJump =
    skillsRead?.mode === "direct" || skillsWrite?.mode === "direct";
  const showMcpJump = mcpWrite?.mode === "direct";
  const showJumps = showModelsJump || showSkillsJump || showMcpJump;
  const intro = getAgentIntro(entry.id);

  return (
    <CatalogDetail
      className="fy-agent-detail"
      ariaLabel={`${entry.displayName} 详情`}
    >
      <div className="fy-agent-identity">
        <BrandIconFrame asset={getAgentBrand(entry.id)} size="detail" />
        <div className="fy-agent-identity-copy">
          <div className="fy-agent-identity-title">
            <h2>{entry.displayName}</h2>
          </div>
        </div>
        <CatalogOfficialLinks
          links={entry.officialLinks}
          disabled={
            productCapability?.mode !== "direct" &&
            productCapability?.mode !== "assisted"
          }
        />
      </div>

      {intro ? (
        <section className="fy-agent-section" aria-label="产品介绍">
          {intro.paragraphs.map((paragraph) => (
            <p key={paragraph} className="fy-feature-intro">
              {paragraph}
            </p>
          ))}
        </section>
      ) : null}

      {entry.id === "codex" && <CodexDesktopInstallerPanel />}

      {showJumps ? (
        <section className="fy-agent-section" aria-label="支持的功能">
          <h3>支持的功能</h3>
          <div className="fy-agent-action-row">
            {showModelsJump && (
              <Button
                className="fy-control-button-primary"
                onClick={() => navigate(`/models?target=${modelTarget}`)}
              >
                配置模型
              </Button>
            )}
            {showSkillsJump && (
              <Button onClick={() => navigate("/skills")}>打开 Skills</Button>
            )}
            {showMcpJump && (
              <Button onClick={() => navigate("/mcp")}>打开 MCP</Button>
            )}
          </div>
        </section>
      ) : null}
    </CatalogDetail>
  );
}

export function AgentsPage() {
  const { pathname } = useLocation();
  const pageActive = pathname === "/agents";
  const [searchParams, setSearchParams] = useSearchParams();
  const catalogQuery = useAgentCatalog();
  const [selectedId, setSelectedId] = useState<AgentCatalogId | null>(null);
  const entries = catalogQuery.data?.agents ?? [];
  const requestedTarget = pageActive ? searchParams.get("target") : null;
  const targetFromRoute =
    AGENT_CATALOG_IDS.find((id) => id === requestedTarget) ?? null;
  if (pageActive && targetFromRoute && targetFromRoute !== selectedId) {
    setSelectedId(targetFromRoute);
  }
  const convergedId = convergeSelection(entries, selectedId ?? targetFromRoute);
  const selected = entries.find((entry) => entry.id === convergedId) ?? null;

  useEffect(() => {
    if (!pageActive) return;
    if (searchParams.get("target") !== null) return;
    if (!selectedId) return;
    setSearchParams({ target: selectedId }, { replace: true });
  }, [pageActive, searchParams, selectedId, setSearchParams]);

  return (
    <div
      className="fy-feature-page fy-split-page fy-catalog-page fy-agents-page"
      data-testid="agents-page"
      aria-label="Agent 目录"
    >
      {catalogQuery.error && catalogQuery.data !== undefined && (
        <InlineNotice tone="warning">
          暂时无法刷新应用信息，正在显示已加载内容。
        </InlineNotice>
      )}

      {catalogQuery.isPending ? (
        <EmptyState title="正在加载 Agent 目录" description="正在获取应用信息">
          <Spinner label="正在加载 Agent 目录" />
        </EmptyState>
      ) : catalogQuery.isError && catalogQuery.data === undefined ? (
        <EmptyState
          title="无法加载 Agent 目录"
          description="暂时无法获取应用信息，请重试。"
          actions={
            <Button onClick={() => void catalogQuery.refetch()}>重试</Button>
          }
        />
      ) : entries.length === 0 || !selected ? (
        <EmptyState
          title="Agent 目录暂不可用"
          description="暂时没有可显示的应用，请重试。"
          actions={
            <Button onClick={() => void catalogQuery.refetch()}>重试</Button>
          }
        />
      ) : (
        <CatalogMasterDetail>
          <CatalogRail ariaLabel="Agent 选择" title="选择 Agent">
            <CatalogList>
              {entries.map((entry) => (
                <CatalogListItem
                  key={entry.id}
                  asset={getAgentBrand(entry.id)}
                  label={entry.displayName}
                  selected={entry.id === selected.id}
                  onSelect={() => {
                    setSelectedId(entry.id);
                    setSearchParams({ target: entry.id }, { replace: true });
                  }}
                />
              ))}
            </CatalogList>
          </CatalogRail>

          <AgentDetail entry={selected} />
        </CatalogMasterDetail>
      )}
    </div>
  );
}
