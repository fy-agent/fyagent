import { useNavigate } from "react-router-dom";

import { getAgentBrand } from "../../shared/assets/agents";
import { createAgentReturnLocationState } from "../../shared/features/agent-navigation";
import { useAgentReturnNavigation } from "../../shared/features/agent-navigation-context";
import type { ProductDirectoryEntry } from "../../shared/features/directory";
import type { AgentCatalogEntry } from "../../shared/features/types";
import { FeatureTabPanel, FeatureTabs } from "../../shared/ui/FeatureTabs";
import { BrandIconFrame } from "../../shared/ui/catalog";
import { Button } from "../../shared/ui/primitives";

import { AgentMcpSection, AgentSkillsSection } from "./AgentAssignmentSections";
import { AgentAuthStatusPanel } from "./AgentAuthStatusPanel";
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
  const agentReturnNavigation = useAgentReturnNavigation();
  const openManagement = () => {
    agentReturnNavigation.remember(entry.agentId, section);
    const state = createAgentReturnLocationState(entry.agentId, section);
    switch (section) {
      case "models":
        navigate(`/models?target=${entry.modelTarget}`, { state });
        break;
      case "skills":
        navigate("/skills", { state });
        break;
      case "mcp":
        navigate("/mcp", { state });
        break;
      case "prompts":
        navigate("/prompts", { state });
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
          <h1>{entry.displayName}</h1>
        </div>
        <Button onClick={onBack}>返回</Button>
      </header>

      <FeatureTabs
        id="agent-configuration-sections"
        label={`${entry.displayName} 配置分段`}
        value={section}
        options={sectionOptions}
        onChange={onSectionChange}
        className="fy-agent-config-tabs"
      />

      <FeatureTabPanel
        tabsId="agent-configuration-sections"
        value={section}
        active
        className="fy-agent-config-body"
      >
        <AgentAuthStatusPanel agentId={entry.agentId} />
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
      </FeatureTabPanel>
    </section>
  );
}
