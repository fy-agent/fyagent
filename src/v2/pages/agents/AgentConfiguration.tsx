import { useNavigate } from "react-router-dom";

import { getAgentBrand } from "../../shared/assets/agents";
import { CodexDesktopInstallerPanel } from "../../shared/codex-desktop/CodexDesktopInstallerPanel";
import type { ProductDirectoryEntry } from "../../shared/features/directory";
import { useFeatures } from "../../shared/features/provider";
import type { AgentCatalogEntry } from "../../shared/features/types";
import { FeatureTabs } from "../../shared/ui/FeatureTabs";
import { BrandIconFrame } from "../../shared/ui/catalog";
import { Button } from "../../shared/ui/primitives";

import { AgentInstallReadinessSection } from "./AgentInstallReadinessSection";
import { AgentMcpSection, AgentSkillsSection } from "./AgentAssignmentSections";
import { AgentModelsSection } from "./AgentModelsSection";
import { AgentPromptsSection } from "./AgentPromptsSection";
import type { AgentSection } from "./agentSections";

const sectionOptions: ReadonlyArray<{ id: AgentSection; label: string }> = [
  { id: "models", label: "模型" },
  { id: "skills", label: "Skills" },
  { id: "mcp", label: "MCP" },
  { id: "prompts", label: "提示词" },
];

export function AgentConfiguration({
  entry,
  catalogEntry,
  section,
  onSectionChange,
  onBack,
}: {
  entry: ProductDirectoryEntry;
  catalogEntry: AgentCatalogEntry;
  section: AgentSection;
  onSectionChange: (section: AgentSection) => void;
  onBack: () => void;
}) {
  const navigate = useNavigate();
  const { ports } = useFeatures();
  const openManagement = () => {
    switch (section) {
      case "models":
        navigate(`/models?target=${entry.modelTarget}`);
        break;
      case "skills":
        navigate("/skills");
        break;
      case "mcp":
        navigate("/mcp");
        break;
      case "prompts":
        navigate("/prompts");
        break;
    }
  };

  return (
    <section
      className="fy-agent-configuration"
      aria-label={`${entry.displayName} 配置`}
    >
      <header className="fy-agent-config-header">
        <div className="fy-agent-config-identity">
          <BrandIconFrame asset={getAgentBrand(entry.agentId)} size="list" />
          <div>
            <p>单 Agent 配置</p>
            <h1>{entry.displayName}</h1>
          </div>
        </div>
        <Button onClick={onBack}>返回软件目录</Button>
      </header>

      <FeatureTabs
        id="agent-configuration-sections"
        label={`${entry.displayName} 配置分段`}
        value={section}
        options={sectionOptions}
        onChange={onSectionChange}
        className="fy-agent-config-tabs"
      />

      <div className="fy-agent-config-body">
        {section === "models" ? (
          <AgentModelsSection
            entry={entry}
            catalogEntry={catalogEntry}
            onOpenManagement={openManagement}
          />
        ) : section === "skills" ? (
          <AgentSkillsSection entry={entry} onOpenManagement={openManagement} />
        ) : section === "mcp" ? (
          <AgentMcpSection entry={entry} onOpenManagement={openManagement} />
        ) : (
          <AgentPromptsSection
            entry={entry}
            onOpenManagement={openManagement}
          />
        )}
      </div>

      <details className="fy-agent-install-disclosure">
        <summary>安装、登录与启动能力</summary>
        <AgentInstallReadinessSection
          agentId={entry.agentId}
          port={ports.agentInstallReadiness}
        />
        {entry.agentId === "codex" ? <CodexDesktopInstallerPanel /> : null}
      </details>
    </section>
  );
}
