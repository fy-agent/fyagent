import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { InstallContractPanel } from "@/components/agent-install/InstallContractPanel";
import { useAgentInstallContract } from "@/hooks/useAgentInstallContract";

export function AgentCatalogPage() {
  const { t } = useTranslation();
  const {
    catalog,
    selectedId,
    setSelectedId,
    contract,
    reloadContract,
    openGuide,
    createPlan,
    startInstall,
  } = useAgentInstallContract();

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold">{t("agentInstall.title")}</h2>
        <p className="text-sm text-muted-foreground">{t("agentInstall.subtitle")}</p>
      </div>
      <div className="flex flex-wrap gap-2">
        {catalog.map((entry) => (
          <Button
            key={entry.agentId}
            variant={selectedId === entry.agentId ? "default" : "outline"}
            size="sm"
            onClick={() => setSelectedId(entry.agentId)}
          >
            {entry.legalEntity ?? entry.agentId}
          </Button>
        ))}
      </div>
      {contract ? (
        <InstallContractPanel
          contract={contract}
          onOpenGuide={() => {
            if (selectedId) void openGuide(selectedId);
          }}
          onRecheck={() => {
            if (selectedId) void reloadContract(selectedId);
          }}
          onRegenerate={() => {
            if (selectedId) void createPlan(selectedId);
          }}
          onInstall={() => {
            const snapshotId = contract.plan.planSnapshotId;
            if (snapshotId) void startInstall(snapshotId);
          }}
        />
      ) : null}
    </div>
  );
}
